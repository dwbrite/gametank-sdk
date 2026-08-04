use std::{env, path::Path, process::Command};

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let fw_dir = Path::new(&manifest).join("audiofw");

    if env::var("CARGO_FEATURE_AUDIO_WAVETABLE_8CH").is_ok() {
        assemble_wavetable_firmware(
            "wavetable-8ch",
            &["main", "wave", "vol"],
            &manifest,
            &out_dir,
            &fw_dir,
        );
    }

    if env::var("CARGO_FEATURE_AUDIO_WAVETABLE_7CH_LINEAR").is_ok() {
        assemble_wavetable_firmware(
            "wavetable-7ch-linear",
            &["main", "wave", "vol"],
            &manifest,
            &out_dir,
            &fw_dir,
        );
    }

    if env::var("CARGO_FEATURE_AUDIO_FM_4CH").is_ok() {
        assemble_fm_firmware(&manifest, &out_dir, &fw_dir);
    }
}

/// Assemble a wavetable firmware using the LLVM MOS toolchain.
///
/// Compiles each ASM file in `audiofw-src/*/` to an object file in
/// `OUT_DIR`, links them with the firmware's linker.ld, and extracts a raw
/// 4096-byte binary with llvm-objcopy. The final BIN file is written to
/// `audiofw/<name>.bin` so `include_bytes!` in the audio module can find it.
///
/// All intermediate files (`.o`, `.elf`) stay in `OUT_DIR` and never touch
/// the source tree.
///
/// If `mos-clang` or `llvm-objcopy` are not on `PATH`, a warning is printed
/// and the pre-built binary in `audiofw/` is used as-is.
fn assemble_wavetable_firmware(
    name: &str,
    sources: &[&str],
    manifest: &str,
    out_dir: &str,
    fw_dir: &Path,
) {
    let src_dir = Path::new(manifest).join("audiofw-src").join(name);
    let linker = src_dir.join("linker.ld");
    let elf = Path::new(out_dir).join(format!("{name}.elf"));
    let bin = fw_dir.join(format!("{name}.bin"));

    println!("cargo:rerun-if-changed={}", linker.display());

    let mut obj_paths = Vec::new();
    for stem in sources {
        let src = src_dir.join(format!("{stem}.asm"));
        println!("cargo:rerun-if-changed={}", src.display());

        let obj = Path::new(out_dir).join(format!("{name}-{stem}.o"));

        let result = Command::new("mos-clang")
            .args(["-c", "--target=mos-unknown-none", "-mcpu=mosw65c02"])
            .arg(&src)
            .arg("-o")
            .arg(&obj)
            .current_dir(&src_dir)
            .status();

        match result {
            Err(_) => {
                println!(
                    "cargo:warning=mos-clang not found; using pre-built {name}.bin. \
                     Install the LLVM MOS SDK to rebuild firmware from source."
                );
                return;
            }
            Ok(s) if !s.success() => {
                panic!("mos-clang failed assembling {}", src.display());
            }
            Ok(_) => {}
        }

        obj_paths.push(obj);
    }

    // Link all objects into an ELF using the firmware's linker script.
    // -nostdlib: bare-metal firmware; do not link against libc or crt.
    let mut link_cmd = Command::new("mos-clang");
    link_cmd
        .args(["--target=mos-unknown-none", "-mcpu=mosw65c02", "-nostdlib"])
        .arg("-T")
        .arg(&linker);
    for obj in &obj_paths {
        link_cmd.arg(obj);
    }
    link_cmd.arg("-o").arg(&elf);

    let status = link_cmd
        .status()
        .expect("mos-clang not found. ensure the LLVM MOS SDK is on PATH");
    assert!(status.success(), "mos-clang failed linking {name} firmware");

    // Extract a flat binary from the ELF.
    let status = Command::new("llvm-objcopy")
        .args(["-O", "binary"])
        .arg(&elf)
        .arg(&bin)
        .status()
        .expect("llvm-objcopy not found. ensure the LLVM MOS SDK is on PATH");
    assert!(
        status.success(),
        "llvm-objcopy failed extracting {name} firmware binary"
    );
}

/// Assemble the FM firmware using the cc65 toolchain (ca65 + ld65).
///
/// The object file is written to `OUT_DIR`; the final `.bin` is written to
/// `audiofw/fm-4ch.bin`. No files are written into the source tree.
///
/// If `ca65` or `ld65` are not on `PATH`, a warning is printed and the
/// pre-built binary in `audiofw/` is used as-is.
fn assemble_fm_firmware(manifest: &str, out_dir: &str, fw_dir: &Path) {
    let src_dir = Path::new(manifest).join("audiofw-src/fm-4ch");
    let asm_src = src_dir.join("audio_fw.asm");
    let cfg = src_dir.join("gametank-acp.cfg");
    let obj = Path::new(out_dir).join("fm-4ch.o");
    let bin = fw_dir.join("fm-4ch.bin");

    println!("cargo:rerun-if-changed={}", src_dir.display());

    let result = Command::new("ca65")
        .args(["--cpu", "65c02", "--bin-include-dir"])
        .arg(&src_dir)
        .arg(&asm_src)
        .arg("-o")
        .arg(&obj)
        .status();

    match result {
        Err(_) => {
            println!(
                "cargo:warning=ca65 not found... using pre-built fm-4ch.bin. \
                 Install cc65 (https://cc65.github.io/) to rebuild firmware from source."
            );
            return;
        }
        Ok(s) if !s.success() => panic!("ca65 failed assembling fm-4ch firmware"),
        Ok(_) => {}
    }

    let status = Command::new("ld65")
        .arg("-C")
        .arg(&cfg)
        .arg(&obj)
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("ld65 not found. install cc65 (https://cc65.github.io/)");
    assert!(status.success(), "ld65 failed linking fm-4ch firmware");
}
