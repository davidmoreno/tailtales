use core::panic;
use std::io::{self, IsTerminal};
use std::time::{self};

use application::Application;
use parser::Parser;
use regex::Regex;
use settings::Settings;
use settings::{Alignment, RulesSettings};

use crate::args::{parse_args_with_clap, ParsedArgs};
use crate::recordlist::load_parsers;
use std::fs;

mod application;
mod args;
mod ast;
mod completions;
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
    // Get command line arguments
    let raw_args: Vec<String> = std::env::args().collect();

    // Parse command line arguments first, before initializing TUI
    let args = parse_args_with_clap(raw_args);

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

fn apply_args_to_app(args: ParsedArgs, app: &mut Application) {
    // Handle Lua script execution first (if provided)
    if let Some(script_path) = args.lua_script {
        execute_lua_script(&script_path, app);
        // Continue to process files after script execution
    }

    // Handle mode selection
    if let Some(rule) = args.rule {
        set_rule_by_name(&rule, app);
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

fn execute_lua_script(script_path: &str, app: &mut Application) {
    // Ensure Lua console is initialized with welcome message
    app.state.ensure_lua_console_initialized();

    // Read the Lua script file
    let script_content = match fs::read_to_string(script_path) {
        Ok(content) => content,
        Err(e) => {
            // Add error to Lua console instead of stderr
            app.state
                .add_error_to_lua_console(format!("Error reading Lua script '{}': {}", script_path, e));
            std::process::exit(1);
        }
    };

    // Execute the script in normal mode (not REPL mode)
    // This allows the script to set up record processors and other configurations
    // while keeping the main view active for file processing

    // Compile and execute the script
    let script_name = format!(
        "external_script_{}",
        script_path.replace("/", "_").replace("\\", "_")
    );

    match app.lua_engine.compile_script(&script_name, &script_content) {
        Ok(_) => {
            // Execute the script and capture output
            match app
                .lua_engine
                .execute_script_string_with_state(&script_content, &mut app.state)
            {
                Ok(output) => {
                    // Add script output to Lua console instead of printing to stderr
                    if !output.is_empty() {
                        app.state
                            .add_to_lua_console(format!("Script '{}' output:", script_path));
                        let output_lines: Vec<String> =
                            output.lines().map(|line| format!("  {}", line)).collect();
                        app.state.add_lines_to_lua_console(output_lines);
                    }
                    // Add success message to Lua console
                    app.state.add_to_lua_console(format!(
                        "Script '{}' executed successfully.",
                        script_path
                    ));
                }
                Err(e) => {
                    // Add error to Lua console instead of stderr
                    app.state.add_error_to_lua_console(format!(
                        "Error executing script '{}': {}",
                        script_path, e
                    ));
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            // Add error to Lua console instead of stderr
            app.state
                .add_error_to_lua_console(format!("Error compiling script '{}': {}", script_path, e));
            std::process::exit(1);
        }
    }
}

fn set_rule_by_name(name: &str, app: &mut Application) {
    // Find rule by name instead of filename
    let rule = app
        .state
        .settings
        .rules
        .iter()
        .find(|rule| rule.name == name)
        .cloned();

    match rule {
        Some(rule) => {
            app.state.current_rule = rule;
            if let Err(err) = load_parsers(&app.state.current_rule, &mut app.state.records.parsers)
            {
                panic!("Could not load parsers for mode '{}': {:?}", name, err);
            }
        }
        None => {
            eprintln!("Error: Unknown mode '{}'", name);
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
