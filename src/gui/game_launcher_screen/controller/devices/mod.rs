pub(super) mod adx_hid;
pub(super) mod adx_io4;
pub(super) mod jvs;
mod led_board;
mod ring_bindings;

use std::sync::Arc;

use super::framework::adapter::ControllerAdapter;

pub(in crate::gui::game_launcher_screen::controller) fn builtin_adapters(
) -> Vec<Arc<dyn ControllerAdapter>> {
    vec![
        Arc::new(adx_io4::adapter()) as Arc<dyn ControllerAdapter>,
        Arc::new(adx_hid::adapter()) as Arc<dyn ControllerAdapter>,
        Arc::new(jvs::JvsAdapter) as Arc<dyn ControllerAdapter>,
    ]
}
