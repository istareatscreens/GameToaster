use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::plugin::catalog::inventory::sha256_file;
use crate::plugin::deployment::{
    AssignmentLedger, InstallScriptReceipt, InstallScriptRunState, LedgerStore,
};
use crate::plugin::manifest::{
    normalize_package_path, validate_package_file, InstallScript, Manifest,
};
use crate::support::process::interpreter::script_command;

const SCRIPT_TIMEOUT: Duration = Duration::from_secs(300);
const OUTPUT_LIMIT: u64 = 16 * 1024;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn run_pending(
    store: &LedgerStore,
    revision_root: &Path,
    manifest: &Manifest,
    ledger: &mut AssignmentLedger,
) -> Result<(), String> {
    for (index, script) in manifest.install_scripts.iter().enumerate() {
        run_one(store, revision_root, ledger, index as u32, script)?;
    }
    Ok(())
}

pub fn pending_count(manifest: &Manifest, ledger: &AssignmentLedger) -> usize {
    manifest
        .install_scripts
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            !ledger.install_script_receipts.iter().any(|receipt| {
                receipt.plugin_version == ledger.plugin_version
                    && receipt
                        .revision_sha256
                        .eq_ignore_ascii_case(&ledger.revision_sha256)
                    && receipt.script_index == *index as u32
                    && receipt.state == InstallScriptRunState::Succeeded
            })
        })
        .count()
}

fn run_one(
    store: &LedgerStore,
    revision_root: &Path,
    ledger: &mut AssignmentLedger,
    script_index: u32,
    script: &InstallScript,
) -> Result<(), String> {
    let source_relative = normalize_package_path(&script.install_script_location)?;
    let source = validate_package_file(revision_root, &source_relative)?;
    let source_sha256 = sha256_file(&source)?;
    let position = ledger.install_script_receipts.iter().position(|receipt| {
        receipt.plugin_version == ledger.plugin_version
            && receipt
                .revision_sha256
                .eq_ignore_ascii_case(&ledger.revision_sha256)
            && receipt.script_index == script_index
    });
    if let Some(position) = position {
        let receipt = &ledger.install_script_receipts[position];
        if !receipt.source_sha256.eq_ignore_ascii_case(&source_sha256)
            || !receipt
                .source_relative
                .eq_ignore_ascii_case(&source_relative)
        {
            return Err(format!(
                "install script {} changed inside immutable plugin {} {}",
                source_relative, ledger.plugin_id, ledger.plugin_version
            ));
        }
        if receipt.state == InstallScriptRunState::Succeeded {
            return Ok(());
        }
        if receipt.state == InstallScriptRunState::Started {
            return Err(format!(
                "install script {} has an incomplete prior run and will not be started again automatically",
                source_relative
            ));
        }
    }

    let attempts = position
        .map(|position| {
            ledger.install_script_receipts[position]
                .attempts
                .saturating_add(1)
        })
        .unwrap_or(1);
    let receipt = InstallScriptReceipt {
        plugin_version: ledger.plugin_version.clone(),
        revision_sha256: ledger.revision_sha256.clone(),
        script_index,
        source_relative: source_relative.clone(),
        source_sha256,
        attempts,
        state: InstallScriptRunState::Started,
        exit_code: None,
        detail: None,
    };
    let receipt_index = if let Some(position) = position {
        ledger.install_script_receipts[position] = receipt;
        position
    } else {
        ledger.install_script_receipts.push(receipt);
        ledger.install_script_receipts.len() - 1
    };
    store.save(ledger)?;

    let outcome = execute(store, ledger, &source, &script.install_script_arguments);
    let receipt = &mut ledger.install_script_receipts[receipt_index];
    match outcome {
        Ok(output) if output.code == 0 => {
            receipt.state = InstallScriptRunState::Succeeded;
            receipt.exit_code = Some(0);
            receipt.detail = output.detail;
            store.save(ledger)
        }
        Ok(output) => {
            receipt.state = InstallScriptRunState::Failed;
            receipt.exit_code = Some(output.code);
            receipt.detail = output.detail.clone();
            store.save(ledger)?;
            Err(format!(
                "install script {} exited with code {}{}",
                source_relative,
                output.code,
                output
                    .detail
                    .filter(|value| !value.is_empty())
                    .map(|value| format!(": {value}"))
                    .unwrap_or_default()
            ))
        }
        Err(error) => {
            receipt.state = InstallScriptRunState::Failed;
            receipt.exit_code = None;
            receipt.detail = Some(error.clone());
            store.save(ledger)?;
            Err(format!("install script {source_relative} failed: {error}"))
        }
    }
}

