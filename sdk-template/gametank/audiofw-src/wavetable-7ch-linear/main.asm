.global audio_irq
.extern vol_table
.extern instrument1_table
.extern instrument2_table
.extern instrument3_table
.extern instrument4_table
.extern instrument5_table
.section .text

; Memory map (4KB = $0000 - $0FFF) as a Markdown table:
; | Range        | Size         | Purpose                        | Notes                                      |
; |--------------|--------------|--------------------------------|--------------------------------------------|
; | $0000-$0040  | $0040 (64)   | Zero Page (Reserved)           | Fast addressing; pointers & small vars     |
; | $0041-$0089  | $0048 (72)   | Voices (8 × 9 bytes)           | VOICE_BASE = $0041, VOICE_SIZE = 9        |
; | $008A-$008C  | $0003 (3)    | IRQ temporaries                | MIX_ACCUMULATOR, TEMP_RESULT1/2           |
; | $0100-$01FF  | $0100 (256)  | CPU Stack                      | CPU stack                                  |
; | $0200-$05FF  | $0400 (1KB)  | Volume tables (4 × 256)        | vol_table_0 through vol_table_3           |
; | $0600-$0BFF  | $0600 (1.5KB)| Wavetables (6 × 256)           | WAVETABLE_BASE = $0600, WAVETABLE_SIZE = 256 |
; | $0C00-$0FF9  | $03FA (1018) | Code / other data              | ARAM region for program code              |
; | $0FFA-$0FFF  | $0006 (6)    | Vector table                   | NMI, RESET, IRQ vectors                   |
;
; Addresses are little-endian, and ranges are inclusive.

; Define the base address for the voices (zero page)
.set VOICE_BASE, 0x0041   ; zero-page base for voice control registers
.set VOICE_SIZE, 9        ; Each voice occupies 9 bytes
.set VOICE_COUNT, 7
.set VOICE_END, (VOICE_BASE + (VOICE_SIZE * VOICE_COUNT) - 1)  ; last byte used by voices

; Temporary ZP storage for IRQ
.set MIX_ACCUMULATOR, 0x008A  ; accumulator for mixing all voices
.set TEMP_RESULT1, 0x008B ; temporary storage for first vol_table result
.set TEMP_RESULT2, 0x008C ; temporary storage for second vol_table result

; Define where wavetables live
.set WAVETABLE_BASE, 0x0400    ; base address for wavetable storage
.set WAVETABLE_SIZE, 256       ; each wavetable is 256 samples (bytes)
.set WAVETABLE_COUNT, 8
.set WAVETABLE_END, (WAVETABLE_BASE + (WAVETABLE_SIZE * WAVETABLE_COUNT) - 1) ; (0x0BFF)

; Macro to define offsets for a voice
.macro DEFINE_VOICE voice_index
    .set VOICE_\voice_index\()_BASE, (VOICE_BASE + (VOICE_SIZE * \voice_index))
    .set VOICE_\voice_index\()_PHASE_L, (VOICE_\voice_index\()_BASE + 0)
    .set VOICE_\voice_index\()_PHASE_H, (VOICE_\voice_index\()_BASE + 1)
    .set VOICE_\voice_index\()_FREQ_L, (VOICE_\voice_index\()_BASE + 2)
    .set VOICE_\voice_index\()_FREQ_H, (VOICE_\voice_index\()_BASE + 3)
    .set VOICE_\voice_index\()_WAVEPTR_L, (VOICE_\voice_index\()_BASE + 4)
    .set VOICE_\voice_index\()_WAVEPTR_H, (VOICE_\voice_index\()_BASE + 5)
    .set VOICE_\voice_index\()_VOLPTR_L, (VOICE_\voice_index\()_BASE + 6)
    .set VOICE_\voice_index\()_VOLPTR_H, (VOICE_\voice_index\()_BASE + 7)
    .set VOICE_\voice_index\()_SHIFT, (VOICE_\voice_index\()_BASE + 8)
.endm

; Macro to define a WAVETABLE_n_BASE equate for a given index
.macro DEFINE_WAVETABLE idx
    .set WAVETABLE_\idx\()_BASE, (WAVETABLE_BASE + (WAVETABLE_SIZE * \idx))
.endm

; Define 7 voices
DEFINE_VOICE 0
DEFINE_VOICE 1
DEFINE_VOICE 2
DEFINE_VOICE 3
DEFINE_VOICE 4
DEFINE_VOICE 5
DEFINE_VOICE 6

