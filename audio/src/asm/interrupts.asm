.section .text
.global wait, return_from_interrupt, enable_irq_handler, disable_irq_handler

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
