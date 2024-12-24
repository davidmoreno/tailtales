use std::time::{self};

mod record;
mod tuichrome;

fn main() {
    // get a filename from the command line
    let filename = std::env::args().nth(1).expect("need a filename");
    println!("reading from file: {}", filename);

    let mut tui_chrome = tuichrome::TuiChrome::new().expect("could not create TuiChrome");
    let start_parse_time = time::Instant::now();
    tui_chrome.state.records.readfile_parallel(&filename);
    tui_chrome.state.read_time = start_parse_time.elapsed();

    tui_chrome.run();
}
