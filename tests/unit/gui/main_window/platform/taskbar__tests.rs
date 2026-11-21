use super::*;

const WS_EX_WINDOWEDGE: isize = 0x0000_0100;
const WS_EX_ACCEPTFILES: isize = 0x0000_0010;
const TOOL: isize = 0x0000_0080;
const APP: isize = 0x0004_0000;

#[test]
fn the_style_without_a_button_is_a_tool_window_and_not_an_app_window() {
    let winit = WS_EX_WINDOWEDGE | WS_EX_ACCEPTFILES | APP;
    let hidden = without_taskbar_button(winit);
    assert_eq!(hidden & TOOL, TOOL);
    assert_eq!(hidden & APP, 0);
    assert_eq!(
        hidden & (WS_EX_WINDOWEDGE | WS_EX_ACCEPTFILES),
        WS_EX_WINDOWEDGE | WS_EX_ACCEPTFILES,
        "every other bit stays"
    );
}

#[test]
fn the_style_with_a_button_is_the_one_winit_writes() {
    let winit = WS_EX_WINDOWEDGE | WS_EX_ACCEPTFILES | APP;
    assert_eq!(with_taskbar_button(without_taskbar_button(winit)), winit);
    assert_eq!(with_taskbar_button(winit), winit);
    assert_eq!(
        without_taskbar_button(without_taskbar_button(winit)),
        without_taskbar_button(winit)
    );
}

#[test]
fn the_constants_are_the_windows_ones() {
    assert_eq!(WS_EX_TOOLWINDOW.0 as isize, TOOL);
    assert_eq!(WS_EX_APPWINDOW.0 as isize, APP);
}
