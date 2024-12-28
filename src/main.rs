use std::time::{self};

mod parser;
mod record;
mod tuichrome;

fn main() {
    let mut tui_chrome = tuichrome::TuiChrome::new().expect("could not create TuiChrome");
    let start_parse_time = time::Instant::now();
    tui_chrome
        .state
        .all_records
        .parsers
        .push(parser::Parser::new_from_pattern(
            r"<timestamp> <hostname> <program>: <rest>",
        ));
    let args = std::env::args();
    if args.len() == 1 {
        tui_chrome.state.all_records.readfile_stdin();
    }
    for filename in args.skip(1) {
        if filename == "-" {
            tui_chrome.state.all_records.readfile_stdin();
        } else {
            tui_chrome.state.all_records.readfile_parallel(&filename);
        }
    }
    tui_chrome.state.records = tui_chrome.state.all_records.clone();
    tui_chrome.state.read_time = start_parse_time.elapsed();

    tui_chrome.run();
}
