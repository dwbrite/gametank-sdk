use std::{env, path::Path, process::Command};

fn main() {
    // Only assemble the FM firmware when that feature is requested.
    // Build scripts read features via CARGO_FEATURE_* env vars.
    if env::var("CARGO_FEATURE_AUDIO_FM_4CH").is_ok() {
        assemble_fm_firmware();
    }
}

fn assemble_fm_firmware() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let cargo_out = env::var("OUT_DIR").unwrap();
    let src_dir  = Path::new(&manifest).join("audiofw-src/fm-4ch");
    let fw_dir   = Path::new(&manifest).join("audiofw");

    let asm_src  = src_dir.join("audio_fw.asm");
    let cfg      = src_dir.join("gametank-acp.cfg");
    let obj      = Path::new(&cargo_out).join("fm-4ch.o");
    let bin      = fw_dir.join("fm-4ch.bin");

    println!("cargo:rerun-if-changed={}", asm_src.display());
    println!("cargo:rerun-if-changed={}", cfg.display());
    println!("cargo:rerun-if-changed={}", src_dir.join("sine_256_-63_63.bin").display());

    let ca65_status = Command::new("ca65")
        .args(["--cpu", "65c02", "--bin-include-dir"])
        .arg(&src_dir)
        .arg(&asm_src)
        .arg("-o")
        .arg(&obj)
        .status()
        .expect("ca65 not found. install cc65 (https://cc65.github.io/)");

    assert!(ca65_status.success(), "ca65 failed assembling fm-4ch firmware");

    let ld65_status = Command::new("ld65")
        .arg("-C")
        .arg(&cfg)
        .arg(&obj)
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("ld65 not found. install cc65 (https://cc65.github.io/)");

    assert!(ld65_status.success(), "ld65 failed linking fm-4ch firmware");
}
