# Bouncing Balls

A simple GameTank demo showing colorful balls bouncing around the screen.

## Features Demonstrated

- **Blitter Operations**: Using `draw_square` and `draw_sprite` for hardware-accelerated graphics
- **Double Buffering**: Flipping between framebuffers for tear-free animation
- **Bank Switching**: Organizing code across ROM banks (125, 126)
- **Audio Initialization**: Setting up the audio coprocessor with wavetable firmware
- **Game Loop Pattern**: VBlank synchronization with async blitter usage

## Building

```bash
cd examples/bouncing-balls
gtrom build
```

This produces `game.gtr` which can be run in the emulator:

```bash
gtrom run
# or
gte game.gtr
```

## Project Structure

```
bouncing-balls/
├── rom/                    # Main ROM project
│   ├── .cargo/config.toml # Linker flags for bare-metal 6502
│   ├── Cargo.toml         # Rust package config with audio feature
│   ├── build.rs           # Linker script generation
│   └── src/
│       ├── main.rs        # Game logic
│       ├── boot.rs        # Startup code and interrupt handlers
│       └── sdk/           # Hardware abstraction modules
├── audiofw/               # Pre-built audio firmware binaries
└── game.gtr               # Built ROM (after gtrom build)
```

## Created With

```bash
gtrom init examples/bouncing-balls
```
