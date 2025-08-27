use std::{env, fs::File, io::Write, path::Path};
use std::process::Command;

fn assemble_and_archive() {
    std::fs::create_dir_all("target/asm").unwrap();

    // assemble all .asm files
    let status = Command::new("bash")
        .arg("-c")
        .arg(r#"find . -name "*.asm" -exec bash -c 'filename=$(basename "{}" .asm); echo "Assembling $filename..."; llvm-mc --filetype=obj -triple=mos -mcpu=mosw65c02 "{}" -o "target/asm/$filename.o"' \;"#)
        .status()
        .expect("failed to run find/llvm-mc");
    assert!(status.success());

    // archive and clean up
    let status = Command::new("bash")
        .arg("-c")
        .arg(r#"llvm-ar rcs target/asm/libasm.a target/asm/*.o && rm target/asm/*.o"#)
        .status()
        .expect("failed to run llvm-ar");
    assert!(status.success());
}

fn generate_linkerscript() {

    let out_dir = env::var("OUT_DIR").unwrap();
    let link_path = Path::new(&out_dir).join("linker.ld");
    let mut f = File::create(&link_path).expect("failed to create linker.ld");

    // dump the static header in one shot
    const LINKER_HEADER: &str = r#"
MEMORY {
  /* 0.5k reserved for zp + hw stack */
  RESERVED (rw)  : ORIGIN = 0x0000, LENGTH = 0x0041
  ZP (rw)        : ORIGIN = 0x0041, LENGTH = 0x00C0
  STACK (rw) : ORIGIN = 0x0100, LENGTH = 0x0100 

  /* 1k for volume table */
  VOL (rwx)     : ORIGIN = 0x0200, LENGTH = 0x400

  /* 2k for wavetables */
  WAVE (rwx)     : ORIGIN = 0x0600, LENGTH = 0x800
  /* 0.5 kb reserved for program, + ideally empty stack */
  ARAM (rwx)     : ORIGIN = 0x0E00, LENGTH = 0x2FA
  VECTOR_TABLE(rw): ORIGIN = 0x0FFA, LENGTH = 6

  SAMPLE (w)     : ORIGIN = 0x8000, LENGTH = 0x8000
}

SECTIONS {
  .header : { . = 0x0000; BYTE(0); } > RESERVED
  .text : { *(.text*) } > ARAM = 0xFF
  .volume : { *(.const.vol*) } > VOL
  .wave : { *(.const.wavetables*) } > WAVE
  
  .rodata : { *(.rodata*) } > ARAM

  .vector_table : { KEEP(*(.vector_table)) } > VECTOR_TABLE
  .bss : { __bss_start = .; *(.bss*) __bss_end = .; } > ARAM
  .zp : { 
    __zp_start = .;

    /* put .data.voices at zp 0x41, should take 81 bytes - should take us to =0xA1 */
    . = 0x0041;
    __voices_start = .;
    KEEP(*(.data.voices))
    __voices_end = .;
    KEEP(*(.data.zp))
    
    __zp_end = .;
  } > ZP
  .data : { __data_start = .; *(.data*) __data_end = .; } > ARAM

  PROVIDE(__zp_load   = LOADADDR(.zp));
  PROVIDE(__zp_start  = ADDR(.zp));
  PROVIDE(__zp_end    = .);

  PROVIDE(__data_load  = LOADADDR(.data));
  PROVIDE(__data_start = ADDR(.data));
  PROVIDE(__data_end   = .);

  PROVIDE(__bss_start = ADDR(.bss));
  PROVIDE(__bss_end   = .);
}
"#;

    f.write_all(LINKER_HEADER.as_bytes()).unwrap();

    // only looped bit
    for rc in 0..=63 {
        writeln!(f, "__rc{} = 0x{:02X};", rc, rc).unwrap();
    }

    // Hook up the linker script
    println!("cargo:rustc-link-arg=-T{}", link_path.display());
}


fn main() {
    // Only run for the correct target
    let target = env::var("TARGET").unwrap();
    if target != "mos-unknown-none" {
        println!(
            "cargo:warning=Not targeting mos-unknown-none; skipping linker script generation."
        );
        return;
    }

    assemble_and_archive();
    generate_linkerscript();

    // Preserve static asm lib
    println!("cargo:rustc-link-search=native=target/asm");
    println!("cargo:rustc-link-lib=static=asm");
}
