mod cache_parser;
mod app;

use app::App;
use std::path::PathBuf;
use clap::{Parser};
use color_eyre::Result;

#[derive(Parser, Debug)]
#[command(
    version = "0.1",
    about = "Modify CMake cache variables",
)]
struct Cli {
    #[arg(short, long, default_value = ".")]
    path: PathBuf,
}


fn main() -> Result<()> {
    let cli = Cli::parse();
    // if !cli.path.exists() {
    //     eprintln!("Error: path '{}' does not exist.", cli.path.display());
    //     std::process::exit(1);
    // }

    println!("Using directory: {}", cli.path.display());

    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::new(cli.path).run(terminal);
    ratatui::restore();
    app_result
}
