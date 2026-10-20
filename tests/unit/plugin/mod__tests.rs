use std::fs;
use std::path::Path;

#[test]
fn plugin_layer_has_no_gui_dependency() {
    let plugin_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("plugin");
    let mut pending = vec![plugin_root];
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let source = fs::read_to_string(&path).unwrap();
            for forbidden in [
                "crate::gui",
                "crate::launcher_gui",
                "crate::game_launcher_screen",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{} depends on the GUI through {forbidden}",
                    path.display()
                );
            }
        }
    }
}
