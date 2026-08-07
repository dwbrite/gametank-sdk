#[cfg(target_arch = "mos")]
use core::panic::PanicInfo;
use core::ptr;

use crate::{blitter::SpriteQuadrant, console::Console};

unsafe extern "Rust" {
    unsafe fn main(console: &mut Console);
}

#[cfg(target_arch = "mos")]
#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}

#[unsafe(link_section = ".data.zp")]
pub static mut VBLANK: bool = false;

unsafe extern "C" {
    pub unsafe fn return_from_interrupt();

    pub unsafe fn wait();

    pub unsafe fn enable_irq_handler();

    pub unsafe fn disable_irq_handler();

    /// Set the overflow (V) flag. Used by llvm-mos for certain operations.
    // pub unsafe fn __set_v();

    pub unsafe static mut __rc50: u8;
    pub unsafe static mut __rc51: u8;
    pub unsafe static mut __rc0: u8;
    pub unsafe static mut __rc1: u8;

    unsafe static __data_load: u8;
    unsafe static mut __data_start: u8;
    unsafe static mut __data_end: u8;

    unsafe static __zp_load: u8;
    unsafe static mut __zp_start: u8;
    unsafe static mut __zp_end: u8;

    unsafe static mut __bss_start: u8;
    unsafe static mut __bss_end: u8;
}

#[inline(always)]
unsafe fn init_data_and_bss() {
    unsafe {
        // Copy .data from flash to RAM
        let mut src = &__data_load as *const u8;
        let mut dst = &raw mut __data_start as *mut u8;
        let end = &raw mut __data_end as *mut u8;
        while dst < end {
            dst.write_volatile(src.read_volatile());
            src = src.add(1);
            dst = dst.add(1);
        }

        // Zero .bss
        let mut bss = &raw mut __bss_start as *mut u8;
        let bss_end = &raw mut __bss_end as *mut u8;
        while bss < bss_end {
            bss.write_volatile(0);
            bss = bss.add(1);
        }

        // Copy .zp load to zp
        let mut src = &__zp_load as *const u8;
        let mut dst = &raw mut __zp_start as *mut u8;
        let end = &raw mut __zp_end as *mut u8;
        while dst < end {
            dst.write_volatile(src.read_volatile());
            src = src.add(1);
            dst = dst.add(1);
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn vblank_nmi() {
    unsafe {
        VBLANK = true;
        return_from_interrupt();
    }
}

#[unsafe(link_section = ".vector_table")]
#[unsafe(no_mangle)]
pub static _VECTOR_TABLE: [unsafe extern "C" fn(); 3] = [
    vblank_nmi,            // Non-Maskable Interrupt vector
    __boot,                // Reset vector
    return_from_interrupt, // IRQ/BRK vector
];


#[inline(never)]
fn call_main() {
    let console = &mut Console::init();
    if let Some(mut blitter) = console.dma.blitter(&mut console.video_flags) {
        blitter.draw_square(0, 0, 10, 10, 0b1010_1010);
    }

    if let Some(mut blitter) = console.dma.blitter(&mut console.video_flags) {
        blitter.set_vram_quad(SpriteQuadrant::One);
    }

    unsafe { main(console) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __boot() {
    unsafe {
        reset_banking_register();
        init_data_and_bss();
        init_stack();

        // IMPORTANT: we can't initialize Console in __boot,
        // because it messes with the stack BEFORE the stack is initialized
        // we _can_, however, toss the initialization in a function which is never inlined :^)
        call_main();
        core::panic!("Came out of main");
    }
}

#[inline(always)]
unsafe fn init_stack() {
    unsafe { __rc0 = 0xFF };
    unsafe { __rc1 = 0x1F }
}

#[inline(always)]
unsafe fn reset_banking_register() {
    let bank_reg: *mut u8 = 0x2005 as *mut u8;
    unsafe { ptr::write_volatile(bank_reg, 0) };
}
