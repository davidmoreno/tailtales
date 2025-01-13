use std::time::{self};

use settings::SETTINGS;

mod ast;
mod events;
mod keyboard_input;
mod parser;
mod record;
mod recordlist;
mod regex_cache;
mod settings;
mod tuichrome;

fn main() {
    let mut tui_chrome = tuichrome::TuiChrome::new().expect("could not create TuiChrome");
    let start_parse_time = time::Instant::now();

    if let Err(err) = load_parsers_from_settings(&mut tui_chrome) {
        panic!("Could not load parsers from settings: {:?}", err);
    }
    keyboard_input::start_event_thread(tui_chrome.tx.clone());

    let args = std::env::args();
    if args.len() == 1 {
        tui_chrome
            .state
            .records
            .readfile_stdin(tui_chrome.tx.clone());
    }
    for filename in args.skip(1) {
        if filename == "-" {
            tui_chrome
                .state
                .records
                .readfile_stdin(tui_chrome.tx.clone());
        } else {
            tui_chrome
                .state
                .records
                .readfile_parallel(&filename, tui_chrome.tx.clone());
        }
    }
    tui_chrome.state.read_time = start_parse_time.elapsed();

    tui_chrome.run();
}

fn load_parsers_from_settings(
    tui_chrome: &mut tuichrome::TuiChrome,
) -> Result<(), parser::ParserError> {
    let parsers = &mut tui_chrome.state.records.parsers;

    // find the detault rules
    let rules = SETTINGS
        .rules
        .iter()
        .find(|rules| rules.name == "default")
        .expect("No default rules found");

    for extractor in &rules.extractors {
        parsers.push(parser::Parser::parse(&extractor)?);
    }

    Ok(())
}
