mod action;
mod app;
mod cli_args;
mod component;
mod components;
mod config;
mod export;
mod file;
mod lane;
mod player;
mod sample_rate;
mod scheme;
mod tracker;

use clap::Parser;
use cli_args::CommandLineArgs;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = CommandLineArgs::parse();
    let result = app::App::new(args.input)?.run();
    ratatui::restore();
    Ok(result?)
}
