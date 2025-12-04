use std::{env, fs::File, io::Write, path::Path};

fn main() {
    // Only run for the correct target
    let target = env::var("TARGET").unwrap();
    if target != "mos-unknown-none" {
        println!(
            "cargo:warning=Not targeting mos-unknown-none; skipping linker script generation."
        );
        return;
    }

    // println!("cargo:rerun-if-changed=../audio/src/");
    // println!("cargo:rerun-if-changed=target/audiofw.bin");
    // println!("cargo:rerun-if-changed=../audio/Cargo.toml");

    // assert!(Command::new("cargo")
    //     .args(["+mos","build","--release","-Z","build-std=core","--target","mos-unknown-none"])
    //     .current_dir("../audio")
    //     .status().unwrap().success());
    // println!("cargo:warning=audiofw successfully built");
    // assert!(Command::new("llvm-objcopy")
    //     .args(["-O","binary",
    //            "../audio/target/mos-unknown-none/release/audiofw",
    //            "target/audiofw.bin"])
    //     .status().unwrap().success());
    // println!("cargo:warning=Generated target/audiofw.bin");

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

    // Preserve static asm lib - use absolute path for container compatibility
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/target/asm", manifest_dir);
    println!("cargo:rustc-link-lib=static=asm");
}
