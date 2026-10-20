use super::*;

fn manifest(config_copy: &str, plugin_copy: &str) -> Manifest {
    let config_replace = if config_copy == "absolute" {
        "configFileReplacePath = \"./example.ini\"\n"
    } else {
        ""
    };
    let plugin_replace = if plugin_copy == "find_and_replace_in_directory" {
        "pluginFileReplacePath = \"./mods\"\n"
    } else {
        ""
    };
    Manifest::parse(&format!(
        "manifestVersion = 1\n\
         pluginId = \"org.example.plugin\"\n\
         pluginVersion = \"1.0.0\"\n\
         label = \"Example\"\n\
         description = \"Example plugin\"\n\
         \n\
         [[configFiles]]\n\
         configFileLocation = \"./example.ini\"\n\
         configFileCopyType = \"{config_copy}\"\n\
         {config_replace}\n\
         [[pluginFiles]]\n\
         pluginFileLocation = \"./example.dll\"\n\
         pluginFileCopyType = \"{plugin_copy}\"\n\
         {plugin_replace}"
    ))
    .unwrap()
}

fn drawn(manifest: &Manifest) -> (Vec<String>, f32) {
    let context = egui::Context::default();
    let mut track_latest = true;
    let mut zero = false;
    let mut directory = false;
    let mut height = 0.0;
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        show(
            ui,
            manifest,
            OptionWording::GameSettings,
            PluginOptions {
                track_latest: &mut track_latest,
                copy_zero_matches_to_game_folder: &mut zero,
                search_directory_destinations: &mut directory,
            },
        );
        height = ui.min_rect().height();
    });
    let texts = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_string()),
            _ => None,
        })
        .collect();
    (texts, height)
}

fn all_distinct_and_non_empty(texts: &[String]) -> bool {
    let mut unique = texts.to_vec();
    unique.sort();
    unique.dedup();
    unique.len() == texts.len() && texts.iter().all(|text| !text.trim().is_empty())
}

#[test]
fn find_and_replace_options_apply_only_to_replacing_manifests() {
    assert!(!manifest_uses_find_and_replace(&manifest(
        "absolute",
        "game_folder"
    )));
    assert!(manifest_uses_find_and_replace(&manifest(
        "find_and_replace",
        "game_folder"
    )));
    assert!(manifest_uses_find_and_replace(&manifest(
        "absolute",
        "find_and_replace_in_directory"
    )));
}

#[test]
fn options_section_hides_irrelevant_choices_and_never_offers_prerelease() {
    let (plain, plain_height) = drawn(&manifest("absolute", "game_folder"));
    let (replacing, replacing_height) = drawn(&manifest("find_and_replace", "game_folder"));

    assert_eq!(plain.len(), 3, "{plain:?}");
    assert!(all_distinct_and_non_empty(&plain), "{plain:?}");
    assert!(
        plain[2].chars().count() > plain[0].chars().count()
            && plain[2].chars().count() > plain[1].chars().count(),
        "{plain:?}"
    );

    assert_eq!(replacing.len(), 7, "{replacing:?}");
    assert!(all_distinct_and_non_empty(&replacing), "{replacing:?}");
    assert!(
        replacing[4].chars().count() > replacing[3].chars().count(),
        "{replacing:?}"
    );
    assert!(
        replacing[6].chars().count() > replacing[5].chars().count(),
        "{replacing:?}"
    );
    assert!(replacing_height > plain_height);
}
