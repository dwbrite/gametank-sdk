pub mod builder;

use clap::{Parser, Subcommand};

use crate::builder::RomBuilder;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Configure {
        /// gtrom.toml to configure llvm shit.
        /// By default checks for a rustup mos toolchain, then checks for a podman or docker container
        _config_file: Option<String>,
    },

    Build {},
}

fn main() {
    let cli = Cli::parse();

    // TODO: check for

    match cli.command {
        Commands::Configure { _config_file } => println!("not implemented"),
        Commands::Build {} => {
            // assumes you're in the sdk/ directory
            let working_dir = std::env::current_dir().expect("Failed to get current directory");
            let rom_path = working_dir.join("rom");
            let _rb = RomBuilder::init(rom_path.to_string_lossy().to_string());
        }
    }
}
