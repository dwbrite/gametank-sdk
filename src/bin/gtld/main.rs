use dialoguer::Select;
use dialoguer::console::style;
use serialport::{SerialPort, SerialPortInfo, available_ports};
use std::fs;
use std::io::{Read, Write};
use std::thread::sleep;
use std::time::Duration;
use structopt::StructOpt;
use tempfile::NamedTempFile;

static FIRMWARE: &[u8] = include_bytes!("latest-fw.hex");

enum Cartridge {
    Cart2M,
    Cart32K,
    Cart8K,
}

#[derive(Debug, PartialEq, StructOpt)]
#[structopt(name = "gtld", about = "gametank (flash) loader")]
struct Opt {
    #[structopt(subcommand)]
    subcommand: Subcommands,
}

#[derive(Debug, PartialEq, StructOpt)]
enum Subcommands {
    Load { file: Option<String>, 
        #[structopt(short, long)]
        port: Option<String>
    },
    Dump {
        #[structopt(short, long)]
        port: Option<String>
    },
    DangerZone(DangerZone),
}

#[derive(Debug, PartialEq, StructOpt)]
enum DangerZone {
    FwUpdate {
        file: Option<String>,
        #[structopt(short, long)]
        port: Option<String>
    },
    SelfDestruct,
}

fn main() {
    let opt: Opt = Opt::from_args();

    match opt.subcommand {
        Subcommands::Load { file, port  } => {
            let (mut port, _) = get_port(port).expect("failed to open port");
            reset_microcontroller(&mut port);
            load_rom(&mut port, file, None).expect("failed to load rom");
        }
        Subcommands::Dump { port } => {
            let (mut port, _) = get_port(port).expect("failed to open port");
            reset_microcontroller(&mut port);
            dump(&mut port);
        }
        Subcommands::DangerZone(DangerZone::FwUpdate { file, port }) => {
            let port_name = select_port(port).expect("failed to select port");
            flash_firmware(port_name, file)
        }
        Subcommands::DangerZone(DangerZone::SelfDestruct) => {
            println!("{}", style("What is *wrong* with you???").dim().italic());
            sleep(Duration::from_secs(1));

            println!("{}", style("...").dim());

            sleep(Duration::from_secs(2));
            println!("{}", style("💥💥💥").red().bold().italic());
        }
    }
}

fn reset_microcontroller(port: &mut Box<dyn SerialPort>) {
    port.write_data_terminal_ready(false).expect("failed to set DTR");
    sleep(Duration::from_millis(100));
    port.write_data_terminal_ready(true).expect("failed to set DTR");
}

fn select_port(port: Option<String>) -> anyhow::Result<String> {
    let ports = available_ports().expect("No ports found!");

    // filter ports for USB serial on linux/windows/macos
    let ports = ports
        .iter()
        .filter(|port| {
            port.port_name.contains("USB")
                || port.port_name.contains("COM")
                || port.port_name.contains("usb")
                || port.port_name.contains("ACM")
        })
        .collect::<Vec<&SerialPortInfo>>();

    match ports.as_slice() {
        [] => {
            println!("No USB serial ports found! Are you in the dialout group?");
            Err(anyhow::anyhow!("No USB serial ports found!"))
        }
        [p] => {
            // if port name is provided and NOT in ports, error, otherwise use the one port
            if let Some(port) = port {
                if !p.port_name.ends_with(&port) {
                    println!("Provided port {} not found among USB serial ports", port);
                    return Err(anyhow::anyhow!("Provided port not found among USB serial ports"));
                }
            }
            println!("Using {}", p.port_name);
            Ok(p.port_name.clone())
        }
        ports => {
            println!("Multiple USB serial ports found");

            let port_names: Vec<String> = ports.iter().map(|port| port.port_name.clone()).collect();

            // if port is in port_names, uniquely, select it, otherwise prompt
            if let Some(port) = port {
                if let Some(idx) = port_names.iter().position(|name| name.ends_with(&port)) {
                    println!("Using {}", port_names[idx]);
                    return Ok(port_names[idx].clone());
                } else {
                    println!("Provided port {} not found among USB serial ports", port);
                }
            }

            let selected = Select::new()
                .with_prompt("Select your USB serial port")
                .default(0)
                .items(&port_names)
                .interact()
                .expect("this should work?");

            Ok(port_names[selected].clone())
        }
    }
}

fn get_port(maybe_port_name: Option<String>) -> anyhow::Result<(Box<dyn SerialPort>, String)> {
    let port_name = select_port(maybe_port_name).expect("failed to select port");
    println!("{}", port_name);

    let port = serialport::new(&port_name, 115_200)
        .timeout(Duration::from_millis(20000))
        .open()
        .expect("Failed to open port");

    Ok((port, port_name))
}

