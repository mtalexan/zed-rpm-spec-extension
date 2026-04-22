fn main() {
    // Re-run this script whenever the grammar configuration changes.
    println!("cargo:rerun-if-changed=extension.toml");

    // Remove the grammars directory so Zed re-clones the grammar repositories
    // on the next dev-extension rebuild, picking up any rev changes.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let grammars_dir = std::path::Path::new(&manifest_dir).join("grammars");
    if grammars_dir.exists() {
        std::fs::remove_dir_all(&grammars_dir).ok();
    }
}
