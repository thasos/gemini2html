#![forbid(unsafe_code)]
use gemini2html::Gemini2HtmlError;
use gemini2html::convert_gemini_file;

use log::info;
use std::path::Path;

fn main() -> Result<(), Gemini2HtmlError> {
    env_logger::Builder::default()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("🚀 start gemini2html");
    info!("---------------------");
    // TODO args for input / output folder ?
    // TODO loop on files
    let gemini_file = Path::new("./tests/gemini_file.gmi");
    let html_file = Path::new("./tests/html_file.html");
    // TODO handle result
    convert_gemini_file(gemini_file, html_file)?;
    info!("---------------------");
    info!("💤 end gemini2html");
    Ok(())
}
