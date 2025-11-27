mod cache_parser;
use cache_parser::parse_cmake_cache;

use std::io;

mod app;
use app::App;

// fn main() -> Result<(), Box<dyn std::error::Error>> {
// fn main() -> io::Result<()> {
//     // let var_map = parse_cmake_cache("/tools/work/x-heep/build/")?;
//     //
//     // for entry in var_map.values() {
//     //     println!("{}", entry);
//     // }
//     //
//     // println!("Parsed {} cache entries", var_map.len());
//
//     // TUI
//     let mut terminal = ratatui::init();
//     let app_result = App::default().run(&mut terminal);
//     ratatui::restore();
//
//     app_result
// }


use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::default().run(terminal);
    ratatui::restore();
    app_result
}
