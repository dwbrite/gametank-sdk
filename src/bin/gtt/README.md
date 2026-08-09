# gt-tracker :: gtt

A music editor/tracker for the [GameTank](https://gametank.zone/).

![gt-tracker screenshot](./screenshot.png)

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

Use the arrow keys, page-up/page-down, and the mouse cursor to navigate between row and columns.
At the top of the pattern editor you'll see:

```
BEAT  SEQ  ch0 v ::↗↘   ch1 v ::↗↘   ...
```

**BEATS** are represented by the number to the left of each row.
The total set of beats on screen represents a **pattern**.
The number of beats in the pattern can be adjusted on the control deck or via the command palette `:beats [1-255]`.

Use the top row of keys and numbers for note entry.
The default tuning is mapped similar to a piano roll.
This is QWERTY-agnostic, but assuming your keyboard is QWERTY-based, the key binding would work like this:

```
 NOTE OFF
 ┊     C♯5 D♯5     F♯5 G♯5 A♯5     C♯6 D♯6     F#6
 ┊   C5┊ D5┊ E5  F5┊ G5┊ A5┊ B5  C6┊ D6┊ E6  F6┊ G6
 ~   │ ┊ │ ┊ │   │ ┊ │ ┊ │ ┊ │   │ ┊ │ ┊ │   │ ┊ │
 ` 1 │ 2 │ 3 │ 4 │ 5 │ 6 │ 7 │ 8 │ 9 │ 0 │ - │ = │
     Q   W   E   R   T   Y   U   I   O   P   [   ]
```

See the [Tuning editor](#tuning-editor) for how to update these mappings.

**Delete** key removes the note/command the cursor has selected.


## Control deck

The control deck allows you to edit global track parameters, pattern-level parameters, as well as convenient buttons for common Command palette (`:command`) actions.

**BPM** is the global BPM.

**Beats** changes the length of the current pattern.

**Trans** transposes the key bindings up/down one unison. With the default tuning, this means incrementing up one octave from c5-g6 to c6-g7. SHIFT key will also shift the range up temporarily.

**Rate** sets the sample rate for the track. This does not carry over into the export data. A custom sample rate can be applied at runtime via the SDKs.

**Instruments** - there are 11 total instruments, which can be swapped out on any beat on any channel to provide a lot of versatility. The 11 instruments can be renamed from here, and their waveforms opened in the instrument editor from the `[⚙]` buttons.

**Tuning editor** opens the [tuning editor](#turning-editor) for key assignment.

**New/Open** will allow you to open an existing file or create a new file (press `n`). Default name is `track.gtt` or `track{n}.gtt` to avoid overwriting. Be sure to save before opening another file.

**Save** writes the changes in memory to file.

**Quit** will prompt to save changes before exiting so that you don't accidentally close the program with unsaved changes.

**Export** save the track, instruments, and tuning into a format consumable by the project. See [Using your tracks](#using-your-tracks) below.


## Command palette

This provides quick keyboard-driven actions. All the commands available on the control deck are available on the command palette, plus more.
When you start typing `:` your command input will start appearing at the very bottom of the screen.
Press Enter to execute or Esc to cancel the command input.

| command       | description            |
| ------------- | ---------------------- |
| `:q`/`:quit`  | quit the application   |
| `:w`/`:write` | save changes to file   |

TODO: lots more commands


## Instrument editor

An instrument is a 256-byte waveform, 8 bytes of 8-byte data points.
This is represented by 256 bars (00-FF) with adjustable height.
You can adjust the data points by dragging the mouse cursor like a brush stroke.
For fine tuning the values, you also have the arrow keys.
Simply click `[cancel]` or `[save]` when done.

Each byte is an unsigned 8-bit PCM sample:

| value         | description                  |
| ------------- | ---------------------------- |
| `0x80`        | Zero / silence (DC midpoint) |
| `0x81`-`0xFF` | Positive half-cycle          |
| `0x00`-`0x7F` | Negative half-cycle          |

## Tuning editor

13983 / 65536 = ~0.2134

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

Just click `[cancel]` or `[save]` when done.


## Using your tracks

Clicking **Export** creates a `<name>-export/` folder next to your gt-tracker file:

```
mysong-export/
  instruments/         instrument raw waveform files
  wave.asm             assembly to import instruments
  mysong.asm           pattern data, ready for the GameTank linker
```

Follow the instructions for your SDK on how to incorporate the track data into your project.

- C SDK: TODO
- [Rust SDK](https://github.com/dwbrite/gametank-sdk)


## Track descriptor reference

The `<name>_track` symbol is a 10-byte binary descriptor:

| offset | type  | field           | description                                  |
| ------ | ----- | --------------- | -------------------------------------------- |
| 0      | `u16` | `bpm`           | Base tempo in beats per minute               |
| 2      | `u8`  | `pattern_count` | Number of unique patterns                    |
| 3      | `u8`  | `sequence_len`  | Number of entries in the playback sequence   |
| 4      | `u16` | `sequence`      | Pointer to `u8[]` of pattern indices         |
| 6      | `u16` | `patterns`      | Pointer to `u16[]` of pattern data pointers  |
| 8      | `u16` | `events`        | Pointer to `u16[]` of event list pointers    |

A `freq_inc` of `0x0000` holds the previous note.
A volume of `0xFF` holds the previous volume.

Event lists use the format `[count: u8, (beat: u8, type: u8, value: u8) * count]`.

Events:

| value  | type   | description           |
| ------ | ------ | --------------------- |
| `0x00` | Stop   | end pattern early     |
| `0x01` | Tempo  | set BPM to `value`    |
