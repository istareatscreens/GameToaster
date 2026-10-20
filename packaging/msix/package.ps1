[CmdletBinding()]
param(
    [switch] $Describe,
    [string] $CargoManifest,
    [string] $IdentityFile,
    [string] $Executable,
    [string] $LogoSource,
    [string] $StagingDirectory,
    [string] $OutputDirectory,
    [string] $MakeAppx
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path

function Resolve-Option([string] $value, [string] $default) {
    if (-not $value) {
        $value = Join-Path $repositoryRoot $default
    }
    return $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($value)
}

function Read-StoreIdentity([string] $path) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Store identity file not found: $path"
    }
    $identity = Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json
    $values = @{}
    $problems = New-Object System.Collections.Generic.List[string]
    foreach ($field in 'identityName', 'publisher', 'publisherDisplayName', 'displayName', 'storeId') {
        $property = $identity.PSObject.Properties[$field]
        $value = ''
        if ($property -and $null -ne $property.Value) {
            $value = ([string] $property.Value).Trim()
        }
        if (-not $value) {
            $problems.Add("$field is empty")
        }
        $values[$field] = $value
    }
    if ($values['identityName'] -and $values['identityName'] -notmatch '^[A-Za-z0-9.-]{3,50}$') {
        $problems.Add("identityName '$($values['identityName'])' must be 3 to 50 letters, digits, dots or hyphens")
    }
    if ($values['publisher'] -and -not $values['publisher'].StartsWith('CN=')) {
        $problems.Add("publisher '$($values['publisher'])' must start with CN=")
    }
    if ($problems.Count -gt 0) {
        throw ("$path is incomplete:`n  " + ($problems -join "`n  ") +
            "`nCopy the values from Partner Center > Product management > Product identity.")
    }
    return $values
}

function Read-CargoVersion([string] $path) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Cargo manifest not found: $path"
    }
    $section = ''
    foreach ($line in Get-Content -LiteralPath $path -Encoding UTF8) {
        $trimmed = $line.Trim()
        if ($trimmed -match '^\[([^\]]+)\]$') {
            $section = $Matches[1].Trim()
            continue
        }
        if ($section -eq 'package' -and $trimmed -match '^version\s*=\s*"([^"]*)"$') {
            return $Matches[1]
        }
    }
    throw "$path has no [package] version"
}

function ConvertTo-MsixVersion([string] $cargoVersion) {
    if ($cargoVersion -notmatch '^(\d{1,9})\.(\d{1,9})\.(\d{1,9})$') {
        throw "Cargo version '$cargoVersion' must be <major>.<minor>.<patch> with numbers only"
    }
    $parts = @([int] $Matches[1], [int] $Matches[2], [int] $Matches[3])
    foreach ($part in $parts) {
        if ($part -gt 65535) {
            throw "Cargo version '$cargoVersion' has a part above 65535"
        }
    }
    if ($parts[0] -lt 1) {
        throw "Cargo version '$cargoVersion' cannot be packaged: Store package versions cannot start with 0; use 1.0.0 or higher"
    }
    return ('{0}.{1}.{2}.0' -f $parts[0], $parts[1], $parts[2])
}

function Assert-X64Executable([string] $path) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Release executable not found: $path. Run build.bat first."
    }
    $stream = [System.IO.File]::OpenRead($path)
    try {
        $reader = New-Object System.IO.BinaryReader($stream)
        if ($reader.ReadUInt16() -ne 0x5A4D) {
            throw "$path is not a Windows executable (no MZ header)"
        }
        $stream.Position = 0x3C
        $stream.Position = $reader.ReadInt32()
        if ($reader.ReadUInt32() -ne 0x00004550) {
            throw "$path has no PE signature"
        }
        $machine = $reader.ReadUInt16()
    }
    finally {
        $stream.Dispose()
    }
    if ($machine -ne 0x8664) {
        throw ('{0} is built for machine type 0x{1:X4}; the package is x64 (0x8664)' -f $path, $machine)
    }
}

function New-Logo([System.Drawing.Image] $source, [int] $size, [string] $path) {
    $bitmap = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
            $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
            $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
            $attributes = New-Object System.Drawing.Imaging.ImageAttributes
            try {
                $attributes.SetWrapMode([System.Drawing.Drawing2D.WrapMode]::TileFlipXY)
                $target = New-Object System.Drawing.Rectangle(0, 0, $size, $size)
                $graphics.DrawImage($source, $target, 0, 0, $source.Width, $source.Height,
                    [System.Drawing.GraphicsUnit]::Pixel, $attributes)
            }
            finally {
                $attributes.Dispose()
            }
        }
        finally {
            $graphics.Dispose()
        }
        $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $bitmap.Dispose()
    }
}

