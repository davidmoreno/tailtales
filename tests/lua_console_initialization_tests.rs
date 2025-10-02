use tailtales::state::TuiState;

#[test]
fn test_lua_console_initialization_consistency() {
    // Test that ensure_lua_console_initialized works correctly
    let mut state = TuiState::new().unwrap();
    
    // Initially, the console should be empty
    assert!(state.repl_output_history.is_empty());
    
    // Call ensure_lua_console_initialized
    state.ensure_lua_console_initialized();
    
    // Now it should have the welcome message
    assert!(!state.repl_output_history.is_empty());
    assert!(state.repl_output_history[0].contains("Welcome to Lua REPL!"));
    assert!(state.repl_output_history[1].contains("Supports multiline input"));
    assert!(state.repl_output_history[2].contains("Use print() to output text"));
    assert!(state.repl_output_history[3].contains("Press Esc to exit"));
    assert!(state.repl_output_history[4].contains("Use ↑/↓ arrows"));
    assert!(state.repl_output_history[5].contains("Press Tab for function"));
    assert_eq!(state.repl_output_history[6], ""); // Empty line separator
}

#[test]
fn test_lua_console_initialization_idempotent() {
    // Test that calling ensure_lua_console_initialized multiple times doesn't duplicate messages
    let mut state = TuiState::new().unwrap();
    
    // Call it multiple times
    state.ensure_lua_console_initialized();
    let first_count = state.repl_output_history.len();
    
    state.ensure_lua_console_initialized();
    let second_count = state.repl_output_history.len();
    
    // Should be the same count (idempotent)
    assert_eq!(first_count, second_count);
}

#[test]
fn test_lua_console_initialization_with_existing_content() {
    // Test that ensure_lua_console_initialized doesn't add welcome message if console already has content
    let mut state = TuiState::new().unwrap();
    
    // Add some content first
    state.add_to_lua_console("Some existing content".to_string());
    let initial_count = state.repl_output_history.len();
    
    // Call ensure_lua_console_initialized
    state.ensure_lua_console_initialized();
    let final_count = state.repl_output_history.len();
    
    // Should not have added anything
    assert_eq!(initial_count, final_count);
    assert_eq!(state.repl_output_history[0], "Some existing content");
}

#[test]
fn test_lua_console_initialization_via_set_mode() {
    // Test that set_mode("lua_repl") also initializes the console
    let mut state = TuiState::new().unwrap();
    
    // Initially empty
    assert!(state.repl_output_history.is_empty());
    
    // Set mode to lua_repl
    state.set_mode("lua_repl");
    
    // Should now have welcome message
    assert!(!state.repl_output_history.is_empty());
    assert!(state.repl_output_history[0].contains("Welcome to Lua REPL!"));
}
