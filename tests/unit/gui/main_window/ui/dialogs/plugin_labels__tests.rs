use super::*;
use crate::plugin::catalog::CatalogRevision;
use crate::plugin::manifest::Manifest;
use std::path::PathBuf;

#[test]
fn a_plugin_choice_reads_its_label_version_and_digest_start() {
    let manifest = Manifest::parse(
        r#"manifestVersion = 1
pluginId = "org.example.labels"
pluginVersion = "2.3.4"
label = "Labels fixture"
description = "Labels fixture"
"#,
    )
    .unwrap();
    let plugin = PluginDescriptor {
        revision: CatalogRevision {
            plugin_id: manifest.plugin_id.clone(),
            plugin_version: manifest.plugin_version.clone(),
            package_md5: "e".repeat(32),
            package_sha256: format!("0123456789ab{}", "f".repeat(52)),
            imported_at: 0,
            root: PathBuf::from("C:/catalog/labels"),
        },
        manifest,
    };
    assert_eq!(
        plugin_choice_label(&plugin),
        "Labels fixture 2.3.4 · 0123456789ab"
    );
}