function Write-Logos([string] $logoSource, [string] $assets) {
    Add-Type -AssemblyName System.Drawing
    if (-not (Test-Path -LiteralPath $logoSource -PathType Leaf)) {
        throw "Logo source not found: $logoSource"
    }
    $source = [System.Drawing.Image]::FromFile($logoSource)
    try {
        if ($source.Width -ne $source.Height) {
            throw "$logoSource must be square to make tile logos; it is $($source.Width)x$($source.Height)"
        }
        New-Logo $source 44 (Join-Path $assets 'Square44x44Logo.png')
        New-Logo $source 150 (Join-Path $assets 'Square150x150Logo.png')
        New-Logo $source 50 (Join-Path $assets 'StoreLogo.png')
    }
    finally {
        $source.Dispose()
    }
}

function Write-Manifest([string] $template, [string] $destination, [hashtable] $tokens) {
    $text = [System.IO.File]::ReadAllText($template, [System.Text.Encoding]::UTF8)
    foreach ($name in $tokens.Keys) {
        $value = [System.Security.SecurityElement]::Escape([string] $tokens[$name])
        $text = $text.Replace('{{' + $name + '}}', $value)
    }
    if ($text.Contains('{{')) {
        throw "$template has a placeholder that this script does not fill"
    }
    [System.IO.File]::WriteAllText($destination, $text, (New-Object System.Text.UTF8Encoding($false)))
}

function Find-MakeAppx([string] $explicit) {
    if ($explicit) {
        if (-not (Test-Path -LiteralPath $explicit -PathType Leaf)) {
            throw "makeappx.exe not found: $explicit"
        }
        return $explicit
    }
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $candidates = @(
        Get-ChildItem -LiteralPath $kits -Directory -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Name -match '^\d+(\.\d+){3}$' -and
                (Test-Path -LiteralPath (Join-Path $_.FullName 'x64\makeappx.exe') -PathType Leaf)
            } |
            Sort-Object -Property { [version] $_.Name } -Descending
    )
    if ($candidates.Count -eq 0) {
        throw "makeappx.exe was not found under $kits; install the Windows SDK or pass -MakeAppx"
    }
    return (Join-Path $candidates[0].FullName 'x64\makeappx.exe')
}

try {
    $CargoManifest = Resolve-Option $CargoManifest 'Cargo.toml'
    $IdentityFile = Resolve-Option $IdentityFile 'packaging\msix\store-identity.json'
    $Executable = Resolve-Option $Executable 'build_output\GameToaster.exe'
    $LogoSource = Resolve-Option $LogoSource 'assets\toaster.png'
    $StagingDirectory = Resolve-Option $StagingDirectory 'build\msix'
    $OutputDirectory = Resolve-Option $OutputDirectory 'build_output'

    $identity = Read-StoreIdentity $IdentityFile
    $cargoVersion = Read-CargoVersion $CargoManifest
    $msixVersion = ConvertTo-MsixVersion $cargoVersion
    $packageName = "GameToaster_${msixVersion}_x64.msix"
    $description = [pscustomobject][ordered]@{
        cargoVersion = $cargoVersion
        msixVersion  = $msixVersion
        package      = $packageName
        storeId      = $identity['storeId']
    }
    $descriptionJson = $description | ConvertTo-Json -Compress
    if ($Describe) {
        Write-Output $descriptionJson
        exit 0
    }

    Assert-X64Executable $Executable
    $makeAppxPath = Find-MakeAppx $MakeAppx

    $layout = Join-Path $StagingDirectory 'layout'
    if (Test-Path -LiteralPath $layout) {
        Remove-Item -LiteralPath $layout -Recurse -Force
    }
    $assets = Join-Path $layout 'Assets'
    New-Item -ItemType Directory -Path $assets -Force | Out-Null
    Copy-Item -LiteralPath $Executable -Destination (Join-Path $layout 'GameToaster.exe')
    Write-Logos $LogoSource $assets
    Write-Manifest (Join-Path $PSScriptRoot 'AppxManifest.xml') (Join-Path $layout 'AppxManifest.xml') @{
        IdentityName         = $identity['identityName']
        Publisher            = $identity['publisher']
        PublisherDisplayName = $identity['publisherDisplayName']
        DisplayName          = $identity['displayName']
        Version              = $msixVersion
    }

    New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
    $package = Join-Path $OutputDirectory $packageName
    & $makeAppxPath pack /o /h SHA256 /d $layout /p $package
    if ($LASTEXITCODE -ne 0) {
        throw "makeappx.exe failed with exit code $LASTEXITCODE"
    }
    [System.IO.File]::WriteAllText((Join-Path $StagingDirectory 'package.json'), $descriptionJson,
        (New-Object System.Text.UTF8Encoding($false)))
    Write-Output $package
}
catch {
    [Console]::Error.WriteLine('package.ps1: ' + $_.Exception.Message)
    exit 1
}
