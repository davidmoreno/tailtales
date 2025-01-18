use std::time::{self};

use regex::Regex;
use settings::RulesSettings;
use settings::Settings;

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

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        tui_chrome.state.current_rule = get_rule_by_filename(
            &mut tui_chrome.state.settings,
            args.get(1).unwrap().to_string(),
        );
    }
    if let Err(err) = load_parsers(
        &tui_chrome.state.current_rule,
        &mut tui_chrome.state.records.parsers,
    ) {
        panic!("Could not load parsers from settings: {:?}", err);
    }
    keyboard_input::start_event_thread(tui_chrome.tx.clone());

    if args.len() == 1 {
        tui_chrome
            .state
            .records
            .readfile_stdin(tui_chrome.tx.clone());
    }
    for filename in &args[1..] {
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

fn load_parsers(
    rule: &RulesSettings,
    parsers: &mut Vec<parser::Parser>,
) -> Result<(), parser::ParserError> {
    for extractor in rule.extractors.iter() {
        parsers.push(parser::Parser::parse(extractor)?);
    }

    Ok(())
}

fn get_rule_by_filename(settings: &mut Settings, filename: String) -> RulesSettings {
    let rules = &settings.rules;

    let mut count = 0;
    for rule in rules.iter() {
        for pattern in &rule.file_patterns {
            if Regex::new(pattern).unwrap().is_match(&filename) {
                return rule.clone();
            }
        }
        count += 1;
    }

    panic!(
        "Could not guess rules for filename: {}. Checked {} rule sets.",
        filename, count
    );
}
