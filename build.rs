use std::{env, fs::File, path::{Path, PathBuf}};
use flate2::{write::GzEncoder, Compression};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let template = manifest.join("rom-template");
    let tarball = manifest.join("src/bin/gtrom/rom-template.tar.gz");

    println!("cargo:rerun-if-changed={}", template.display());

    // Published crates have no rom-template/ (cargo package strips nested
    // manifests), but ship the tarball built during `cargo package`.
    if !template.is_dir() {
        assert!(
            tarball.exists(),
            "neither rom-template/ nor src/rom-template.tar.gz present"
        );
        return;
    }

    let template = template.canonicalize().unwrap();
    let enc = GzEncoder::new(File::create(&tarball).unwrap(), Compression::default());
    let mut tar = tar::Builder::new(enc);

    for entry in walkdir::WalkDir::new(&template)
        .into_iter()
        .filter_entry(|e| keep(e.path(), &template))
    {
        let entry = entry.unwrap();
        if !entry.file_type().is_file() { continue; }
        let rel = entry.path().strip_prefix(&template).unwrap();
        tar.append_path_with_name(entry.path(), rel).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap();
}

fn keep(path: &Path, root: &Path) -> bool {
    if path == root { return true; }
    !matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("target" | ".git" | "Cargo.lock" | "justfile")
    )
}