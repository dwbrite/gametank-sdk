.section .text
.global __do_init_stack

__do_init_stack:
  lda #mos16lo(__stack)
  sta __rc0
  lda #mos16hi(__stack)
  sta __rc1
