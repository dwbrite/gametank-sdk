MEMORY {
  /* 0.5k reserved for zp + hw stack */
  RESERVED (rw)  : ORIGIN = 0x0000, LENGTH = 0x0041
  ZP (rw)        : ORIGIN = 0x0041, LENGTH = 0x00C0
  STACK (rw)     : ORIGIN = 0x0100, LENGTH = 0x0100 

  /* 512 bytes for volume table */
  VOL (rwx)      : ORIGIN = 0x0200, LENGTH = 0x0200

  /* 2.5k for wavetables */
  WAVE (rwx)     : ORIGIN = 0x0400, LENGTH = 0x0A00
  
  /* 0.5 kb reserved for program, + ideally empty stack */
  ARAM (rwx)     : ORIGIN = 0x0E00, LENGTH = 0x2FA
  VECTOR_TABLE(rw): ORIGIN = 0x0FFA, LENGTH = 6

  SAMPLE (w)     : ORIGIN = 0x8000, LENGTH = 0x8000
}

SECTIONS {
  .header : { . = 0x0000; BYTE(0); } > RESERVED
  .text : { *(.text*) } > ARAM = 0xFF
  .volume : { KEEP(*(.const.vol*)) } > VOL
  .wave : { KEEP(*(.const.wavetables*)) } > WAVE
  
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

/* helper rc symbols (0..63) */
__rc0 = 0x00;
__rc1 = 0x01;
__rc2 = 0x02;
__rc3 = 0x03;
__rc4 = 0x04;
__rc5 = 0x05;
__rc6 = 0x06;
__rc7 = 0x07;
__rc8 = 0x08;
__rc9 = 0x09;
__rc10 = 0x0A;
__rc11 = 0x0B;
__rc12 = 0x0C;
__rc13 = 0x0D;
__rc14 = 0x0E;
__rc15 = 0x0F;
__rc16 = 0x10;
__rc17 = 0x11;
__rc18 = 0x12;
__rc19 = 0x13;
__rc20 = 0x14;
__rc21 = 0x15;
__rc22 = 0x16;
__rc23 = 0x17;
__rc24 = 0x18;
__rc25 = 0x19;
__rc26 = 0x1A;
__rc27 = 0x1B;
__rc28 = 0x1C;
__rc29 = 0x1D;
__rc30 = 0x1E;
__rc31 = 0x1F;
__rc32 = 0x20;
__rc33 = 0x21;
__rc34 = 0x22;
__rc35 = 0x23;
__rc36 = 0x24;
__rc37 = 0x25;
__rc38 = 0x26;
__rc39 = 0x27;
__rc40 = 0x28;
__rc41 = 0x29;
__rc42 = 0x2A;
__rc43 = 0x2B;
__rc44 = 0x2C;
__rc45 = 0x2D;
__rc46 = 0x2E;
__rc47 = 0x2F;
__rc48 = 0x30;
__rc49 = 0x31;
__rc50 = 0x32;
__rc51 = 0x33;
__rc52 = 0x34;
__rc53 = 0x35;
__rc54 = 0x36;
__rc55 = 0x37;
__rc56 = 0x38;
__rc57 = 0x39;
__rc58 = 0x3A;
__rc59 = 0x3B;
__rc60 = 0x3C;
__rc61 = 0x3D;
__rc62 = 0x3E;
__rc63 = 0x3F;
