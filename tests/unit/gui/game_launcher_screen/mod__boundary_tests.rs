use super::*;

fn assert_send<T: Send>() {}

#[test]
fn facade_is_send_and_concrete_modules_remain_private() {
    assert_send::<ControllerLauncher>();
}

#[test]
fn every_builtin_driver_recommends_a_registered_layout() {
    let layouts = ui::layouts::LauncherLayoutRegistry::with_builtin_layouts();
    for driver in controller::AdapterRegistry::with_default_adapters().descriptors() {
        assert_eq!(
            layouts.validate(driver.recommended_layout),
            Ok(()),
            "controller driver '{}' recommends an unregistered layout",
            driver.id
        );
    }
}

#[test]
fn saving_a_background_imports_it_and_retires_the_previous_copy_after_the_write() {
    use image::{ImageBuffer, Rgba};
    let dir = std::env::temp_dir().join(format!(
        "gametoaster-background-save-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let picture = |name: &str, red: u8| {
        let path = dir.join(name);
        ImageBuffer::from_pixel(20, 10, Rgba([red, 0, 0, 255]))
            .save(&path)
            .unwrap();
        path
    };
    let first = picture("first.png", 10);
    let second = picture("second.png", 200);

    let saved = resolve_background_image(&dir, Some(first.clone())).unwrap();
    assert!(saved.is_relative() && dir.join(&saved).is_file());
    assert_eq!(background_copy_to_retire(None, Some(&saved)), None);

    let replaced = resolve_background_image(&dir, Some(second)).unwrap();
    assert_ne!(replaced, saved);
    assert!(dir.join(&saved).is_file() && dir.join(&replaced).is_file());
    assert_eq!(
        background_copy_to_retire(Some(&saved), Some(&replaced)),
        Some(saved.clone())
    );
    assert!(retire_background_copy(&dir, &saved, Some(&replaced)).unwrap());
    assert!(!dir.join(&saved).exists() && dir.join(&replaced).is_file());

    assert_eq!(
        resolve_background_image(&dir, Some(replaced.clone())),
        Some(replaced.clone())
    );
    assert_eq!(
        background_copy_to_retire(Some(&replaced), Some(&replaced)),
        None
    );

    let missing = dir.join("missing.png");
    assert_eq!(
        resolve_background_image(&dir, Some(missing.clone())),
        Some(missing.clone())
    );
    assert_eq!(
        background_copy_to_retire(Some(&replaced), Some(&missing)),
        None
    );

    assert!(!retire_background_copy(&dir, &replaced, Some(&replaced)).unwrap());
    assert!(dir.join(&replaced).is_file());
    assert_eq!(resolve_background_image(&dir, None), None);
    assert_eq!(
        background_copy_to_retire(Some(&replaced), None),
        Some(replaced.clone())
    );
    assert!(retire_background_copy(&dir, &replaced, None).unwrap());
    assert!(!dir.join(&replaced).exists());

    assert_eq!(background_copy_to_retire(Some(&missing), None), None);
    std::fs::remove_dir_all(dir).unwrap();
}
