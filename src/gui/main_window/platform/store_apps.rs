use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use rust_i18n::t;
use windows::core::{HSTRING, PCWSTR, PWSTR};
use windows::Win32::Storage::Packaging::Appx::GetPackagesByPackageFamily;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    ApplicationActivationManager, IApplicationActivationManager, AO_NONE,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const STORE_PREFIX: &str = "shell:AppsFolder\\";

const ACTIVATE_ATTEMPTS: u32 = 3;
const ACTIVATE_BACKOFF_MS: [u64; 2] = [120, 250];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreApp {
    pub name: String,

    pub aumid: String,
}

pub fn launch_path(aumid: &str) -> String {
    format!("{STORE_PREFIX}{aumid}")
}

pub fn is_store_target(path: &Path) -> bool {
    path.to_string_lossy().starts_with(STORE_PREFIX)
}

fn aumid_from_target(shell_path: &Path) -> Option<String> {
    shell_path
        .to_string_lossy()
        .strip_prefix(STORE_PREFIX)
        .map(str::to_string)
}

fn packaged_family_name(aumid: &str) -> Option<&str> {
    aumid
        .split_once('!')
        .map(|(family, _)| family)
        .filter(|family| !family.is_empty())
}

enum ActivateOutcome {
    Launched,

    NotRegistered,

    Failed,
}

pub fn launch(shell_path: &Path) -> Result<(), String> {
    if let Some(aumid) = aumid_from_target(shell_path) {
        if let Some(family) = packaged_family_name(&aumid) {
            match activate_store_app(&aumid, family) {
                ActivateOutcome::Launched => return Ok(()),
                ActivateOutcome::NotRegistered => {
                    return Err(t!("gs.store_app_installing").to_string())
                }
                ActivateOutcome::Failed => {}
            }
        }
    }
    launch_via_explorer(shell_path)
}

fn launch_via_explorer(shell_path: &Path) -> Result<(), String> {
    Command::new("explorer.exe")
        .raw_arg(format!("\"{}\"", shell_path.display()))
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            format!(
                "failed to launch Store app {}: {error}",
                shell_path.display()
            )
        })
}

fn activate_store_app(aumid: &str, family_name: &str) -> ActivateOutcome {
    let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let should_uninitialize = com.is_ok();
    let outcome = activate_inner(aumid, family_name);
    if should_uninitialize {
        unsafe { CoUninitialize() };
    }
    outcome
}

fn activate_inner(aumid: &str, family_name: &str) -> ActivateOutcome {
    let manager: IApplicationActivationManager =
        match unsafe { CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_ALL) } {
            Ok(manager) => manager,

            Err(_) => return ActivateOutcome::Failed,
        };

    let aumid_wide = HSTRING::from(aumid);
    for attempt in 0..ACTIVATE_ATTEMPTS {
        if !package_family_registered(family_name) {
            backoff(attempt);
            continue;
        }

        let activated = unsafe {
            manager.ActivateApplication(PCWSTR(aumid_wide.as_ptr()), PCWSTR::null(), AO_NONE)
        };
        match activated {
            Ok(_process_id) => return ActivateOutcome::Launched,
            Err(_) => backoff(attempt),
        }
    }

    if package_family_registered(family_name) {
        ActivateOutcome::Failed
    } else {
        ActivateOutcome::NotRegistered
    }
}

fn backoff(attempt: u32) {
    if let Some(ms) = ACTIVATE_BACKOFF_MS.get(attempt as usize) {
        std::thread::sleep(std::time::Duration::from_millis(*ms));
    }
}

fn package_family_registered(family_name: &str) -> bool {
    let family = HSTRING::from(family_name);
    let mut count: u32 = 0;
    let mut buffer_length: u32 = 0;

    let _ = unsafe {
        GetPackagesByPackageFamily(
            PCWSTR(family.as_ptr()),
            &mut count,
            None,
            &mut buffer_length,
            PWSTR::null(),
        )
    };
    count > 0
}

pub fn list() -> Result<Vec<StoreApp>, String> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; \
             $apps=(New-Object -ComObject Shell.Application).NameSpace('shell:AppsFolder'); \
             $apps.Items() | ForEach-Object { \
               $_.Name; \
               $id=$_.ExtendedProperty('System.AppUserModel.ID'); \
               if ([string]::IsNullOrEmpty($id)) { $id = $_.Path }; \
               $id }",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("failed to enumerate Store apps: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Store app enumeration failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text.lines().collect();
    let mut apps: Vec<StoreApp> = lines
        .chunks(2)
        .filter_map(|pair| {
            let name = pair.first()?.trim();
            let aumid = pair.get(1)?.trim();
            (!name.is_empty() && !aumid.is_empty()).then(|| StoreApp {
                name: name.to_string(),
                aumid: aumid.to_string(),
            })
        })
        .collect();
    apps.sort_by_key(|app| app.name.to_lowercase());
    apps.dedup_by(|a, b| a.aumid == b.aumid);
    Ok(apps)
}
