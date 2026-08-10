use clap::Parser;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

#[derive(Parser, Debug)]
#[command(name = NAME, version = VERSION, about = DESCRIPTION)]
pub struct CommandLineArgs {
    #[arg(value_name = "FILE")]
    pub input: Option<std::path::PathBuf>,
}
