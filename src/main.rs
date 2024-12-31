use std::sync::mpsc;
use std::time::{self};

mod events;
mod keyboard_input;
mod parser;
mod record;
mod recordlist;
mod tuichrome;

fn main() {
    let mut tui_chrome = tuichrome::TuiChrome::new().expect("could not create TuiChrome");
    let start_parse_time = time::Instant::now();
    let parsers = &mut tui_chrome.state.records.parsers;
    parsers.push(parser::Parser::new_from_pattern(
        r"<timestamp> <hostname> <program>: <rest>",
    ));
    parsers.push(parser::Parser::new_logfmt());
    parsers.push(parser::Parser::new_from_regex(
        r"^(?P<timestamp>....-..-.. ..:..:..) (?P<action>startup .*? .*?)$",
    ));
    parsers.push(parser::Parser::new_from_regex(
        r"^(?P<timestamp>....-..-.. ..:..:..) (?P<action>status .*?|upgrade|install|remove) (?P<package>.*?)-(?P<version>\d.*) (?P<version_2>.+)$"
    ));
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
