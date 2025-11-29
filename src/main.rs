mod cache_parser;
use cache_parser::parse_cmake_cache;

use std::io;

mod app;
use app::App;

mod trace_logger;
// use tracing::initialize_logging;
// use tracing::{trace_dbg};


use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    // trace_logger::initialize_logging()?;  

    // now logging works!
    // tracing::info!("App starting…");
    // tracing::debug!("Debug message here");
    // tracing::warn!("Something odd happened…");

    let terminal = ratatui::init();
    let app_result = App::default().run(terminal);
    ratatui::restore();
    app_result
}
