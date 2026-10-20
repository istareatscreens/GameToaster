use super::*;

static BOUND: ControllerDriverDescriptor = ControllerDriverDescriptor {
    id: "bound",
    label_key: "controller.type.adx_io4",
    recommended_layout: "washing-machine",
    options: &[],
    lights: None,
    bindings: &[
        ButtonBinding {
            button: Button::Ba7,
            command: LauncherCommand::Previous,
        },
        ButtonBinding {
            button: Button::Select,
            command: LauncherCommand::Launch,
        },
    ],
};

#[test]
fn command_for_returns_only_the_bound_command() {
    assert_eq!(
        BOUND.command_for(Button::Ba7),
        Some(LauncherCommand::Previous)
    );
    assert_eq!(
        BOUND.command_for(Button::Select),
        Some(LauncherCommand::Launch)
    );
    for button in Button::ALL {
        if button != Button::Ba7 && button != Button::Select {
            assert_eq!(BOUND.command_for(button), None, "{button:?}");
        }
    }
}
