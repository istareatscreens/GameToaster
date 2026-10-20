use super::*;

#[test]
fn left_click_restores_and_menu_items_map_to_their_actions() {
    assert_eq!(
        click_action(MouseButton::Left, MouseButtonState::Up),
        Some(TrayAction::Restore)
    );
    assert_eq!(click_action(MouseButton::Right, MouseButtonState::Up), None);

    let restore = MenuId::new("restore-test");
    let controller = MenuId::new("controller-test");
    let close = MenuId::new("close-test");
    let game = MenuId::new("game-test");
    let games = HashMap::from([(game.clone(), GameId(9))]);

    assert_eq!(
        menu_action(&restore, &restore, &controller, &close, &games),
        Some(TrayAction::ToggleMainWindow)
    );
    assert_eq!(
        menu_action(&controller, &restore, &controller, &close, &games),
        Some(TrayAction::ToggleControllerScreen)
    );
    assert_eq!(
        menu_action(&close, &restore, &controller, &close, &games),
        Some(TrayAction::Close)
    );
    assert_eq!(
        menu_action(&game, &restore, &controller, &close, &games),
        Some(TrayAction::Launch(GameId(9)))
    );
}
