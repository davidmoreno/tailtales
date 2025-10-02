use tailtales::lua_console::ConsoleLine;
use tailtales::state::TuiState;

// Helper function to extract text from ConsoleLine
fn get_console_text(console_line: &ConsoleLine) -> &str {
    match console_line {
        ConsoleLine::Stdout(msg) => msg,
        ConsoleLine::Stderr(msg) => msg,
    }
}

#[test]
fn test_lua_console_initialization_consistency() {
    // Test that ensure_lua_console_initialized works correctly
    let mut state = TuiState::new().unwrap();
    
    // Initially, the console should be empty
    assert!(state.lua_console.output_history.is_empty());
    
    // Call ensure_lua_console_initialized
    state.ensure_lua_console_initialized();
    
    // Now it should have the welcome message
    assert!(!state.lua_console.output_history.is_empty());
    assert!(get_console_text(&state.lua_console.output_history[0]).contains("Welcome to Lua REPL!"));
    assert!(get_console_text(&state.lua_console.output_history[1]).contains("Supports multiline input"));
    assert!(get_console_text(&state.lua_console.output_history[2]).contains("Use print() to output text"));
    assert!(get_console_text(&state.lua_console.output_history[3]).contains("Press Esc to exit"));
    assert!(get_console_text(&state.lua_console.output_history[4]).contains("Use ↑/↓ arrows"));
    assert!(get_console_text(&state.lua_console.output_history[5]).contains("Press Tab for function"));
    assert_eq!(get_console_text(&state.lua_console.output_history[6]), ""); // Empty line separator
}

#[test]
fn test_lua_console_initialization_idempotent() {
    // Test that calling ensure_lua_console_initialized multiple times doesn't duplicate messages
    let mut state = TuiState::new().unwrap();
    
    // Call it multiple times
    state.ensure_lua_console_initialized();
    let first_count = state.lua_console.output_history.len();
    
    state.ensure_lua_console_initialized();
    let second_count = state.lua_console.output_history.len();
    
    // Should be the same count (idempotent)
    assert_eq!(first_count, second_count);
}

#[test]
fn test_lua_console_initialization_with_existing_content() {
    // Test that ensure_lua_console_initialized doesn't add welcome message if console already has content
    let mut state = TuiState::new().unwrap();
    
    // Add some content first
    state.lua_console.add_output("Some existing content".to_string(), state.visible_width);
    let initial_count = state.lua_console.output_history.len();
    
    // Call ensure_lua_console_initialized
    state.ensure_lua_console_initialized();
    let final_count = state.lua_console.output_history.len();
    
    // Should not have added anything
    assert_eq!(initial_count, final_count);
    assert_eq!(get_console_text(&state.lua_console.output_history[0]), "Some existing content");
}

#[test]
fn test_lua_console_initialization_via_set_mode() {
    // Test that set_mode("lua_repl") also initializes the console
    let mut state = TuiState::new().unwrap();
    
    // Initially empty
    assert!(state.lua_console.output_history.is_empty());
    
    // Set mode to lua_repl
    state.set_mode("lua_repl");
    
    // Should now have welcome message
    assert!(!state.lua_console.output_history.is_empty());
    assert!(get_console_text(&state.lua_console.output_history[0]).contains("Welcome to Lua REPL!"));
}
