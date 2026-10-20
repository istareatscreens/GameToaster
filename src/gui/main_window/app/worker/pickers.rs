use rust_i18n::t;

use super::messages::WorkerResult;

pub(super) fn pick_executable() -> WorkerResult {
    let selected = rfd::FileDialog::new()
        .set_title(t!("picker.add_game").to_string())
        .add_filter(t!("picker.filter_executable"), &["exe", "bat", "cmd"])
        .add_filter(t!("picker.filter_all"), &["*"])
        .pick_file();
    WorkerResult::FilePicked(selected)
}

pub(super) fn pick_controller_image() -> WorkerResult {
    let selected = rfd::FileDialog::new()
        .set_title(t!("controller.file_dialog.artwork_title").to_string())
        .add_filter(
            t!("controller.file_dialog.image_filter").as_ref(),
            &["png", "jpg", "jpeg"],
        )
        .pick_file();
    WorkerResult::ControllerImagePicked(selected)
}
