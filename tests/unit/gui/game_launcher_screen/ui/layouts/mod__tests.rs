use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::launcher_gui::model::GameId;

static FAKE_DESCRIPTOR: LauncherLayoutDescriptor = LauncherLayoutDescriptor {
    id: "fake-grid",
    label_key: "controller.layout.washing_machine",
    surface: SurfaceShape::Circle,
};

fn surface() -> PixelRect {
    PixelRect {
        x: 1,
        y: 2,
        width: 3,
        height: 3,
    }
}

struct FakeLayout {
    resources: Arc<AtomicUsize>,
}

impl LauncherScreenLayout for FakeLayout {
    fn descriptor(&self) -> &'static LauncherLayoutDescriptor {
        &FAKE_DESCRIPTOR
    }

    fn activate(&mut self) {
        self.resources.store(1, Ordering::SeqCst);
    }

    fn library_changed(&mut self, _config: &LauncherConfig) {}

    fn deactivate(&mut self) {
        self.resources.store(0, Ordering::SeqCst);
    }

    fn render(&mut self, _ui: &mut egui::Ui, frame: &LauncherFrame<'_>) -> Option<LauncherCommand> {
        assert_eq!(frame.games.selected(), Some(GameId(7)));
        assert_eq!(frame.surface_px, surface());
        assert!(matches!(frame.mode, LauncherFrameMode::Menu { .. }));
        Some(LauncherCommand::Next)
    }

    fn resource_count(&self) -> usize {
        self.resources.load(Ordering::SeqCst)
    }
}

#[test]
fn fake_layout_receives_shared_frame_returns_action_and_releases_resources() {
    let resources = Arc::new(AtomicUsize::new(0));
    let mut registry = LauncherLayoutRegistry::with_builtin_layouts();
    registry
        .register(Box::new(FakeLayout {
            resources: resources.clone(),
        }))
        .unwrap();
    registry.activate("fake-grid").unwrap();
    assert_eq!(resources.load(Ordering::SeqCst), 1);

    let mut config = LauncherConfig::default();
    config.settings.controller_screen.launcher_layout = "fake-grid".into();
    let mut games = CircularGameList::default();
    games.rebuild([GameId(7)]);
    let mut action = None;
    let context = egui::Context::default();
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        let frame = LauncherFrame {
            launcher_dir: Path::new("."),
            config: &config,
            background: None,
            games: &games,
            surface_px: surface(),
            mode: LauncherFrameMode::Menu {
                credit_readout: Some("1"),
            },
        };
        action = registry.active_mut("fake-grid").unwrap().render(ui, &frame);
    });
    assert_eq!(action, Some(LauncherCommand::Next));

    registry.deactivate();
    assert_eq!(resources.load(Ordering::SeqCst), 0);
    assert!(registry.active_mut("fake-grid").is_err());
}

#[test]
fn switching_layouts_deactivates_previous_and_duplicate_unknown_ids_fail() {
    let _locale = crate::gui::common::harness::locale("en");
    let resources = Arc::new(AtomicUsize::new(0));
    let mut registry = LauncherLayoutRegistry::with_builtin_layouts();
    registry
        .register(Box::new(FakeLayout {
            resources: resources.clone(),
        }))
        .unwrap();
    let duplicate = registry.register(Box::new(FakeLayout {
        resources: Arc::new(AtomicUsize::new(0)),
    }));
    assert!(duplicate.unwrap_err().shown.contains("already registered"));
    assert!(registry
        .validate("missing-layout")
        .unwrap_err()
        .shown
        .contains("not registered"));

    registry.activate("fake-grid").unwrap();
    registry.activate("washing-machine").unwrap();
    assert_eq!(resources.load(Ordering::SeqCst), 0);
}

#[test]
fn an_unregistered_id_gets_the_default_surface_but_is_still_refused() {
    let _locale = crate::gui::common::harness::locale("en");
    let registry = LauncherLayoutRegistry::with_builtin_layouts();
    assert_eq!(
        registry.surface_shape("washing-machine"),
        SurfaceShape::Circle
    );
    assert_eq!(
        registry.surface_shape("missing-layout"),
        SurfaceShape::default()
    );
    assert_eq!(
        registry
            .validate("missing-layout")
            .map_err(|error| error.shown),
        Err("launcher layout 'missing-layout' is not registered".to_string())
    );
}