struct ScriptOutput {
    code: i32,
    detail: Option<String>,
}

fn execute(
    store: &LedgerStore,
    ledger: &AssignmentLedger,
    script: &Path,
    arguments: &[String],
) -> Result<ScriptOutput, String> {
    let output_root = store
        .assignment_root(ledger.game_id, &ledger.assignment_id)?
        .join("script-output");
    fs::create_dir_all(&output_root)
        .map_err(|error| format!("failed to create {}: {error}", output_root.display()))?;
    let token = crate::support::fs::random_hex(16)?;
    let stdout_path = output_root.join(format!("{token}.stdout"));
    let stderr_path = output_root.join(format!("{token}.stderr"));
    let stdout = create_output(&stdout_path)?;
    let stderr = match create_output(&stderr_path) {
        Ok(file) => file,
        Err(error) => {
            drop(stdout);
            let _ = fs::remove_file(&stdout_path);
            return Err(error);
        }
    };

    let mut command = command_for(script, arguments)?;
    command
        .current_dir(&ledger.game_folder)
        .stdin(Stdio::null())
        .env("GAMETOASTER_PLUGIN_ID", &ledger.plugin_id)
        .env("GAMETOASTER_PLUGIN_VERSION", &ledger.plugin_version)
        .env("GAMETOASTER_GAME_FOLDER", &ledger.game_folder)
        .env("GAMETOASTER_SELECTED_TARGET", &ledger.selected_target)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .creation_flags(CREATE_NO_WINDOW);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            cleanup_output(&stdout_path, &stderr_path, &output_root);
            return Err(format!("failed to start {}: {error}", script.display()));
        }
    };
    let deadline = Instant::now() + SCRIPT_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let detail = read_detail(&stdout_path, &stderr_path);
                cleanup_output(&stdout_path, &stderr_path, &output_root);
                return Err(format!(
                    "timed out after {} seconds{}",
                    SCRIPT_TIMEOUT.as_secs(),
                    detail.map(|value| format!(": {value}")).unwrap_or_default()
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                cleanup_output(&stdout_path, &stderr_path, &output_root);
                return Err(format!("failed to monitor {}: {error}", script.display()));
            }
        }
    };
    let detail = read_detail(&stdout_path, &stderr_path);
    cleanup_output(&stdout_path, &stderr_path, &output_root);
    Ok(ScriptOutput {
        code: status.code().unwrap_or(-1),
        detail,
    })
}

fn command_for(script: &Path, arguments: &[String]) -> Result<Command, String> {
    let extension = script
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "exe" => {
            let mut command = Command::new(script);
            command.args(arguments.iter().map(OsString::from));
            Ok(command)
        }
        "ps1" => {
            let mut command = Command::new(crate::support::system_folders::windows_powershell()?);
            command.args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ]);
            command.arg(script).args(arguments);
            Ok(command)
        }
        "bat" | "cmd" => {
            let arguments: Vec<OsString> = arguments.iter().map(OsString::from).collect();
            script_command(script, &arguments)
        }
        _ => Err(format!(
            "unsupported install script type: {}",
            script.display()
        )),
    }
}

fn create_output(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))
}

fn read_detail(stdout: &Path, stderr: &Path) -> Option<String> {
    let stderr = read_tail(stderr).unwrap_or_default();
    let stdout = read_tail(stdout).unwrap_or_default();
    match (stderr.is_empty(), stdout.is_empty()) {
        (false, false) => Some(format!("{stderr}\n{stdout}")),
        (false, true) => Some(stderr),
        (true, false) => Some(stdout),
        (true, true) => None,
    }
}

fn read_tail(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let length = file.metadata().map_err(|error| error.to_string())?.len();
    if length > OUTPUT_LIMIT {
        file.seek(SeekFrom::End(-(OUTPUT_LIMIT as i64)))
            .map_err(|error| error.to_string())?;
    }
    let mut bytes = Vec::with_capacity(length.min(OUTPUT_LIMIT) as usize);
    file.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let value = String::from_utf8_lossy(&bytes).replace('\0', "");
    Ok(value.trim().to_string())
}

fn cleanup_output(stdout: &Path, stderr: &Path, root: &Path) {
    let _ = fs::remove_file(stdout);
    let _ = fs::remove_file(stderr);
    let _ = fs::remove_dir(root);
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/lifecycle/install_scripts__tests.rs"]
mod tests;
