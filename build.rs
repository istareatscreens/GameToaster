fn main() {
    println!("cargo:rerun-if-changed=locales");
    println!("cargo:rerun-if-changed=assets/gametoaster.rc");
    println!("cargo:rerun-if-changed=assets/toaster.ico");
    println!("cargo:rerun-if-changed=assets/toaster.png");
    embed_resource::compile_for("assets/gametoaster.rc", ["GameToaster"], version_macros())
        .manifest_optional()
        .unwrap();
}

fn version_macros() -> [String; 4] {
    let part = |name: &str| std::env::var(name).expect("cargo sets the package version");
    [
        format!("VERSION_MAJOR={}", part("CARGO_PKG_VERSION_MAJOR")),
        format!("VERSION_MINOR={}", part("CARGO_PKG_VERSION_MINOR")),
        format!("VERSION_PATCH={}", part("CARGO_PKG_VERSION_PATCH")),
        format!("VERSION_TEXT=\"{}\"", part("CARGO_PKG_VERSION")),
    ]
}