; Define 8 wavetables (change count if needed)
DEFINE_WAVETABLE 0
DEFINE_WAVETABLE 1
DEFINE_WAVETABLE 2
DEFINE_WAVETABLE 3
DEFINE_WAVETABLE 4
DEFINE_WAVETABLE 5
DEFINE_WAVETABLE 6
DEFINE_WAVETABLE 7

; Macro to process a single voice and mix into TEMP_SAMPLE
.macro PROCESS_VOICE n
    ; Check if voice is silent (shift >= 4)
    lda VOICE_\n\()_SHIFT
    cmp #4
    bcs silent_voice_\n         ; If shift >= 4, contribute 0x08 and skip processing
    
    ; Add FREQ to PHASE (16-bit addition)
    clc
    lda VOICE_\n\()_PHASE_L
    adc VOICE_\n\()_FREQ_L
    sta VOICE_\n\()_PHASE_L
    lda VOICE_\n\()_PHASE_H
    adc VOICE_\n\()_FREQ_H
    sta VOICE_\n\()_PHASE_H
    tay                         ; Y = phase high byte

    ; Get wavetable sample
    lda VOICE_\n\()_WAVEPTR_L
    sta 0x7C
    lda VOICE_\n\()_WAVEPTR_H  
    sta 0x7D
    lda (0x7C), y
    
    ; Volume table lookup
    ldx VOICE_\n\()_VOLPTR_L
    stx 0x7C
    ldx VOICE_\n\()_VOLPTR_H
    stx 0x7D
    tay
    lda (0x7C), y
    
    ; Convert to signed, shift, convert back
    sec
    sbc #0x80
    ldx VOICE_\n\()_SHIFT
    beq no_shift_\n
shift_\n:
    cmp #0x80
    ror a
    dex
    bne shift_\n
no_shift_\n:
    clc
    adc #0x80
    
    ; Divide by 8 and mix
    lsr a
    lsr a
    lsr a
    clc
    adc MIX_ACCUMULATOR
    sta MIX_ACCUMULATOR
    jmp skip_voice_\n
silent_voice_\n:
    ; Silent voice contributes 0x10 (centered)
    lda MIX_ACCUMULATOR
    clc
    adc #0x10
    sta MIX_ACCUMULATOR
skip_voice_\n:
.endm

audio_irq:
    ; Initialize accumulator to 0
    lda #0x00
    sta MIX_ACCUMULATOR        ; Use as running mix accumulator

    ; Process all 7 voices using macro
    PROCESS_VOICE 0
    PROCESS_VOICE 1
    PROCESS_VOICE 2
    PROCESS_VOICE 3
    PROCESS_VOICE 4
    PROCESS_VOICE 5

    ; Output final mixed sample
    ; Silent voices contribute 0x10 (centered)
    ; Active voices at center also contribute 0x10
    ; With 6 voices: 6 * 0x10 = 0x60, plus 0x20 offset = 0x80 (centered)
    lda MIX_ACCUMULATOR
    clc
    adc #0x20
    sta 0x8040

    rti                    ; return from interrupt


; Simple main function that just waits
.section .text
.global _start
_start:
    sei                    ; disable interrupts during setup
    cld                    ; clear decimal mode
    
    ; Initialize stack pointer
    ldx #0xff
    txs
    
    ; Voice data is already initialized by .data.voices section
    ; (all phases, freqs, volumes = 0, waveptrs = instrument1_table)
    
    ; Enable interrupts
    cli
    
main_loop:
    wai                    ; wait for interrupt
    jmp main_loop          ; loop forever

; Initialize voice data with default wavetable pointers
.section .data.voices
.rept 7
    .byte 0, 0              ; phase_l, phase_h
    .byte 0, 0              ; freq_l, freq_h
    .word instrument1_table ; waveptr (little-endian)
    .word vol_table_3       ; volptr (little-endian, points to 62.5% table)
    .byte 4                 ; shift (4 = silence)
.endr

; Vector table (must be at $FFFA-$FFFF)
.section .vector_table, "a"
    .word audio_irq        ; NMI vector ($FFFA-$FFFB)
    .word _start           ; RESET vector ($FFFC-$FFFD)
    .word audio_irq        ; IRQ/BRK vector ($FFFE-$FFFF)

