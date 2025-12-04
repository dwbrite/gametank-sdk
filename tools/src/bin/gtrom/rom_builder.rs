use std::{fs::File, io::Write};

use elf::{ElfBytes, endian::AnyEndian};
use rustc_demangle::demangle;

#[derive(Debug, Clone)]
pub struct ElfSection {
    _internal_name: String,
    display_name: String,
    bytes: Vec<u8>,
    size: usize,
    mem_loc: usize,
    bank_loc: usize,
    bank: u8,
}

impl ElfSection {
    fn from_static(section_name: String, elf: &ElfBytes<'_, AnyEndian>, bank: u8) -> Option<Self> {
        let header = elf
            .section_header_by_name(&section_name)
            .expect("something failed")?;

        let load_addr = header.sh_addr as usize;
        let size = header.sh_size as usize;
        let offset_in_bank = load_addr & 0x3FFF;

        let (d, _ch) = elf.section_data(&header).expect("bruh");

        Some(Self {
            display_name: demangle(&section_name).to_string(),
            _internal_name: section_name,
            bytes: Vec::from(d),
            size,
            mem_loc: load_addr,
            bank,
            bank_loc: offset_in_bank,
        })
    }

    fn from_loaded(
        section_name: String,
        elf: &ElfBytes<'_, AnyEndian>,
        load_symbol: String,
    ) -> Option<Self> {
        let header = elf.section_header_by_name(&section_name).ok().flatten()?;
        let (bytes, _) = elf.section_data(&header).ok()?;

        let (symtab, strtab) = elf.symbol_table().ok().flatten()?;
        let load_sym = symtab.iter().find_map(|sym| {
            let name = strtab.get(sym.st_name as usize).ok()?;
            if name == load_symbol { Some(sym) } else { None }
        })?;

        let load_rom_addr = load_sym.st_value as usize; // where the section is in ROM
        let mem_target_addr = header.sh_addr as usize; // where the section ends up in RAM

        Some(Self {
            display_name: demangle(&section_name).to_string(),
            _internal_name: section_name,
            bytes: bytes.to_vec(),
            size: bytes.len(),
            mem_loc: mem_target_addr,
            bank: 127,
            bank_loc: load_rom_addr & 0x3FFF,
        })
    }
}

pub struct RomBuilder {}

impl RomBuilder {
    /// Build a .gtr ROM from an ELF file
    pub fn build(elf_path: String, output_path: String) -> Self {
        let file_data = std::fs::read(&elf_path).expect("Could not read ELF file.");
        let slice = file_data.as_slice();
        let file = ElfBytes::<AnyEndian>::minimal_parse(slice).expect("Failed to parse ELF");
        let elf = &file;

        // 128 banks
        let static_sections: [Vec<String>; 128] = std::array::from_fn(|i| match i {
            0..=126 => vec![format!(".text.bank{}", i), format!(".rodata.bank{}", i)],
            127 => vec![
                ".text".to_string(),
                ".rodata".to_string(),
                ".vector_table".to_string(),
            ],
            _ => panic!("you fucked up"),
        });

        // loaded sections must be in the FIXED bank for crt0
        let loaded_sections = [
            (".data".to_string(), "__data_load".to_string()),
            (".zp".to_string(), "__zp_load".to_string()),
        ];

        let map_sections: Vec<ElfSection> = static_sections
            .iter()
            .enumerate()
            .flat_map(|(bank, names)| {
                names
                    .iter()
                    .filter_map(move |name| ElfSection::from_static(name.clone(), elf, bank as u8))
            })
            .chain(loaded_sections.iter().filter_map(|(section, load_symbol)| {
                ElfSection::from_loaded(section.clone(), elf, load_symbol.clone())
            }))
            .collect();

        // ROM data - 128x 16k banks
        let mut rom = [[0x00u8; 1 << 14]; 128];

        for s in map_sections {
            rom[s.bank as usize][s.bank_loc..s.bank_loc + s.size].copy_from_slice(&s.bytes);
            println!(
                "{:<24}bank {} @{:04X}..{:04X} ${:04X}",
                s.display_name,
                s.bank,
                s.bank_loc,
                s.bank_loc + s.size,
                s.mem_loc
            );
        }

        let mut file = File::create(&output_path).expect("Failed to create output file");
        let flat: &[u8; 2 * 1024 * 1024] = unsafe { core::mem::transmute(&rom) };
        file.write_all(flat).expect("Failed to write ROM data");

        println!("Created: {}", output_path);

        Self {}
    }
}
