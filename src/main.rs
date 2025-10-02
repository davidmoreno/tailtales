use core::panic;
use std::io::{self, IsTerminal};
use std::time::{self};

use application::Application;
use clap::{Arg, Command};
use parser::Parser;
use regex::Regex;
use settings::Settings;
use settings::{Alignment, RulesSettings};

use crate::recordlist::load_parsers;

#[derive(Debug)]
struct ParsedArgs {
    mode: Option<String>,
    files: Vec<String>,
}

mod application;
mod ast;
mod events;
mod keyboard_input;
mod keyboard_management;
mod lua_engine;
mod parser;
mod record;
mod recordlist;
mod regex_cache;
mod settings;
mod state;
mod tuichrome;
mod utils;

fn main() {
    // Parse command line arguments first, before initializing TUI
    let args = parse_args_with_clap();

    let app = Application::new();

    if app.is_err() {
        eprintln!("Error starting application: {}", app.err().unwrap());
        std::process::exit(1);
    }

    let mut app = app.unwrap();

    let start_parse_time = time::Instant::now();

    // Apply the parsed arguments to the app
    apply_args_to_app(args, &mut app);

    keyboard_input::start_event_thread(app.ui.tx.clone());

    app.state.read_time = start_parse_time.elapsed();

    app.run();
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

fn parse_args_with_clap() -> ParsedArgs {
    let matches = Command::new("tt")
        .about("TailTales - Flexible log viewer for logfmt and other formats")
        .version("0.2.0")
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .value_name("MODE")
                .help("Force parsing mode (apache, nginx, json, csv, logfmt, etc.)"),
        )
        .arg(
            Arg::new("files")
                .num_args(0..)
                .help("Files to process, or '-' for stdin, or '--' followed by command to execute"),
        )
        .get_matches();

    let mode = matches.get_one::<String>("mode").cloned();
    let files = matches
        .get_many::<String>("files")
        .map(|f| f.cloned().collect())
        .unwrap_or_default();

    ParsedArgs { mode, files }
}

fn apply_args_to_app(args: ParsedArgs, app: &mut Application) {
    // Handle mode selection
    if let Some(mode) = args.mode {
        set_rule_by_mode(&mode, app);
    } else {
        // Use default behavior - determine mode from first file or default arguments
        let args_vec = if !args.files.is_empty() {
            let mut args_vec = vec!["tt".to_string()]; // Program name
            args_vec.extend(args.files.clone());
            args_vec
        } else {
            vec!["tt".to_string()]
        };
        set_rule_from_args(&args_vec, app);
    }

    // Handle file processing
    let args_vec = if !args.files.is_empty() {
        let mut args_vec = vec!["tt".to_string()]; // Program name
        args_vec.extend(args.files);
        args_vec
    } else if !stdin_is_a_file() {
        let mut args_vec = vec!["tt".to_string()];
        args_vec.extend(app.state.settings.default_arguments.clone());
        args_vec
    } else {
        vec!["tt".to_string()]
    };

    if args_vec.len() <= 1 {
        app.state.records.readfile_stdin(app.ui.tx.clone());
        return;
    }

    let mut narg = 1;
    while narg < args_vec.len() {
        let filename = &args_vec[narg];
        if filename == "-" {
            app.state.records.readfile_stdin(app.ui.tx.clone());
        } else if filename == "--" {
            // this is to exec a command and read the output
            let args: Vec<&str> = args_vec[(narg + 1)..].iter().map(|s| &**s).collect();
            app.state.records.readfile_exec(&args, app.ui.tx.clone());
            return;
        } else if filename.starts_with("!") {
            // this is to exec a command and read the output
            let mut args: Vec<&str> = args_vec[narg..].iter().map(|s| &**s).collect();
            if let Some(first_arg) = args.first_mut() {
                *first_arg = &first_arg[1..];
            }
            app.state.records.readfile_exec(&args, app.ui.tx.clone());
            return;
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

fn set_rule_by_mode(mode: &str, app: &mut Application) {
    // Find rule by name instead of filename
    let rule = app
        .state
        .settings
        .rules
        .iter()
        .find(|rule| rule.name == mode)
        .cloned();

    match rule {
        Some(rule) => {
            app.state.current_rule = rule;
            if let Err(err) = load_parsers(&app.state.current_rule, &mut app.state.records.parsers)
            {
                panic!("Could not load parsers for mode '{}': {:?}", mode, err);
            }
        }
        None => {
            eprintln!("Error: Unknown mode '{}'", mode);
            eprintln!(
                "Available modes: {}",
                app.state
                    .settings
                    .rules
                    .iter()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            std::process::exit(1);
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
