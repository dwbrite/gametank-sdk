.global audio_irq
.extern vol_table
.section .text

; Memory map (4KB = $0000 - $0FFF) as a Markdown table (matches linker.ld):
; | Range        | Size         | Purpose                  | Notes                                          |
; |--------------|--------------|---------------------------|------------------------------------------------|
; | $0000-$0040  | $0041 (65)   | Reserved                  | header byte + low reserved region              |
; | $0041-$00FF  | $00C0 (192)  | Zero page (voices + vars) | VOICE_BASE = $0041; 7 voices × 7 bytes + temps |
; | $0100-$01FF  | $0100 (256)  | CPU stack                | hardware stack                                 |
; | $0200-$02FF  | $0100 (256)  | Volume table              | vol_table (half-amplitude sine)                |
; | $0300-$0CFF  | $0A00 (2560) | Wavetables (10 × 256)     | placed by linker.ld's WAVE region              |
; | $0D00-$0FF9  | ~$02FA (762) | Code / other data (ARAM)  | Remaining program space before the vector table |
; | $0FFA-$0FFF  | 6            | Vector table              | NMI/RESET/IRQ vectors                          |
;
; Addresses are little-endian, and ranges are inclusive.

; Define the base address for the voices (zero page)
.set VOICE_BASE, 0x0041   ; zero-page base for voice control registers
.set VOICE_SIZE, 7        ; Each voice occupies 7 bytes
.set VOICE_COUNT, 7
.set VOICE_END, (VOICE_BASE + (VOICE_SIZE * VOICE_COUNT) - 1)  ; last byte used by voices (0x0071)

; Temporary ZP storage for IRQ
.set TEMP_SAMPLE, 0x0079  ; temporary storage for scaled sample
.set TEMP_RESULT1, 0x007a ; temporary storage for first vol_table result
.set TEMP_RESULT2, 0x007b ; temporary storage for second vol_table result

; Noise instrument state
.set LFSR_BASE, 0x007e       ; 7 voices x 2 bytes (low, high) of 15-bit LFSR state
.set LFSR_SIZE, 2

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

; Define 7 voices
DEFINE_VOICE 0
DEFINE_VOICE 1
DEFINE_VOICE 2
DEFINE_VOICE 3
DEFINE_VOICE 4
DEFINE_VOICE 5
DEFINE_VOICE 6

; A voice's WAVEPTR is set to one of these two values (both share the same
; high byte, `NOISE_SENTINEL_HI`, which real wavetable addresses never use)
; to select the built-in Noise instrument instead of sampling a wavetable.
; The low byte's bit 0 selects the noise "color" (see lfsr_clock below).
.set NOISE_SENTINEL_HI, 0xff

; Sample the next wavetable index (or clock the noise generator) and apply
; volume scaling to this tick's output sample
.macro PROCESS_VOICE_CORE n
    ; A voice plays Noise instead of a wavetable when its WAVEPTR high byte
    ; is the reserved NOISE_SENTINEL_HI value
    lda VOICE_\n\()_WAVEPTR_H
    bpl voice_\n\()_wavetable

    ; --- Noise sampling path: clock this voice's LFSR on phase overflow,
    ; i.e. at a rate set by the note's frequency, same as a wavetable voice's
    ; table index advances - so note/frequency controls perceived pitch here too.
    clc
    lda VOICE_\n\()_PHASE_L
    adc VOICE_\n\()_FREQ_L
    sta VOICE_\n\()_PHASE_L
    lda VOICE_\n\()_PHASE_H
    adc VOICE_\n\()_FREQ_H
    sta VOICE_\n\()_PHASE_H  ; carry here is this tick's phase-overflow, fresh off the ADC
    ldx #(\n * LFSR_SIZE)   ; this voice's LFSR state offset
    lda VOICE_\n\()_WAVEPTR_L
    and #1                  ; A = noise mode (0/1), passed directly to lfsr_clock in A
    bcc voice_\n\()_noise_sample
    jsr lfsr_clock
voice_\n\()_noise_sample:
    lda LFSR_BASE, x        ; use the LFSR's low byte as this tick's sample
    jmp voice_\n\()_scaled

voice_\n\()_wavetable:
    ; --- Wavetable sampling path ---
    ; Advance phase accumulator by FREQ (16-bit addition)
    clc
    lda VOICE_\n\()_PHASE_L
    adc VOICE_\n\()_FREQ_L
    sta VOICE_\n\()_PHASE_L
    lda VOICE_\n\()_PHASE_H
    adc VOICE_\n\()_FREQ_H
    sta VOICE_\n\()_PHASE_H
    tay                     ; Y = phase high byte (wavetable index)
    lda (VOICE_\n\()_WAVEPTR_L), y ; indirect indexed read from voice's wavetable

