use std::{env, fs::File, io::Read, path::{Path, PathBuf}};
use flate2::{write::GzEncoder, Compression};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let template = manifest.join("rom-template");
    let tarball = manifest.join("src/bin/gtrom/rom-template.tar.gz");

    println!("cargo:rerun-if-changed={}", template.display());

    // Published crates have no rom-template/ (cargo package strips nested
    // manifests), so they use the tarball committed alongside the source.
    if !template.is_dir() {
        assert!(
            tarball.exists(),
            "neither rom-template/ nor {} present",
            tarball.display()
        );
        return;
    }

    let template = template.canonicalize().unwrap();
    let enc = GzEncoder::new(File::create(&tarball).unwrap(), Compression::default());
    let mut tar = tar::Builder::new(enc);

    // Collect and sort so archive order doesn't depend on filesystem order.
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(&template)
        .into_iter()
        .filter_entry(|e| keep(e.path(), &template))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();
    files.sort();

    for path in files {
        let rel = path.strip_prefix(&template).unwrap();

        let mut buf = Vec::new();
        File::open(&path).unwrap().read_to_end(&mut buf).unwrap();

        // Fixed metadata: mtime/uid/gid would otherwise make every rebuild
        // produce different bytes and dirty the working tree.
        let mut header = tar::Header::new_gnu();
        header.set_size(buf.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        header.set_cksum();

        tar.append_data(&mut header, rel, &buf[..]).unwrap();
    }

    tar.into_inner().unwrap().finish().unwrap();
}

fn keep(path: &Path, root: &Path) -> bool {
    if path == root { return true; }
    let name = path.file_name().and_then(|s| s.to_str());
    !matches!(name, Some("target" | ".git" | "Cargo.lock" | "justfile"))
        && !name.is_some_and(|n| n.ends_with(".gtr"))
}
