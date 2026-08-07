use std::{env, fs, fs::File, io::Write, path::Path, process::Command};

fn main() {
    // Only run for the correct target
    let target = env::var("TARGET").unwrap();
    if target != "mos-unknown-none" {
        println!(
            "cargo:warning=Not targeting mos-unknown-none; skipping linker script generation."
        );
        return;
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let link_path = Path::new(&out_dir).join("linker.ld");
    let mut f = File::create(&link_path).expect("failed to create linker.ld");

    // Write your full memory layout here
    writeln!(f, "MEMORY {{").unwrap();
    for bank in 0..=126 {
        let addr = 0x8000 + bank * 0x10000;
        writeln!(
            f,
            "  BANK{0} (rx) : ORIGIN = 0x{1:06X}, LENGTH = 0x4000",
            bank, addr
        )
        .unwrap();
    }
    writeln!(f, "  RAM (rwx) : ORIGIN = 0x0400, LENGTH = 0x1BFF").unwrap();
    writeln!(f, "  ZP (rw) : ORIGIN = 0x0040, LENGTH = 0x00C0").unwrap();
    writeln!(f, "  SCR (w) : ORIGIN = 0x2000, LENGTH = 0x0008").unwrap();
    writeln!(f, "  FIXED_FLASH (rx) : ORIGIN = 0x0C000, LENGTH = 0x3FFA").unwrap();
    writeln!(f, "  VECTOR_TABLE (rw) : ORIGIN = 0x0FFFA, LENGTH = 6").unwrap();
    writeln!(f, "}}").unwrap();

    writeln!(f, "SECTIONS {{").unwrap();
    for bank in 0..=126 {
        writeln!(f, "  .text.bank{0} : {{ KEEP(*(.text.bank{0})) KEEP(*(.text.bank{0}.*)) }} > BANK{0} = 0xFF", bank).unwrap();
        writeln!(f, "  .rodata.bank{0} : {{ KEEP(*(.rodata.bank{0})) KEEP(*(.rodata.bank{0}.*)) }} > BANK{0}", bank).unwrap();
    }

    writeln!(f, "  .text : {{ *(.text*) }} > FIXED_FLASH = 0xFF").unwrap();
    writeln!(f, "  .rodata : {{ *(.rodata*) }} > FIXED_FLASH").unwrap();

    // writeln!(f, "  .init : {{ KEEP(*(.init)) }} > FIXED_FLASH").unwrap();

    writeln!(
        f,
        "  .vector_table : {{ KEEP(*(.vector_table)) }} > VECTOR_TABLE"
    )
    .unwrap();
    writeln!(
        f,
        "  .bss : {{ __bss_start = .; *(.bss*) __bss_end = .; }} > RAM"
    )
    .unwrap();
    writeln!(
        f,
        "  .zp : {{ __zp_start = .; KEEP(*(.data.zp)) __zp_end = .;}} > ZP AT > FIXED_FLASH"
    )
    .unwrap();
    writeln!(
        f,
        "  .data : {{ __data_start = .; *(.data*) __data_end = .; }} > RAM AT > FIXED_FLASH"
    )
    .unwrap();

    writeln!(f, "  PROVIDE(__zp_load = LOADADDR(.zp));").unwrap();
    writeln!(f, "  PROVIDE(__zp_start = ADDR(.zp));").unwrap();
    writeln!(f, "  PROVIDE(__zp_end = .);").unwrap();

    writeln!(f, "  PROVIDE(__data_load = LOADADDR(.data));").unwrap();
    writeln!(f, "  PROVIDE(__data_start = ADDR(.data));").unwrap();
    writeln!(f, "  PROVIDE(__data_end = .);").unwrap();

    writeln!(f, "  PROVIDE(__bss_start = ADDR(.bss));").unwrap();
    writeln!(f, "  PROVIDE(__bss_end = .);").unwrap();

    writeln!(f, "}}").unwrap();

    for rc in 0..=63 {
        writeln!(f, "__rc{} = 0x{:02X};", rc, rc).unwrap();
    }

    // Hook up the linker script
    println!("cargo:rustc-link-arg=-T{}", link_path.display());

    // Assemble src/asm/*.asm into target/asm/libasm.a
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    assemble_asm_lib(&manifest_dir, &out_dir);
    println!("cargo:rustc-link-search=native={}/target/asm", manifest_dir);
    println!("cargo:rustc-link-lib=static=asm");
}

/// Assemble every `src/asm/*.asm` file with `mos-clang` and archive them into
/// `target/asm/libasm.a` using `llvm-ar`, so the linker can find `-lasm`.
fn assemble_asm_lib(manifest_dir: &str, out_dir: &str) {
    let asm_src_dir = Path::new(manifest_dir).join("src/asm");
    let asm_out_dir = Path::new(manifest_dir).join("target/asm");
    fs::create_dir_all(&asm_out_dir).expect("failed to create target/asm");

    println!("cargo:rerun-if-changed={}", asm_src_dir.display());

    let mut obj_paths: Vec<std::path::PathBuf> = Vec::new();

    for entry in fs::read_dir(&asm_src_dir).expect("failed to read src/asm") {
        let entry = entry.expect("failed to read dir entry");
        let src = entry.path();
        if src.extension().and_then(|e| e.to_str()) != Some("asm") {
            continue;
        }
        println!("cargo:rerun-if-changed={}", src.display());

        let stem = src.file_stem().unwrap().to_str().unwrap();
        let obj = Path::new(out_dir).join(format!("asm-{stem}.o"));

        let status = Command::new("mos-clang")
            .args(["-c", "--target=mos-unknown-none", "-mcpu=mosw65c02"])
            .arg(&src)
            .arg("-o")
            .arg(&obj)
            .status()
            .expect("mos-clang not found; ensure the LLVM MOS SDK is on PATH");
        assert!(
            status.success(),
            "mos-clang failed assembling {}",
            src.display()
        );

        obj_paths.push(obj);
    }

    let lib = asm_out_dir.join("libasm.a");
    let status = Command::new("llvm-ar")
        .arg("crs")
        .arg(&lib)
        .args(&obj_paths)
        .status()
        .expect("llvm-ar not found; ensure the LLVM MOS SDK is on PATH");
    assert!(status.success(), "llvm-ar failed creating libasm.a");
}
