.section .text
.global wait, return_from_interrupt, enable_irq_handler, disable_irq_handler, __set_v

wait:
    WAI
    RTS

return_from_interrupt:
    RTI

enable_irq_handler:
    CLI
    RTS

disable_irq_handler:
    SEI
    RTS

; Set the overflow flag (V)
; Used by llvm-mos for certain operations
; 65C02S has BIT #imm, so we can use immediate mode
__set_v:
    BIT #0x40
    RTS
