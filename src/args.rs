//! Command Line Argument Parsing
//!
//! This module handles parsing of command line arguments for TailTales.

use clap::{Arg, Command};

#[derive(Debug)]
pub struct ParsedArgs {
    pub rule: Option<String>,
    pub files: Vec<String>,
    pub lua_script: Option<String>,
}

/// Parse command line arguments using clap
pub fn parse_args_with_clap(args: Vec<String>) -> ParsedArgs {
    let settings = crate::settings::Settings::new().unwrap();
    let rules = settings
        .rules
        .iter()
        .map(|r| r.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    // Use the provided arguments to handle -- separator manually
    let raw_args = args;

    // Find the -- separator position
    let separator_pos = raw_args.iter().position(|arg| arg == "--");

    // Split arguments at the -- separator
    let (tt_args, command_args) = if let Some(pos) = separator_pos {
        let (before_sep, after_sep) = raw_args.split_at(pos);
        (before_sep.to_vec(), Some(after_sep[1..].to_vec())) // Skip the "--" itself
    } else {
        (raw_args, None)
    };

    let matches = Command::new("tt")
        .about("TailTales - Flexible log viewer for logfmt and other formats")
        .version("0.2.0")
        .arg(
            Arg::new("rule")
                .long("rule")
                .value_name("RULE")
                .help(format!("Force parsing rule ({})", &rules)),
        )
        .arg(
            Arg::new("lua")
                .long("lua")
                .value_name("SCRIPT")
                .help("Execute a Lua script file before processing logs"),
        )
        .arg(
            Arg::new("files")
                .num_args(0..)
                .help("Files to process, or '-' for stdin, or '--' followed by command to execute"),
        )
        .try_get_matches_from(tt_args);

    let matches = match matches {
        Ok(matches) => matches,
        Err(e) => {
            eprintln!("Error parsing arguments: {}", e);
            std::process::exit(1);
        }
    };

    let rule = matches.get_one::<String>("rule").cloned();
    let lua_script = matches.get_one::<String>("lua").cloned();
    let mut files: Vec<String> = matches
        .get_many::<String>("files")
        .map(|f| f.cloned().collect())
        .unwrap_or_default();

    // If we have command arguments after --, add them to files
    if let Some(cmd_args) = command_args {
        files.push("--".to_string());
        files.extend(cmd_args);
    }

    ParsedArgs {
        rule,
        files,
        lua_script,
    }
}
