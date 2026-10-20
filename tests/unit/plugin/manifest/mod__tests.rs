use super::*;

fn valid() -> Manifest {
    Manifest {
        manifest_version: 1,
        plugin_id: "org.example.patch".into(),
        plugin_version: Version::parse("1.2.3").unwrap(),
        label: "Example".into(),
        description: "Example plugin".into(),
        is_patcher: true,
        game_launcher: Some("./runner.exe".into()),
        executable_watch: Some("game.exe".into()),
        executable_watch_type: Some(WatchType::FindInDirectory),
        runtime: Some(Runtime {
            executable: "./runner.exe".into(),
            operations: vec![Operation::Prepare, Operation::Launch, Operation::Restore],
        }),
        config_files: vec![ConfigFile {
            config_file_location: "./config/default.ini".into(),
            config_file_copy_type: CopyType::Absolute,
            config_file_replace_path: Some("./config.ini".into()),
            config_add_to_config_menu: true,
        }],
        plugin_files: vec![PluginFile {
            plugin_file_location: "./runner.exe".into(),
            plugin_file_copy_type: CopyType::GameFolder,
            plugin_file_replace_path: None,
        }],
        install_scripts: vec![InstallScript {
            install_script_location: "./install.ps1".into(),
            install_script_arguments: vec!["-Mode".into(), "quiet".into()],
        }],
    }
}

#[test]
fn canonical_round_trip_is_stable_and_typed() {
    let first = valid().canonical_toml().unwrap();
    let parsed = Manifest::parse(&first).unwrap();
    assert_eq!(parsed, valid());
    assert_eq!(parsed.canonical_toml().unwrap(), first);
    assert!(first.contains("[[configFiles]]"));
    assert!(first.contains("[[pluginFiles]]"));
    assert!(first.contains("[[installScripts]]"));
}

#[test]
fn rejects_unknown_and_numbered_keys_with_guidance() {
    let raw = valid().canonical_toml().unwrap();
    let unknown = format!("{raw}\nunknownField = true\n");
    assert!(Manifest::parse(&unknown).unwrap_err().contains("unknown"));
    let numbered = raw.replace(
        "[[configFiles]]",
        "configFileLocation1 = \"./config/default.ini\"",
    );
    assert!(Manifest::parse(&numbered)
        .unwrap_err()
        .contains("[[configFiles]]"));
}

#[test]
fn validates_mode_specific_paths_and_patcher_contract() {
    let mut manifest = valid();
    manifest.config_files[0].config_file_replace_path = None;
    assert!(manifest.validate().unwrap_err().contains("required"));
    manifest.config_files[0].config_file_replace_path = Some("configs\\".into());
    assert!(manifest.validate().is_ok());
    manifest.config_files[0].config_file_replace_path = Some("configs\\\\".into());
    assert!(manifest.validate().unwrap_err().contains("empty path"));
    let mut manifest = valid();
    manifest.runtime.as_mut().unwrap().operations = vec![Operation::Prepare];
    assert!(manifest.validate().unwrap_err().contains("restore"));
}

#[test]
fn the_runtime_must_be_an_exe_program() {
    for script in ["./runner.cmd", "./runner.bat", "./runner.ps1"] {
        let mut manifest = valid();
        manifest.runtime.as_mut().unwrap().executable = script.into();
        manifest.plugin_files[0].plugin_file_location = script.into();
        manifest.game_launcher = Some(script.into());
        assert_eq!(
            manifest.validate().unwrap_err(),
            "runtime.executable must name an .exe program",
            "{script}"
        );
    }
    let mut manifest = valid();
    manifest.runtime.as_mut().unwrap().executable = "./RUNNER.EXE".into();
    assert!(manifest.validate().is_ok());
}

#[test]
fn package_paths_reject_windows_escape_forms() {
    for bad in [
        r"C:\outside.dll",
        r"\\server\share\a",
        r"\\?\C:\a",
        "../a",
        "a/../../b",
        "a:stream",
        "NUL.txt",
        "dir/COM1",
        "dir//file",
    ] {
        assert!(normalize_package_path(bad).is_err(), "{bad}");
    }
    assert_eq!(
        normalize_package_path(r".\payload\file.dll").unwrap(),
        "payload/file.dll"
    );
}

#[test]
fn defaults_are_false_and_optional() {
    let raw = r#"
manifestVersion = 1
pluginId = "org.example.config"
pluginVersion = "1.0.0"
label = "Config"
description = "One file"

[[configFiles]]
configFileLocation = "./config.ini"
configFileCopyType = "game_folder"
"#;
    let parsed = Manifest::parse(raw).unwrap();
    assert!(!parsed.is_patcher);
    assert!(!parsed.config_files[0].config_add_to_config_menu);
    assert_eq!(parsed.executable_watch_type, None);
}
