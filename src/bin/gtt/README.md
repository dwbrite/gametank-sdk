# gt-tracker :: gtt

A music editor/tracker for the [GameTank](https://gametank.zone/).

![gt-tracker screenshot](./screenshot1.png)

<!-- Run `npx doctoc README.md to re-generate the TOC` -->
<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Getting started](#getting-started)
- [Anatomy](#anatomy)
- [Pattern editor](#pattern-editor)
- [Control deck](#control-deck)
- [Command palette](#command-palette)
- [Instrument editor](#instrument-editor)
- [Tuning editor](#tuning-editor)
- [Using your tracks](#using-your-tracks)
- [Track descriptor reference](#track-descriptor-reference)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Getting started

Install `gtgo` and `gtt` via Rust: `cargo install gtgo` and `cargo install gtt`

You can then launch `gtgo` from the command line and select the tracker from the menu option.

You can also run the tracker command directly: `gtt [input-file]`

If a file name not specified, you'll be prompted to browse and select a file.


## Anatomy

The screen is organized into a control deck on top and the pattern editor on bottom.
The layout and key bindings are designed to feel familiar to other trackers while also being intuitive to new users.
Use the arrow keys or mouse pointer to jump around the tracker.
Using the tab key toggles focus between the control deck and the pattern editor.


## Pattern editor

Use the arrow keys, page-up/page-down, and the mouse cursor to navigate between rows and columns.
At the top of the pattern editor you'll see:

```
BEAT  SEQ  ch0 v :↗↘   ch1 v :↗↘   ...
```

**BEATS** are represented by the number to the left of each row.
The total set of beats on screen represents a **pattern**.
The number of beats in the pattern can be adjusted on the control deck or via the command palette `:beats [1-255]`.

**SEQ** is the leftmost lane. Sequence Commands are track-wide.
They affect all channels unless a channel explicitly overrides the command.
They are processed at the start of a beat before any voices.
Use Enter/Up/Down/Esc on a SEQ cell to select a sequencer command.

| ID | Label           | Parameters                                     | Description           |
| -- | -----           | ----------                                     | -----------           |
| 0  | (no effect)     |                                                |                       |
| 1  | `[S]top`        | (none)                                         | Ends track playback   |
| 2  | `[T]empo`       | x = tempo                                      | Updates the track BPM |
| 3  | `F[x]Speed`     | x (00-1F) = number of ticks per beat           | How many ticks per beat channel effects advance. (Higher is faster.) |
| 4  | `[#] FlowCount` | x (00-FF) = value for "count" register         | Replace the value of the count register. Any number greater than 0 will trigger the conditional "CountJump". | 
| 5  | `Count[j]ump`   | x (00-FF) = pattern idx, y (00-FF) = beat idx  | When the count register is greater than 0, jump to the given pattern+beat, then decrement the count register by 1. Essential for looping a section of track a finite number of times. When the register is 0, this command does nothing. |
| 6  | `[J]ump`        | x (00-FF) = pattern idx, y (00-FF) = beat idx  | Unconditionally jump to a given pattern and beat. Useful for jumping to the next pattern or looping a track indefinitely. Note that jumps happen before the target beat is processed. |


**Notes**: select the column with the channel number (ie ch0) and use the top row of letters and numbers for note entry.
The default tuning is mapped similar to a piano roll. This is QWERTY-agnostic, but assuming your keyboard is QWERTY-based, the key binding would work like this:

```
 Note OFF
 ┊     C♯5 D♯5     F♯5 G♯5 A♯5     C♯6 D♯6     F♯6
 ┊   C5┊ D5┊ E5  F5┊ G5┊ A5┊ B5  C6┊ D6┊ E6  F6┊ G6
 ~   │ ┊ │ ┊ │   │ ┊ │ ┊ │ ┊ │   │ ┊ │ ┊ │   │ ┊ │
 ` 1 │ 2 │ 3 │ 4 │ 5 │ 6 │ 7 │ 8 │ 9 │ 0 │ - │ = │
     Q   W   E   R   T   Y   U   I   O   P   [   ]
```

See the [Tuning editor](#tuning-editor) for how to update these mappings.

**Backspace/Delete** key removes the note/command the cursor has selected.

**`/~** sets Note OFF. While an empty beat carries a note, an explicit Note OFF effectively ends the note by temporarily muting the channel.

**v** represents volume. Entered as a two-digit hexadecimal value from `00` to `3F` (0-63 decimal). Type 0-9/A-F to set digits or `-`/`=` to decrement/increment. The last set volume carries to the next note, even if a note OFF is set along the way.

**Fx** lets you apply effects per channel. Effects apply at the start of the beat before the note is processed.

| ID | Label       | Parameters                                               | Description                                    |
| -- | -----       | ----------                                               | -----------                                    |
| 0  | (no effect) |                                                          |                                                |
| 1  | Instrument  | x (0-A) = instrument index, y (0-1) = noise channel mode | Switch which instrument this channel is using. Instrument `A` is the virtual noise channel. Similar to the noise "percussion" of the NES, mode 0 has a slow hiss and mode 1 has more crash. |
| 2  | Arpeggio    | x (0-F) = how many steps up to the second note,<br>y (0-F) = optional, steps up for a third note. | The arpeggiation steps to its next note once per tick, and FxSpeed sets how many ticks occur per beat. So an FxSpeed of `05` yields 3 ticks/notes a beat. |
| 3  | PitchUp     | x (00-FF) = how many steps up to slide to                | Portamento that increments at a rate of FxSpeed. |
| 4  | PitchDown   | x (00-FF) = how many steps down to slide to              | Portamento that decrements at a rate of FxSpeed. |
| 5  | FadeIn      | x (00-3B) = how many ticks to hold each volume increment | Play note with a volume of 0 and raise volume up at a rate of `x` until reaching set volume or the beat ends. |
| 6  | FadeOut     | x (00-3B) = how many ticks to hold each volume increment | Play note at the volume level set for the beat and lower volume down to 0 at a rate of `x`. |
| 7  | Tremble     | x (00-3B) = how many ticks to hold a note off then on    | Hard tremolo. Rapidly play and mute a note. |

## Control deck

The control deck allows you to edit global track parameters, pattern-level parameters, as well as convenient buttons for common Command palette (`:command`) actions.

**BPM** is the global BPM.

**Beats** changes the length of the current pattern.

**Trans** transposes the key bindings up/down one unison. With the default tuning, this means incrementing up one octave from c5-g6 to c6-g7. SHIFT key will also shift the range up temporarily.

**FxSpeed** is the starting effects speed for the track (see **SEQ** commands for more details).

**Instruments** - there are 11 total instruments, which can be swapped out on any beat on any channel to provide a lot of versatility. The 11 instruments can be renamed from here, and their waveforms opened in the instrument editor from the `[⚙]` buttons.

**Tuning editor** opens the [tuning editor](#tuning-editor) for key assignment.

**New/Open** will allow you to open an existing file or create a new file (press `n`). Default name is `track.gtt` or `track{n}.gtt` to avoid overwriting. Be sure to save before opening another file.

**Save** writes the changes in memory to file.

**Quit** will prompt to save changes before exiting so that you don't accidentally close the program with unsaved changes.

**Export** saves the track, instruments, and tuning into a format consumable by the project. See [Using your tracks](#using-your-tracks) below.


## Command palette

This provides quick keyboard-driven actions. All the commands available on the control deck are available on the command palette, plus more.
When you start typing `:` your command input will start appearing at the very bottom of the screen.
Press Enter to execute or Esc to cancel the command input.

| command          | description                             |
| -------          | -----------                             |
| `:exp`/`:export` | export track                            |
| `:instrument`    | open instrument editor                  |
| `:m`             | toggle mute on the active voice channel |
| `:mute`          | mute the active voice channel           |
| `:unmute`        | unmute the active voice channel         |
| `:q`/`:quit`     | quit the application                    |
| `:w`/`:write`    | save changes to file                    |

TODO: lots more commands


## Instrument editor

![gt-tracker instrument editor screenshot](./screenshot2.png)

An instrument is a 256-sample waveform, 256 single-byte data points.
This is represented by 256 bars (00-FF) with adjustable height.
You can adjust the data points by dragging the mouse cursor like a brush stroke.
For fine tuning the values, you also have the arrow keys.
Each byte is an unsigned 8-bit PCM sample:

| value         | description                  |
| ------------- | ---------------------------- |
| `0x80`        | Zero / silence (DC midpoint) |
| `0x81`-`0xFF` | Positive half-cycle          |
| `0x00`-`0x7F` | Negative half-cycle          |

For exporting and importing, you can use the `[Export]` button on the Control Deck to export out the raw waveform from a given track, then within the instrument editor use the `[Import]` button to slot in an instrument waveform you already made.

Simply click `[Cancel]` or `[Save]` when done.

## Tuning editor

This window serves two purposes.
One, it defines the music notation mapping used for pattern editing, in other words the tuning system used as the basis for your music.
Two, it allows you to create key bindings to those notes to create a mapping ergonomic for your setup.
The default tuning and key bindings outlined above will be familiar to users of DefleMask and MilkyTracker.

You can import [Scala scale files (.scl)](https://www.huygens-fokker.org/scala/scl_format.html) if you want to get crazy and make some [microtonal tracks](https://youtu.be/QBC8Bjxu5y8).
Imported scales are populated to my best guess of the usable frequency range of the APU firmware, 7Hz-4200Hz.

The tuning lives inside the track file and exports with it.

To adjust the key bindings, click on the row and press a key to see it appear under the "Key assign" column.
The key bindings lives in your application config:

| Linux | macOS | Windows |
| ----- | ----- | ------- |
| `$XDG_CONFIG_HOME/gtt/default-config.toml` or `$HOME/.config/gtt/default-config.toml` | `$HOME/Library/Application Support/gtt/default-config.toml` | `{FOLDERID_RoamingAppData}/gtt/config/default-config.toml` |

Just click `[Cancel]` or `[Save]` when done.


## Using your tracks

Clicking **Export** creates a `<name>-export/` folder next to your gt-tracker file:

```
mycooltrack-export/
  instruments/         instrument raw waveform files
  mycooltrack.bin      pattern data, ready to embed
```

Follow the instructions for your SDK on how to incorporate the track data into your project.

- [C SDK](https://github.com/clydeshaffer/gametank_sdk)
- [Rust SDK](https://github.com/dwbrite/gametank-sdk)


## Track descriptor reference

The exported `<name>-export/<name>.bin` is made of three parts:

**1. Header** fixed at 5 bytes, starting at offset 0.

| offset | type  | field           | description                                     |
| ------ | ----  | -----           | -----------                                     |
| 0      | `u16` | `bpm`           | Base tempo, beats per minute                    |
| 2      | `u16` | `speed`         | FxSpeed. Track-wide number of ticks per effect  |
| 4      | `u8`  | `pattern_count` | Number of unique patterns                       |

**2. Patterns table** immediately follows the header at byte offset 5. It is a variable-sized array where each entry tells the location of the pattern data block for the given pattern index.

| index                | type  | value                                                              |
| -------------------- | ----  | -----                                                              |
| 0                    | `u16` | Offset from the start of the file to pattern 0's data block        |
| 1                    | `u16` | Offset from the start of the file to pattern 1's data block (i.e. pattern 0's offset + pattern 0's `pattern_size`) |
| ...                  | ...   | ...                                                                |
| `pattern_count - 1`  | `u16` | Offset from the start of the file to the last pattern's data block |

**3. Pattern data blocks**, one per table entry above. Located at the offset given by that entry. Each block is laid out as:

```
[0] beats: u8 (row count for this pattern)
per channel, each array sized to `beats` bytes:
  freq_lo[beats], freq_hi[beats]   16-bit phase increment; 0x0000 holds the previous note
  vol[beats]                       0-63, or 0xFF to hold the previous volume
  fx_id[beats]                     equal to the channel effect ID (see channel FX definitions above); 0 = no effect
  fx_x[beats], fx_y[beats]         raw effect parameters (scale-degree offsets for Arpeggio/PitchUp/PitchDown)
  fx_freq_x_lo/hi[beats]           baked target frequency for fx_x (Arpeggio's +x note, or PitchUp/PitchDown's slide target)
  fx_freq_y_lo/hi[beats]           baked +y scale-degree frequency (Arpeggio only)
  seq_cmd_type[beats]              equal to the sequence command ID (see SEQ definitions above); 0 = no command
  seq_cmd_value[beats]             raw command value (unused for Stop; pattern index for CountJump)
  seq_cmd_value2[beats]            second raw command value (only used for CountJump: target beat)
```
