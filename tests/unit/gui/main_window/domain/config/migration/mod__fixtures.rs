const PREPARED: &str = "{prepared kind}";

pub(super) fn prepared_kind() -> String {
    String::from_utf8(vec![0x70, 0x72, 0x65, 0x2D, 0x64, 0x78]).expect("ASCII")
}

pub(super) fn as_lf(text: &str) -> String {
    text.replace(PREPARED, &prepared_kind())
}

pub(super) fn as_written(text: &str) -> String {
    as_lf(text).replace('\n', "\r\n")
}

pub(super) const V1_FULL: &str = r#"[game.7]
label=Second Prepared
exe_path=D:\Games\Second\second.exe
kind={prepared kind}
close_mode=kill
[controller_screen]
background_image=C:\Art #2\background.png
reopen_after_game_close=true
jvs_player=p2
calibration_mode_height=1080
pid=0x5750
keep_controller_alive_when_minimized=true
monitor_adapter=2
ba2_enabled=true
vid=0x2E3C
controller_adapter=adx-hid
ba1_enabled=false
calibration_monitor_id=\\?\DISPLAY#ABC1234#5&1a2b3c4d&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}
calibration_rotation=90
orientation=production-landscape
calibration_mode_width=1920
launch_on_start=true
calibration_diameter_px=1000
calibration_profile=cabinet-portrait
enabled=true
jvs_endpoint=COM4
jvs_hardware_validated=true
controller_only_start=true
open_on_os_startup=true
launcher_x_action=exit-launcher
[game.3]
label=日本語 ゲーム
hotkey=Ctrl+Shift+F5
watch_exe=example.exe
config_target1=C:\遊戲\settings.ini
kind={prepared kind}
close_mode=script
exe_path=C:\遊戲\example.exe
config_target2=C:\遊戲\tool.exe
close_hotkey=Ctrl+Shift+F6
controller_image=launcher_assets/game_images/3.png
close_script=C:\遊戲\close.bat
[game.5]
kind=other
exe_path=D:\Games\Script\start.bat
close_mode=kill
label=Script Game
[launcher]
default_game=3
close_game_hotkey=Ctrl+Alt+K
launcher_enabled=true
launch_default_on_open=true
block_exit_on_credit=true
language=ja
show_command_windows=true
launcher_hotkey=Ctrl+Alt+J
auto_hide_on_launch=false
pause_on_failure=true
next_game_id=8
adapter=1
diagnostics_logging=true
"#;

pub(super) const V1_JVS: &str = r#"[launcher]
launcher_hotkey=
close_game_hotkey=
auto_hide_on_launch=true
adapter
next_game_id=2
show_command_windows=false
default_game
launch_default_on_open=false
diagnostics_logging=false
launcher_enabled=true
block_exit_on_credit=false
pause_on_failure=false
language=
[controller_screen]
jvs_hardware_validated=false
keep_controller_alive_when_minimized=false
controller_adapter=jvs
launch_on_start=false
launcher_x_action=minimize-to-tray
orientation=auto
vid=0x0000
open_on_os_startup=false
jvs_endpoint=COM4
ba2_enabled=true
background_image
controller_only_start=false
jvs_player=p2
ba1_enabled=true
enabled=true
reopen_after_game_close=true
pid=0x0000
monitor_adapter
[game.1]
exe_path=C:\Games\Only\only.exe
label=Only Game
close_mode=kill
kind=other
"#;

pub(super) const V1_DEFAULT: &str = r#"[launcher]
auto_hide_on_launch=true
close_game_hotkey=
launch_default_on_open=false
adapter
default_game
language=
next_game_id=1
show_command_windows=false
launcher_enabled=true
diagnostics_logging=false
block_exit_on_credit=false
pause_on_failure=false
launcher_hotkey=Ctrl+Alt+M
[controller_screen]
background_image
ba2_enabled=true
vid=0x0CA3
pid=0x0021
enabled=false
launch_on_start=false
launcher_x_action=minimize-to-tray
controller_adapter=adx-io4
jvs_endpoint=\\.\mxjvs
reopen_after_game_close=true
orientation=auto
keep_controller_alive_when_minimized=false
controller_only_start=false
jvs_player=p1
open_on_os_startup=false
jvs_hardware_validated=false
ba1_enabled=true
monitor_adapter
"#;

