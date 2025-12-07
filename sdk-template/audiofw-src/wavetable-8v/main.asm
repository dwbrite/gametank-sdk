.global audio_irq
.extern vol_table
.extern sine_table
.section .text

; Memory map (4KB = $0000 - $0FFF) as a Markdown table:
; | Range        | Size         | Purpose                        | Notes                                      |
; |--------------|--------------|--------------------------------|--------------------------------------------|
; | $0000-$0040  | $0100 (256)  | Zero Page (Reserved)           | Fast addressing; pointers & small vars     |
; | $0041-$0078  | $0038 (56)   | Voices (8 × 7 bytes)           | VOICE_BASE = $0042, VOICE_SIZE = 7        |
; | $0100-$01FF  | $0100 (256)  | CPU Stack                      | CPU stack                                  |
; | $0200-$03FF  | $0200 (512)  | Hardcoded wavetables (2 × 256) | WAVETABLE_BASE = $0200, WAVETABLE_SIZE = 256 |
; | $0400-$0BFF  | $0A00 (2560) | Wavetables (10 × 256)          | WAVETABLE_BASE = $0400, WAVETABLE_SIZE = 256 |
; | $0E00-$0FFF  | $0200 (512)  | Code / other data              | Remaining 1KB for code/data/ROM layout     |
;
; Addresses are little-endian, and ranges are inclusive.

; Define the base address for the voices (zero page)
.set VOICE_BASE, 0x0041   ; zero-page base for voice control registers
.set VOICE_SIZE, 7        ; Each voice occupies 7 bytes
.set VOICE_COUNT, 8
.set VOICE_END, (VOICE_BASE + (VOICE_SIZE * VOICE_COUNT) - 1)  ; last byte used by voices (0x0078)

; Temporary ZP storage for IRQ
.set TEMP_SAMPLE, 0x0079  ; temporary storage for scaled sample
.set TEMP_RESULT1, 0x007a ; temporary storage for first vol_table result
.set TEMP_RESULT2, 0x007b ; temporary storage for second vol_table result

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
    .set VOICE_\voice_index\()_VOLUME, (VOICE_\voice_index\()_BASE + 6)
.endm

; Macro to define a WAVETABLE_n_BASE equate for a given index
.macro DEFINE_WAVETABLE idx
    .set WAVETABLE_\idx\()_BASE, (WAVETABLE_BASE + (WAVETABLE_SIZE * \idx))
.endm

; Define 8 voices
DEFINE_VOICE 0
DEFINE_VOICE 1
DEFINE_VOICE 2
DEFINE_VOICE 3
DEFINE_VOICE 4
DEFINE_VOICE 5
DEFINE_VOICE 6
DEFINE_VOICE 7

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
    ; Add FREQ to PHASE (16-bit addition)
    clc
    lda VOICE_\n\()_PHASE_L
    adc VOICE_\n\()_FREQ_L
    sta VOICE_\n\()_PHASE_L
    lda VOICE_\n\()_PHASE_H
    adc VOICE_\n\()_FREQ_H
    sta VOICE_\n\()_PHASE_H

    ; Get wavetable sample using phase_high as index
    tax                    ; X = phase high byte
    lda sine_table, x      ; lookup sample (TODO: use per-voice wavetable pointer)

    ; Scale to 7-bit for volume scaling
    lsr a                  ; divide by 2
    sta TEMP_RESULT1       ; save scaled sample (s)

    ; Compute vol_table[s - volume]
    sec
    sbc VOICE_\n\()_VOLUME
    tax
    lda vol_table, x
    sta TEMP_RESULT2       ; save vol_table[s - v]

    ; Compute vol_table[s + volume]
    lda TEMP_RESULT1       ; restore scaled sample
    clc
    adc VOICE_\n\()_VOLUME
    tax
    lda vol_table, x       ; A = vol_table[s + v]

    ; Result = vol_table[s-v] - vol_table[s+v] (signed voice output)
    sta TEMP_RESULT1       ; temp save s+v result
    lda TEMP_RESULT2       ; load s-v result
    sec
    sbc TEMP_RESULT1       ; A = vol_table[s-v] - vol_table[s+v]

    ; Mix into accumulator (add to running total)
    clc
    adc TEMP_SAMPLE
    sta TEMP_SAMPLE
.endm

audio_irq:
    ; Initialize accumulator with center value (silence = 0x80)
    ; We'll mix all voices relative to center
    lda #0x80
    sta TEMP_SAMPLE        ; Use as running mix accumulator

    ; Process all 8 voices using macro
    PROCESS_VOICE 0
    PROCESS_VOICE 1
    PROCESS_VOICE 2
    PROCESS_VOICE 3
    PROCESS_VOICE 4
    PROCESS_VOICE 5
    PROCESS_VOICE 6
    PROCESS_VOICE 7

    ; Output final mixed sample
    lda TEMP_SAMPLE
    sta 0x8040

    rti                    ; return from interrupt


; Macro to initialize a voice (phase=0, freq=0, waveptr=sine_table, volume=0)
.macro INIT_VOICE n
    lda #0
    sta VOICE_\n\()_VOLUME
    ; Set wavetable pointer to sine_table
    lda #<sine_table
    sta VOICE_\n\()_WAVEPTR_L
    lda #>sine_table
    sta VOICE_\n\()_WAVEPTR_H
.endm

; Simple main function that just waits
.section .text
.global _start
_start:
    sei                    ; disable interrupts during setup
    cld                    ; clear decimal mode
    
    ; Initialize stack pointer
    ldx #0xff
    txs
    
    ; Initialize all 8 voices
    INIT_VOICE 0
    INIT_VOICE 1
    INIT_VOICE 2
    INIT_VOICE 3
    INIT_VOICE 4
    INIT_VOICE 5
    INIT_VOICE 6
    INIT_VOICE 7
    
    ; Enable interrupts
    cli
    
main_loop:
    wai                    ; wait for interrupt
    jmp main_loop          ; loop forever

; Vector table (must be at $FFFA-$FFFF)
.section .vector_table, "a"
    .word audio_irq        ; NMI vector ($FFFA-$FFFB)
    .word _start           ; RESET vector ($FFFC-$FFFD)
    .word audio_irq        ; IRQ/BRK vector ($FFFE-$FFFF)
