use core::panic;
use std::io::{self, IsTerminal};
use std::time::{self};

use application::Application;
use parser::Parser;
use regex::Regex;
use settings::Settings;
use settings::{Alignment, RulesSettings};

mod application;
mod ast;
mod events;
mod keyboard_input;
mod keyboard_management;
mod parser;
mod record;
mod recordlist;
mod regex_cache;
mod settings;
mod state;
mod tuichrome;
mod utils;

fn main() {
    let mut app = Application::new().expect("could not create Application");
    let start_parse_time = time::Instant::now();

    parse_args(&std::env::args().collect(), &mut app);

    keyboard_input::start_event_thread(app.ui.tx.clone());

    app.state.read_time = start_parse_time.elapsed();

    app.run();
}

fn load_parsers(
    rule: &RulesSettings,
    parsers: &mut Vec<parser::Parser>,
) -> Result<(), parser::ParserError> {
    for extractor in rule.extractors.iter() {
        parsers.push(parser::Parser::new(extractor)?);
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

fn parse_args(args: &Vec<String>, app: &mut Application) {
    set_rule_from_args(&args, app);

    let args = if args.len() == 1 && !stdin_is_a_file() {
        let mut args = vec![args[0].clone()];
        args.extend(app.state.settings.default_arguments.clone());
        args
    } else {
        args.clone()
    };

    if args.len() <= 1 {
        app.state.records.readfile_stdin(app.ui.tx.clone());
        return;
    }

    let mut narg = 1;
    while narg < args.len() {
        let filename = &args[narg];
        if filename == "-" {
            app.state.records.readfile_stdin(app.ui.tx.clone());
        } else if filename == "--" {
            // this is to exec a command and read the output
            let args: Vec<&str> = args[(narg + 1)..].iter().map(|s| &**s).collect();
            app.state.records.readfile_exec(&args, app.ui.tx.clone());
            return;
        } else if filename.starts_with("!") {
            // this is to exec a command and read the output
            let mut args: Vec<&str> = args[narg..].iter().map(|s| &**s).collect();
            if let Some(first_arg) = args.first_mut() {
                *first_arg = &first_arg[1..];
            }
            app.state.records.readfile_exec(&args, app.ui.tx.clone());
        } else if filename.ends_with(".gz") {
            app.state.records.readfile_gz(&filename);
        } else {
            app.state
                .records
                .readfile_parallel(&filename, app.ui.tx.clone());
        }
        narg += 1;
    }

    // If the parser is CSV, we auto add the columns from the headers
    for parser_i in &app.state.records.parsers {
        if let Parser::Csv(parser) = parser_i {
            let headers = &parser.read().unwrap().headers;
            for header in headers {
                app.state
                    .current_rule
                    .columns
                    .push(settings::ColumnSettings {
                        name: header.clone(),
                        width: header.len().max(app.state.records.max_record_size(header)),
                        align: Alignment::Left,
                    });
            }
        }
    }
}

fn set_rule_from_args(args: &Vec<String>, app: &mut Application) {
    let filename = if args.len() > 1 {
        args.get(1).unwrap().to_string()
    } else {
        app.state
            .settings
            .default_arguments
            .get(0)
            .unwrap()
            .to_string()
    };

    app.state.current_rule = get_rule_by_filename(&mut app.state.settings, filename);

    if let Err(err) = load_parsers(&app.state.current_rule, &mut app.state.records.parsers) {
        panic!("Could not load parsers from settings: {:?}", err);
    }
}

// Checks if stdin is a file in contraswt to a tty
fn stdin_is_a_file() -> bool {
    let stdin = io::stdin();

    return !stdin.is_terminal();
}