pub(super) const V2_FULL: &str = r#"[game.3]
close_script=C:\遊戲\close.bat
selected_target=C:\遊戲\example.exe
controller_image=launcher_assets/game_images/3.png
plugin_sha256=ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01
config_target2=C:\遊戲\tool.exe
close_hotkey=Ctrl+Shift+F6
plugin_copy_zero_matches_to_game_folder=true
close_mode=script
plugin_update_policy=pinned
plugin_assignment_id=0123456789abcdef0123456789abcdef
label=日本語 ゲーム
plugin_resolved_launcher=C:\遊戲\launcher.exe
hotkey=Ctrl+Shift+F5
config_target1=C:\遊戲\settings.ini
plugin_resolved_watch_target
plugin_id=example.plugin
plugin_search_directory_destinations=false
plugin_grant1={"pluginId":"example.plugin","gameId":3,"revisionSha256":"ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01ab01","destinationRoot":"C:\\遊戲","rootIdentity":{"volumeSerial":305419896,"fileIndex":42}}
plugin_version=1.2.3
watch_exe=example.exe
plugin_ledger_reference=C:\State\plugins\ledgers\3.toml
plugin_allow_prerelease=false
[game_launcher_screen]
launcher_x_action=exit-launcher
reopen_after_game_close=true
open_on_os_startup=true
controller_adapter=adx-hid
launch_on_start=true
jvs_endpoint=COM4
calibration_monitor_id=\\?\DISPLAY#ABC1234#5&1a2b3c4d&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}
calibration_rotation=270
calibration_profile=cabinet-portrait
calibration_mode_height=1920
ba2_enabled=false
background_image=C:\Art #2\background.png
orientation=cabinet-portrait
keep_controller_alive_when_minimized=true
enabled=true
vid=0x2E3C
controller_only_start=false
jvs_player=p2
pid=0x5750
ba1_enabled=true
jvs_hardware_validated=true
calibration_diameter_px=1050
calibration_mode_width=1080
monitor_adapter=2
[game.5]
label=Script Game
close_mode=kill
selected_target=D:\Games\Script\start.bat
[launcher]
pause_on_failure=true
next_game_id=6
launcher_enabled=true
auto_hide_on_launch=false
default_game=3
show_command_windows=true
schema_version=2
launch_default_on_open=true
launcher_hotkey=Ctrl+Alt+J
close_game_hotkey=Ctrl+Alt+K
diagnostics_logging=true
block_exit_on_credit=true
language=zh-Hant
adapter=1
"#;

pub(super) const V2_JVS: &str = r#"[game_launcher_screen]
vid=0x0000
jvs_endpoint=\\.\mxjvs
orientation=auto
controller_only_start=false
launcher_x_action=minimize-to-tray
monitor_adapter
jvs_hardware_validated=true
pid=0x0000
ba1_enabled=true
ba2_enabled=true
background_image
open_on_os_startup=false
jvs_player=p1
keep_controller_alive_when_minimized=false
enabled=true
reopen_after_game_close=true
launch_on_start=false
controller_adapter=jvs
[launcher]
auto_hide_on_launch=true
pause_on_failure=false
default_game
next_game_id=1
launcher_hotkey=Ctrl+Alt+M
close_game_hotkey=
language=
adapter
diagnostics_logging=false
launcher_enabled=true
show_command_windows=false
block_exit_on_credit=false
launch_default_on_open=false
schema_version=2
"#;

pub(super) const V2_IO4: &str = r#"[launcher]
close_game_hotkey=
auto_hide_on_launch=true
schema_version=2
launcher_enabled=true
default_game
diagnostics_logging=false
pause_on_failure=false
next_game_id=2
show_command_windows=false
adapter
block_exit_on_credit=false
launch_default_on_open=false
language=
launcher_hotkey=Ctrl+Alt+M
[game_launcher_screen]
ba2_enabled=true
background_image
keep_controller_alive_when_minimized=false
monitor_adapter
controller_only_start=false
open_on_os_startup=false
vid=0x0CA3
pid=0x0022
orientation=auto
enabled=false
jvs_player=p1
jvs_hardware_validated=false
ba1_enabled=true
launch_on_start=false
reopen_after_game_close=true
controller_adapter=adx-io4
launcher_x_action=minimize-to-tray
jvs_endpoint=\\.\mxjvs
[game.1]
selected_target=C:\Games\Only\only.exe
close_mode=kill
label=Only Game
"#;

pub(super) const V1_SEED: &str = r#"[launcher]
launcher_enabled=true
auto_hide_on_launch=true
show_command_windows=false
launch_default_on_open=false
diagnostics_logging=false
default_game=
next_game_id=1
"#;

pub(super) const V1_SEED_RELEASE: &str = r#"[launcher]
launcher_enabled=true
auto_hide_on_launch=true
show_command_windows=false
launch_default_on_open=false
default_game=
next_game_id=1
"#;
