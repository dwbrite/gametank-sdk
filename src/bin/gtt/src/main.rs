mod action;
mod app;
mod cli_args;
mod component;
mod components;
mod config;
mod export;
mod file;
mod focus_cursor;
mod lane;
mod player;
mod scheme;
mod tracker;
mod util;

use clap::Parser;
use cli_args::CommandLineArgs;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = CommandLineArgs::parse();
    let result = app::App::new(args.input)?.run();
    ratatui::restore();
    Ok(result?)
}
