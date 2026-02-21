#![forbid(unsafe_code)]
use gemini2html::parse_gemini_file;

use log::info;
use std::path::Path;

fn main() {
    env_logger::Builder::default()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("🚀 start gemini2html");
    // TODO args for input / output folder ?
    // TODO loop on files
    parse_gemini_file(Path::new("./tests/vlc-via-smb.gmi"));
    info!("💤 end gemini2html");
}
