//! Direct Argument Parsing Tests
//!
//! These tests directly test the parse_args_with_clap function without
//! running the full application or initializing the TUI.

use tailtales::args::{parse_args_with_clap, ParsedArgs};

/// Helper function to test argument parsing directly
fn test_argument_parsing(args: Vec<&str>) -> ParsedArgs {
    let mut full_args = vec!["tt".to_string()];
    full_args.extend(args.iter().map(|s| s.to_string()));
    parse_args_with_clap(full_args)
}

#[test]
fn test_double_dash_separator_parsing() {
    println!("Testing -- separator argument parsing");

    // Test case 1: tt -- journalctl
    let result = test_argument_parsing(vec!["--", "journalctl"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(result.files, vec!["--", "journalctl"]);
    println!("✓ tt -- journalctl parsed correctly");

    // Test case 2: tt -- find -type f
    let result = test_argument_parsing(vec!["--", "find", "-type", "f"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(result.files, vec!["--", "find", "-type", "f"]);
    println!("✓ tt -- find -type f parsed correctly");

    // Test case 3: tt -- ls -la
    let result = test_argument_parsing(vec!["--", "ls", "-la"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(result.files, vec!["--", "ls", "-la"]);
    println!("✓ tt -- ls -la parsed correctly");
}

#[test]
fn test_tt_args_before_separator_parsing() {
    println!("Testing tt arguments before -- separator parsing");

    // Test case: tt --rule logfmt -- journalctl
    let result = test_argument_parsing(vec!["--rule", "logfmt", "--", "journalctl"]);
    assert_eq!(result.rule, Some("logfmt".to_string()));
    assert_eq!(result.lua_script, None);
    assert_eq!(result.files, vec!["--", "journalctl"]);
    println!("✓ tt --rule logfmt -- journalctl parsed correctly");

    // Test case: tt --lua script.lua -- echo hello
    let result = test_argument_parsing(vec!["--lua", "script.lua", "--", "echo", "hello"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, Some("script.lua".to_string()));
    assert_eq!(result.files, vec!["--", "echo", "hello"]);
    println!("✓ tt --lua script.lua -- echo hello parsed correctly");
}

#[test]
fn test_complex_command_args_parsing() {
    println!("Testing complex command arguments after -- separator parsing");

    // Test case: tt -- find . -name "*.rs" -type f
    let result = test_argument_parsing(vec!["--", "find", ".", "-name", "*.rs", "-type", "f"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(
        result.files,
        vec!["--", "find", ".", "-name", "*.rs", "-type", "f"]
    );
    println!("✓ tt -- find . -name '*.rs' -type f parsed correctly");

    // Test case: tt -- grep -r "test" . --include="*.rs"
    let result = test_argument_parsing(vec!["--", "grep", "-r", "test", ".", "--include=*.rs"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(
        result.files,
        vec!["--", "grep", "-r", "test", ".", "--include=*.rs"]
    );
    println!("✓ tt -- grep -r 'test' . --include='*.rs' parsed correctly");
}

#[test]
fn test_file_args_without_separator_parsing() {
    println!("Testing file arguments without -- separator parsing");

    // Test case: tt README.md
    let result = test_argument_parsing(vec!["README.md"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(result.files, vec!["README.md"]);
    println!("✓ tt README.md parsed correctly");
}

#[test]
fn test_exclamation_command_syntax_parsing() {
    println!("Testing ! command syntax parsing");

    // Test case: tt !echo hello
    let result = test_argument_parsing(vec!["!echo", "hello"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(result.files, vec!["!echo", "hello"]);
    println!("✓ tt !echo hello parsed correctly");
}

#[test]
fn test_stdin_handling_parsing() {
    println!("Testing stdin handling parsing");

    // Test case: tt -
    let result = test_argument_parsing(vec!["-"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(result.files, vec!["-"]);
    println!("✓ tt - parsed correctly");
}

#[test]
fn test_error_handling_parsing() {
    println!("Testing error handling for malformed arguments parsing");

    // Test case: tt -- (empty command after --)
    let result = test_argument_parsing(vec!["--"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(result.files, vec!["--"]);
    println!("✓ tt -- parsed correctly (empty command)");

    // Test case: tt -- nonexistentcommand
    let result = test_argument_parsing(vec!["--", "nonexistentcommand"]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(result.files, vec!["--", "nonexistentcommand"]);
    println!("✓ tt -- nonexistentcommand parsed correctly");
}

#[test]
fn test_combined_arguments_parsing() {
    println!("Testing combined arguments parsing");

    // Test case: tt --rule logfmt --lua script.lua -- echo hello
    let result = test_argument_parsing(vec![
        "--rule",
        "logfmt",
        "--lua",
        "script.lua",
        "--",
        "echo",
        "hello",
    ]);
    assert_eq!(result.rule, Some("logfmt".to_string()));
    assert_eq!(result.lua_script, Some("script.lua".to_string()));
    assert_eq!(result.files, vec!["--", "echo", "hello"]);
    println!("✓ Combined arguments parsed correctly");
}

#[test]
fn test_no_arguments_parsing() {
    println!("Testing no arguments parsing");

    // Test case: tt (no arguments)
    let result = test_argument_parsing(vec![]);
    assert_eq!(result.rule, None);
    assert_eq!(result.lua_script, None);
    assert_eq!(result.files, Vec::<String>::new());
    println!("✓ tt with no arguments parsed correctly");
}
