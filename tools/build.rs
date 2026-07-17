use std::{env, path::Path};
use flate2::{write::GzEncoder, Compression};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let sdk_template_dir = Path::new(&manifest_dir).join("../sdk-template");

    // Rerun whenever sdk-template changes
    println!("cargo:rerun-if-changed={}", sdk_template_dir.display());

    let out_dir = env::var("OUT_DIR").unwrap();
    let tarball_path = Path::new(&out_dir).join("sdk-template.tar.gz");

    let file = std::fs::File::create(&tarball_path).expect("failed to create sdk-template.tar.gz");
    let encoder = GzEncoder::new(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);

    archive
        .append_dir_all("sdk", &sdk_template_dir)
        .expect("failed to archive sdk-template");

    archive.finish().expect("failed to finalize tarball");
}
