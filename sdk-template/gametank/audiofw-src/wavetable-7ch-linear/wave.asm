.section .const.wavetables, "a"
.align 256
.global instrument1_table
instrument1_table:
    .incbin "../../instruments/sine.raw"

.align 256
.global instrument2_table
instrument2_table:
    .incbin "../../instruments/saw.raw"

.align 256
.global instrument3_table
instaument3_table:
    .incbin "../../instruments/triangle.raw"

.align 256
.global instrument4_table
instrument4_table:
    .incbin "../../instruments/square.raw"

.align 256
.global instrument5_table
instrument5_table:
    .incbin "../../instruments/pulse.raw"
