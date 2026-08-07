This crate is a cycle-accurate simulator for the WDC W65C02S, the most
advanced direct descendent of that catalyst of the home computer
revolution, the 6502.

This crate accurately simulates all bus signals of the W65C02S except RDY,
SOB, and BE, which can all be simulated by outside code. It is written in
such a way that the unused bus logic usually gets optimized out. Make sure
LTO is enabled in your `Cargo.toml` for a tremendous speedup:

```toml
[profile.release]
lto = true
```

This crate does not depend on any other libraries, including the standard
library.

The W65C02S instruction set includes the original NMOS 6502 instructions,
the additional instructions supported by all CMOS 6502s, the "Rockwell bit
extensions" (`BBRx`/`BBSx`/`RMBx`/`SMBx`), and the `WAI` and `STP`
instructions.

The accuracy of this simulation has been tested on the [`65test` test
suite](https://github.com/SolraBizna/65test), which contains over 4500
tests. In every single test, the simulator's bus traffic is *exactly* the
same as the real hardware—even down to the timing of IRQ and NMI signals.
This means that this simulator is suitable for prototyping and simulation
of real systems using the W65C02S processor, including systems based on the
W65C134S MCU.

To use it, you will need an instance of `W65C02S` and an implementation of
`System`. `W65C02S` simulates the CPU; `System` must simulate the hardware
attached to the bus (memory, IO devices, et cetera).

```rust
use w65c02s::*;

pub fn main() {
    let mut system = HelloWorldSystem::new();
    let mut cpu = W65C02S::new();
    while cpu.get_state() != State::Stopped { cpu.step(&mut system); }
}

/// A simple system with 64K of RAM, along with an output-only "serial
/// port" mapped to $0000.
struct HelloWorldSystem {
    ram: [u8; 65536],
}

impl HelloWorldSystem {
    pub fn new() -> HelloWorldSystem {
        // initialize RAM with all 0xFFs
        let mut ram = [0xFF; 65536];
        // initialize the message
        ram[0x0001..0x000F].copy_from_slice(b"Hello World!\n\0");
        // initialize the program
        ram[0x0200..0x020C].copy_from_slice(&[
            op::LDX_IMM, 0,     //   LDX #0
                                // loop:
            op::LDA_ZPX, 1,     //   LDA $01, X
            op::BNE, 1,         //   BNE +
            op::STP,            //   STP
            op::STA_ZP, 0,      // + STA $00
            op::INC_X,          //   INX
            op::BRA, 0xF6,      //   BRA loop
        ]);
        // initialize the reset vector to point to $0200
        ram[0xFFFC..0xFFFE].copy_from_slice(&[0x00, 0x02]);
        HelloWorldSystem { ram }
    }
}

impl System for HelloWorldSystem {
    fn read(&mut self, _cpu: &mut W65C02S, addr: u16) -> u8 {
        // all reads return RAM values directly
        self.ram[addr as usize]
    }
    fn write(&mut self, _cpu: &mut W65C02S, addr: u16, value: u8) {
        if addr == 0 {
            // writing address $0000 outputs on an ASCII-only "serial port"
            print!("{}", String::from_utf8_lossy(&[value]));
        }
        else {
            // all other writes write to RAM
            self.ram[addr as usize] = value
        }
    }
}
```

This simulator is based on the simulator in the original [ARS
Emulator](https://github.com/SolraBizna/ars-emu).

# License

w65c02s is distributed under the zlib license. The complete text is as
follows:

> Copyright (c) 2019, Solra Bizna
> 
> This software is provided "as-is", without any express or implied
> warranty. In no event will the author be held liable for any damages
> arising from the use of this software.
> 
> Permission is granted to anyone to use this software for any purpose,
> including commercial applications, and to alter it and redistribute it
> freely, subject to the following restrictions:
> 
> 1. The origin of this software must not be misrepresented; you must not
> claim that you wrote the original software. If you use this software in a
> product, an acknowledgement in the product documentation would be
> appreciated but is not required.
> 2. Altered source versions must be plainly marked as such, and must not
> be misrepresented as being the original software.
> 3. This notice may not be removed or altered from any source
> distribution.