fn load_rom(port: &mut Box<dyn SerialPort>, file: Option<String>, cartridge: Option<Cartridge>) -> anyhow::Result<String> {
    // TODO: probably return a checksum?
    let path = file.ok_or_else(|| anyhow::anyhow!("No file provided"))?;
    let rom_buffer = fs::read(&path)?;

    // TODO: heuristics to determine cartridge type, but for now assume 2M if not provided
    let cartridge = cartridge.unwrap_or_else(|| { Cartridge::Cart2M});

    port.write(b"mode f\r").expect("write failed");
    port.flush().ok();
    wait_for_str(port, "FLASH");

    // stretch 32k roms to fill all 2M banks, such that they're usable without bank switching
    let stretched_rom_buffer = match rom_buffer.len() {
        32_768 => {
            let mut stretched = Vec::new();
            // Fill first 127 banks with the first 16k, last bank with the second 16k
            for _ in 0..127 {
                stretched.extend_from_slice(&rom_buffer[..16_384]);
            }
            stretched.extend_from_slice(&rom_buffer[16_384..]);
            stretched
        }
        _ => rom_buffer.clone(),
    };

    read_output(port);

    port.write_all(b"mode f\r").expect("write data failed");
    port.flush().ok();
    wait_for_str(port, "FLASH");

    write_all(port, stretched_rom_buffer);

    port.flush()?;

    Ok("go check it".to_string())
}

pub fn read_output(port: &mut Box<dyn SerialPort>) {
    // Read whatever's there
    let mut buf = [0u8; 1024];
    match port.read(&mut buf) {
        Ok(n) if n > 0 => {
            let line = String::from_utf8_lossy(&buf[..n]);
            let mut styled = style(&line).dim();
            if line.contains(">") {
                styled = styled.italic();
            }
            println!("{}", styled);
        }
        _ => panic!("Waited too long for output"),
    }
    port.flush().ok();
}

pub fn write_bank(port: &mut Box<dyn SerialPort>, bank: u8, data: &[u8]) {
    let crc32_in = crc32fast::hash(data);

    port.write_all(format!("shift {:X}\r", bank).as_bytes())
        .expect("Failed to write bank");
    port.flush().ok();
    read_output(port);

    let chunks = data.len() / 4096;

    for chunk in 0..chunks {
        let chunk_start = chunk * 4096;
        let chunk_end = chunk_start + 4096;

        // Send the header alone
        let header = format!("writeMulti {:X} 1000\r", chunk_start);
        port.write_all(header.as_bytes())
            .expect("write header failed");
        port.flush().ok();

        sleep(Duration::from_millis(50));

        port.write_all(&data[chunk_start..chunk_end])
            .expect("write data failed");
        port.flush().ok();

        sleep(Duration::from_millis(20));

        wait_for_str(port, "ACK");
    }

    port.write_all("checksum 0 4000\r".as_bytes())
        .expect("failed to get checksum");
    let checksum = wait_for_str(port, "CRC32");

    if checksum.contains(&format!("{:X}", crc32_in)) {
        println!("{}", style("Checksum valid").green());
    } else {
        panic!("Checksum failed, try again and/or ping burdock");
    }
}

fn wait_for_str(port: &mut Box<dyn SerialPort>, contains: &str) -> String {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];

    loop {
        match port.read(&mut byte) {
            Ok(1) => {
                if byte[0] == b'\n' {
                    let line = String::from_utf8_lossy(&buf);
                    let mut styled = style(&line).dim();
                    if line.contains(">") {
                        styled = styled.italic();
                    }
                    println!("{}", styled);

                    if line.contains(contains) {
                        return line.to_string();
                    } else {
                        buf.clear(); // reset for next line
                    }
                } else {
                    buf.push(byte[0]);
                }
            }
            _ => continue,
        }
    }
}

pub fn flash_firmware(port_name: String, firmware: Option<String>) {
    let mut tmp = NamedTempFile::new().unwrap();

    let firmware_file = match firmware {
        None => {
            tmp.write_all(&FIRMWARE).unwrap();
            tmp.path().to_str().unwrap().to_string()
        }
        Some(path) => path,
    };

    flash_optiboot_da(&port_name, &firmware_file);
}

pub fn flash_optiboot_da(port: &str, firmware_path: &str) {
    let status = std::process::Command::new("avrdude")
        .args(&[
            "-v",
            "-p",
            "avr64da64",
            "-c",
            "arduino",
            "-P",
            port,
            "-b",
            "115200",
            "-D",
            "-U",
            &format!("flash:w:{}:i", firmware_path),
        ])
        .status()
        .expect("Failed to run avrdude");

    if !status.success() {
        panic!("avrdude exited with status {}", status);
    }
}

pub fn dump(port: &mut Box<dyn SerialPort>) {
    let mut buf = [0u8; 4096 * 4];
    port.write_all(b"dump\r").unwrap();
    port.flush().ok();

    port.read_exact(&mut buf).unwrap();
    println!("{:?}", &buf);
}

pub fn write_all(port: &mut Box<dyn SerialPort>, data: Vec<u8>) {
    let mut data = data.to_vec();
    let remainder = data.len() % 16_384;
    if remainder != 0 {
        data.splice(0..0, std::iter::repeat(0xFF).take(16_384 - remainder));
    }

    let num_banks = data.len() / 16_384; // # of 16k banks
    let first_bank = 128 - num_banks;
    println!("Writing {} bank(s)", num_banks);

    port.write_all(b"reset\r").expect("reset failed");
    port.flush().ok();
    wait_for_str(port, "OK");

    port.write_all(b"eraseChip\r").expect("erase failed");
    port.flush().ok();
    wait_for_str(port, "Done");

    for (idx, shifted_bank) in (first_bank..128).enumerate() {
        let start = idx * 16384;
        let end = (idx + 1) * 16384;

        let hash = crc32fast::hash(&data[start..end]);
        if hash == 0xAB_54_D2_86 {
            continue;
        }
        write_bank(port, shifted_bank as u8, &data[start..end]);
        port.flush().ok();
    }
}