voice_\n\()_scaled:
    ; Scale to 7-bit for volume scaling
    lsr a                  ; divide by 2
    sta TEMP_RESULT1       ; save scaled sample (s)

    ; index1 = s - volume
    sec
    sbc VOICE_\n\()_VOLUME
    sta TEMP_RESULT2       ; save index1
    tax
    lda vol_table, x       ; vol_table[s - v]
    tay                    ; cache in Y... cheaper than a second zero-page temp

    ; index2 = s + v, derived as 2s - index1
    ; No second volume read means no opportunity for the value to change under our noses
    lda TEMP_RESULT1       ; restore s
    asl a                  ; 2*s
    sec
    sbc TEMP_RESULT2       ; index2 = 2s - index1 = s + v
    tax
    lda vol_table, x       ; vol_table[s + v]
    sta TEMP_RESULT1       ; stash (s no longer needed)

    tya                    ; A = vol_table[s - v]
    sec
    sbc TEMP_RESULT1        ; A = vol_table[s-v] - vol_table[s+v]
.endm

; Clock one voice's 15-bit Galois LFSR by one step, producing the next noise
; sample in its low byte.
; X = the voice's LFSR_BASE offset
; A picks the tap position
;   0 = taps bit 0 ^ bit 1,  period 32767 samples (adjacent bits: computed
;       branch-free below via a shift + EOR, cheaper than mode 1's tap)
;   1 = taps bit 0 ^ bit 6, period 93 samples
lfsr_clock:
    bne lfsr_tap_mode1
    ; --- Mode 0: bit0 ^ bit1 are adjacent, so shifting the register right
    ; by one lines bit1 up under bit0; EORing against the unshifted register
    ; then shifting bit0 out into carry gives the feedback bit with no
    ; branching at all.
    lda LFSR_BASE, x
    lsr a
    eor LFSR_BASE, x
    lsr a                    ; carry = feedback bit (bit 0 of the EOR result); A discarded
    jmp lfsr_shift
lfsr_tap_mode1:
    ; --- Mode 1: bit0 ^ bit6 aren't adjacent, so there's no cheap shift
    ; trick; fall back to an explicit tap-bit check.
    lda LFSR_BASE, x
    and #0x40               ; tap = bit 6 (short/metallic)
    beq lfsr_feedback_bit0  ; tap bit was 0 -> feedback = bit 0 as-is
    lda LFSR_BASE, x
    and #0x01
    eor #0x01               ; feedback = NOT bit 0
    lsr a                    ; carry = feedback bit; A discarded
    jmp lfsr_shift
lfsr_feedback_bit0:
    lda LFSR_BASE, x
    and #0x01               ; feedback = bit 0
    lsr a                    ; carry = feedback bit; A discarded

    ; Shift the 15-bit register right by one, folding `feedback` (already
    ; in carry) straight into the high byte's new top bit via ROR - no
    ; scratch byte or lookup table needed. The high byte's old bit 0 then
    ; rides carry into the low byte's new top bit the same way.
lfsr_shift:
    lda LFSR_BASE+1, x
    ror a                     ; new top bit = feedback (from carry); carry = old high-byte bit 0
    sta LFSR_BASE+1, x

    lda LFSR_BASE, x
    ror a                     ; carry (old high-byte bit 0) becomes the new bit 7
    sta LFSR_BASE, x
    rts

.section .text

audio_irq:
    PROCESS_VOICE_CORE 0
    clc
    adc #0x80              ; fold the center/silence value into voice 0's mix
    sta TEMP_SAMPLE

    PROCESS_VOICE_CORE 1
    clc
    adc TEMP_SAMPLE
    sta TEMP_SAMPLE

    PROCESS_VOICE_CORE 2
    clc
    adc TEMP_SAMPLE
    sta TEMP_SAMPLE

    PROCESS_VOICE_CORE 3
    clc
    adc TEMP_SAMPLE
    sta TEMP_SAMPLE

    PROCESS_VOICE_CORE 4
    clc
    adc TEMP_SAMPLE
    sta TEMP_SAMPLE

    PROCESS_VOICE_CORE 5
    clc
    adc TEMP_SAMPLE
    sta TEMP_SAMPLE

    PROCESS_VOICE_CORE 6
    clc
    adc TEMP_SAMPLE
    ; Output final mixed sample directly to the DAC instead
    ; of doing a round trip through the TEMP_SAMPLE
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

    ; Seed every voice's noise LFSR to a fixed nonzero value. Never
    ; reseeded after this (see lfsr_clock) so it free-runs continuously;
    ; each voice gets a different seed so multiple noise voices playing
    ; the same note don't produce identical, reinforcing noise.
    lda #0xe1
    sta LFSR_BASE+0
    lda #0x2c
    sta LFSR_BASE+1
    lda #0x3d
    sta LFSR_BASE+2
    lda #0x15
    sta LFSR_BASE+3
    lda #0x7a
    sta LFSR_BASE+4
    lda #0x6e
    sta LFSR_BASE+5
    lda #0xc4
    sta LFSR_BASE+6
    lda #0x09
    sta LFSR_BASE+7
    lda #0x29
    sta LFSR_BASE+8
    lda #0x41
    sta LFSR_BASE+9
    lda #0x85
    sta LFSR_BASE+10
    lda #0x33
    sta LFSR_BASE+11
    lda #0x5b
    sta LFSR_BASE+12
    lda #0x1a
    sta LFSR_BASE+13

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
