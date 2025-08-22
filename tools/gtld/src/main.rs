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

#[derive(Debug, PartialEq, StructOpt)]
#[structopt(name = "gtld", about = "gametank (flash) loader")]
struct Opt {
    #[structopt(subcommand)]
    subcommand: Subcommands,
}

#[derive(Debug, PartialEq, StructOpt)]
enum Subcommands {
    Load { file: Option<String> },
    Dump {},
    DangerZone(DangerZone),
}

#[derive(Debug, PartialEq, StructOpt)]
enum DangerZone {
    FwUpdate { file: Option<String> },
    SelfDestruct,
}

fn main() {
    let opt: Opt = Opt::from_args();

    match opt.subcommand {
        Subcommands::Load { file } => {
            let mut port = get_port().expect("failed to open port");
            load_rom(&mut port, file).expect("failed to load rom");
        }
        Subcommands::Dump { .. } => {
            let mut port = get_port().expect("failed to open port");
            dump(&mut port);
        }
        Subcommands::DangerZone(DangerZone::FwUpdate { file }) => {
            let port = select_port().expect("failed to select port");
            flash_firmware(port, file)
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

fn select_port() -> anyhow::Result<String> {
    let ports = available_ports().expect("No ports found!");

    // filter ports for USB serial on linux/windows/macos
    let ports = ports
        .iter()
        .filter(|port| {
            port.port_name.contains("USB")
                || port.port_name.contains("COM")
                || port.port_name.contains("usb")
        })
        .collect::<Vec<&SerialPortInfo>>();

    match ports.as_slice() {
        [] => {
            println!("No USB serial ports found! Are you in the dialout group?");
            Err(anyhow::anyhow!("No USB serial ports found!"))
        }
        [p] => {
            println!("Using {}", p.port_name);
            Ok(p.port_name.clone())
        }
        ports => {
            println!("Multiple USB serial ports found");

            let port_names: Vec<String> = ports.iter().map(|port| port.port_name.clone()).collect();

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

fn get_port() -> anyhow::Result<Box<dyn SerialPort>> {
    let port_name = select_port()?;

    let port = serialport::new(&port_name, 115_200)
        .timeout(Duration::from_millis(20000))
        .open()
        .expect("Failed to open port");

    Ok(port)
}

fn load_rom(port: &mut Box<dyn SerialPort>, file: Option<String>) -> anyhow::Result<String> {
    // probably return a checksum?
    let path = file.ok_or_else(|| anyhow::anyhow!("No file provided"))?;
    let rom_buffer = fs::read(&path)?;

    read_output(port);

    port.write_all(b"mode f\r").expect("write data failed");
    port.flush().ok();
    wait_for_str(port, "FLASH");

    write_all(port, rom_buffer);

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
    }
}
