use core::{arch::asm, panic::PanicInfo, ptr};

use crate::{audio_irq};

#[panic_handler]
#[unsafe(no_mangle)]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}

unsafe extern "C" {
    #[inline(always)]
    pub unsafe fn return_from_interrupt();

    #[inline(always)]
    pub unsafe fn wait();

    #[inline(always)]
    pub unsafe fn enable_irq_handler();

    #[inline(always)]
    pub unsafe fn disable_irq_handler();

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

// #[inline(always)]
// unsafe fn init_data_and_bss() {
//     unsafe {
//         // Copy .data from flash to RAM
//         let mut src = &__data_load as *const u8;
//         let mut dst = &raw mut __data_start as *mut u8;
//         let end = &raw mut __data_end as *mut u8;
//         while dst < end {
//             dst.write_volatile(src.read_volatile());
//             src = src.add(1);
//             dst = dst.add(1);
//         }

//         // Zero .bss
//         let mut bss = &raw mut __bss_start as *mut u8;
//         let bss_end = &raw mut __bss_end as *mut u8;
//         while bss < bss_end {
//             bss.write_volatile(0);
//             bss = bss.add(1);
//         }

//         // Copy .zp load to zp
//         let mut src = &__zp_load as *const u8;
//         let mut dst = &raw mut __zp_start as *mut u8;
//         let end = &raw mut __zp_end as *mut u8;
//         while dst < end {
//             dst.write_volatile(src.read_volatile());
//             src = src.add(1);
//             dst = dst.add(1);
//         }
//     }
// }

#[unsafe(no_mangle)]
extern "C" fn vblank_nmi() {
    unsafe {
        return_from_interrupt();
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".vector_table")]
pub static _VECTOR_TABLE: [unsafe extern "C" fn(); 3] = [
    vblank_nmi, // Non-Maskable Interrupt vector
    __boot,     // Reset vector
    audio_irq,  // IRQ/BRK vector
];

#[unsafe(no_mangle)]
fn casio_loopy() {
    loop {
        unsafe { 
            wait();
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __boot() {
    unsafe {
        // init_data_and_bss();
        init_stack();

        disable_irq_handler();
        enable_irq_handler();

        casio_loopy();
        core::panic!("Came out of main");
    }
}

#[inline(always)]
#[unsafe(no_mangle)]
unsafe fn init_stack() {
    unsafe { __rc0 = 0xF9 };
    unsafe { __rc1 = 0x0F }
}
