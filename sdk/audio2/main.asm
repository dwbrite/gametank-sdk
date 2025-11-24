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

audio_irq:
    ; Clear output buffer for mixing
    lda #0x80              ; 2 cycles - center value (silence)
    ;sta 0x8040              ; 4 cycles - clear output buffer
    ; Cumulative: 6 cycles

    ; Add FREQ to PHASE for voice 0 (16-bit addition with carry)
    clc                    ; 2 cycles - clear carry
    lda VOICE_0_PHASE_L    ; 3 cycles - load phase low byte
    adc VOICE_0_FREQ_L     ; 3 cycles - add freq low byte (carry propagates)
    sta VOICE_0_PHASE_L    ; 3 cycles - store phase low byte
    lda VOICE_0_PHASE_H    ; 3 cycles - load phase high byte
    adc VOICE_0_FREQ_H     ; 3 cycles - add freq high byte with carry
    sta VOICE_0_PHASE_H    ; 3 cycles - store phase high byte
    ; Cumulative: 20 cycles

    ; Get wavetable sample using phase_high as index
    ldx VOICE_0_PHASE_H    ; 3 cycles - load phase high byte to X
    lda sine_table, x      ; 4 cycles - lookup sine sample
    ; Cumulative: 27 cycles
    
    ; Scale to 7-bit for volume scaling
    lsr a                  ; 2 cycles - divide by 2 (shift right)
    sta TEMP_SAMPLE        ; 3 cycles - save scaled sample
    ; 32 cycles
    
    ; Compute s - volume first
;    lda TEMP_SAMPLE        ; 3 cycles - load scaled sample
    sec                    ; 2 cycles - set carry for subtraction
    sbc VOICE_0_VOLUME     ; 3 cycles - A = s - volume
    tax                    ; 2 cycles - A -> X
    lda vol_table, x       ; 4 cycles - A = vol_table[X]
    sta TEMP_RESULT1       ; 3 cycles - save first result of vol_tab[sample - volume]
    ; Cumulative: 46 cycles
    
    ; Compute s + volume
    lda TEMP_SAMPLE        ; 3 cycles - restore scaled sample
    clc                    ; 2 cycles - clear carry
    adc VOICE_0_VOLUME     ; 3 cycles - A = s + volume
    tax                    ; 2 cycles - X for second lookup
    lda vol_table, x       ; 4 cycles - A = vol_table[s + volume]
    ; Cumulative: 60 cycles
    
    ; Subtract: vol_table[s-v] - vol_table[s+v]
    ; First result (s-v) is in TEMP_RESULT1, second (s+v) is in A
    sta TEMP_RESULT2       ; 3 cycles - save second result (s+v)
    lda TEMP_RESULT1       ; 3 cycles - load first result (s-v)
    sec                    ; 2 cycles - set carry for subtraction
    sbc TEMP_RESULT2       ; 3 cycles - A = vol_table[s-v] - vol_table[s+v]
    ;tax                    ; 2 cycles
    ; Cumulative: 71 cycles
    
    ; Store to output buffer at $8040
    ;sta 0x40              ; 4 cycles - store voice 0 output
    ; 

    adc #0x80


    ; remove me
    sta 0x8040

    ; Cumulative: 75 cycles
    
    ; TODO: Process voices 1-7 and add to $8040
    
    rti                    ; 6 cycles - return from interrupt
    ; Total: 81 cycles for voice 0



; Simple main function that just waits
.section .text
.global _start
_start:
    sei                    ; disable interrupts during setup
    cld                    ; clear decimal mode
    
    ; Initialize stack pointer
    ldx #0xff
    txs
    
    ; Initialize voice 0
    ; Set phase to 0
    lda #0
    sta VOICE_0_PHASE_L
    sta VOICE_0_PHASE_H
    
    ; Set frequency for 400Hz at 24kHz sample rate
    ; Phase increment = (65536 * 400) / 14000 = ~1872 = 0x0750
    lda #0x50
    sta VOICE_0_FREQ_L
    lda #0x07
    sta VOICE_0_FREQ_H
    
    ; Set wavetable pointer to sine_table (0x0400)
    lda #0x00
    sta VOICE_0_WAVEPTR_L
    lda #0x04
    sta VOICE_0_WAVEPTR_H

    lda #0x40
    sta VOICE_0_VOLUME
    
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
