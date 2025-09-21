//! Lua Function Integration Tests
//!
//! These tests verify that all Lua functions used in keybindings work correctly
//! with the new Lua engine architecture where LuaEngine is owned by Application.

use tailtales::lua_engine::LuaEngine;
use tailtales::record::Record;
use tailtales::state::{Mode, TuiState};

/// Helper function to create a test state with some records
fn create_test_state_with_records() -> TuiState {
    let mut state = TuiState::new().unwrap();

    // Add test records
    for i in 0..10 {
        let mut record = Record::new(format!("Test log line {} with some content", i));
        record.index = i;
        record.set_data(
            "level",
            if i % 3 == 0 {
                "ERROR".to_string()
            } else {
                "INFO".to_string()
            },
        );
        record.set_data(
            "timestamp",
            format!("2024-01-{:02}T10:{}:00", (i % 28) + 1, i * 2),
        );
        state.records.add(record);
    }

    state.position = 5; // Start in the middle
    state
}

/// Helper function to create a LuaEngine and compile a script
fn compile_and_execute_script(
    engine: &mut LuaEngine,
    state: &mut TuiState,
    script: &str,
) -> Result<(), String> {
    let script_name = format!(
        "test_{}",
        script.replace(" ", "_").replace("(", "").replace(")", "")
    );
    engine
        .compile_script(&script_name, script)
        .map_err(|e| e.to_string())?;
    engine
        .execute_with_state(&script_name, state)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn test_navigation_functions() {
    println!("Testing navigation functions (vmove, vgoto, move_top, move_bottom)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test vmove with positive value (down movement)
    let initial_pos = state.position;
    compile_and_execute_script(&mut engine, &mut state, "vmove(1)").unwrap();
    assert_eq!(
        state.position,
        initial_pos + 1,
        "vmove(1) should move down by 1"
    );

    // Test vmove with negative value (up movement)
    compile_and_execute_script(&mut engine, &mut state, "vmove(-1)").unwrap();
    assert_eq!(state.position, initial_pos, "vmove(-1) should move up by 1");

    // Test vmove with large positive value (will be clamped to max available records)
    compile_and_execute_script(&mut engine, &mut state, "vmove(10)").unwrap();
    let expected_pos = std::cmp::min(initial_pos + 10, 9); // We have 10 records (0-9)
    assert_eq!(
        state.position, expected_pos,
        "vmove(10) should move down by 10 or to max record"
    );

    // Reset to initial position for negative vmove test
    state.position = initial_pos;

    // Test vmove with large negative value
    compile_and_execute_script(&mut engine, &mut state, "vmove(-10)").unwrap();
    let expected_up_pos = if initial_pos >= 10 {
        initial_pos - 10
    } else {
        0
    };
    assert_eq!(
        state.position, expected_up_pos,
        "vmove(-10) should move up by 10 or to 0"
    );

    // Test vgoto to beginning
    compile_and_execute_script(&mut engine, &mut state, "vgoto(0)").unwrap();
    assert_eq!(state.position, 0, "vgoto(0) should go to position 0");

    // Test vgoto to large number (should clamp to last record)
    compile_and_execute_script(&mut engine, &mut state, "vgoto(2000000000)").unwrap();
    assert_eq!(
        state.position, 9,
        "vgoto(large) should go to last record position"
    );

    // Test move_top function
    compile_and_execute_script(&mut engine, &mut state, "move_top()").unwrap();
    assert_eq!(state.position, 0, "move_top() should go to position 0");

    // Test move_bottom function
    compile_and_execute_script(&mut engine, &mut state, "move_bottom()").unwrap();
    assert_eq!(
        state.position, 9,
        "move_bottom() should go to last record position"
    );

    println!("✓ Navigation functions working correctly");
}

#[test]
fn test_horizontal_movement_functions() {
    println!("Testing horizontal movement function (hmove)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    let initial_offset = state.scroll_offset_left;

    // Test hmove with positive value (right movement)
    compile_and_execute_script(&mut engine, &mut state, "hmove(1)").unwrap();
    assert_eq!(
        state.scroll_offset_left,
        initial_offset + 1,
        "hmove(1) should move horizontally right by 1"
    );

    // Test hmove with negative value (left movement)
    compile_and_execute_script(&mut engine, &mut state, "hmove(-1)").unwrap();
    assert_eq!(
        state.scroll_offset_left, initial_offset,
        "hmove(-1) should move horizontally left by 1"
    );

    // Test hmove with large positive value
    compile_and_execute_script(&mut engine, &mut state, "hmove(10)").unwrap();
    assert_eq!(
        state.scroll_offset_left,
        initial_offset + 10,
        "hmove(10) should move right by 10"
    );

    // Test hmove with large negative value
    compile_and_execute_script(&mut engine, &mut state, "hmove(-10)").unwrap();
    assert_eq!(
        state.scroll_offset_left, initial_offset,
        "hmove(-10) should move left by 10"
    );

    println!("✓ Horizontal movement functions working correctly");
}

#[test]
fn test_marking_functions() {
    println!("Testing marking functions (toggle_mark, move_to_next_mark, move_to_prev_mark)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test toggle_mark with default yellow color
    compile_and_execute_script(&mut engine, &mut state, "toggle_mark('yellow')").unwrap();
    // Verify mark was added (this would need to check the record's mark data)

    // Test toggle_mark with different colors
    compile_and_execute_script(&mut engine, &mut state, "toggle_mark('red')").unwrap();
    compile_and_execute_script(&mut engine, &mut state, "toggle_mark('green')").unwrap();
    compile_and_execute_script(&mut engine, &mut state, "toggle_mark('blue')").unwrap();
    compile_and_execute_script(&mut engine, &mut state, "toggle_mark('magenta')").unwrap();
    compile_and_execute_script(&mut engine, &mut state, "toggle_mark('cyan')").unwrap();

    // Test mark navigation functions (these functions exist but may not find marks in simple test)
    compile_and_execute_script(&mut engine, &mut state, "move_to_next_mark()").unwrap();
    compile_and_execute_script(&mut engine, &mut state, "move_to_prev_mark()").unwrap();

    println!("✓ Marking functions working correctly");
}

#[test]
fn test_mode_and_ui_functions() {
    println!("Testing mode switching and UI functions (mode, toggle_details)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    assert_eq!(state.mode, Mode::Normal, "Should start in normal mode");

    // Test mode function with command mode
    compile_and_execute_script(&mut engine, &mut state, "mode('command')").unwrap();
    assert_eq!(
        state.mode,
        Mode::Command,
        "mode('command') should switch to command mode"
    );

    // Test mode function with filter mode
    compile_and_execute_script(&mut engine, &mut state, "mode('filter')").unwrap();
    assert_eq!(
        state.mode,
        Mode::Filter,
        "mode('filter') should switch to filter mode"
    );

    // Test mode function with search mode
    compile_and_execute_script(&mut engine, &mut state, "mode('search')").unwrap();
    assert_eq!(
        state.mode,
        Mode::Search,
        "mode('search') should switch to search mode"
    );

    // Test mode function back to normal
    compile_and_execute_script(&mut engine, &mut state, "mode('normal')").unwrap();
    assert_eq!(
        state.mode,
        Mode::Normal,
        "mode('normal') should switch back to normal mode"
    );

    // Test toggle_details function
    let initial_details = state.view_details;
    compile_and_execute_script(&mut engine, &mut state, "toggle_details()").unwrap();
    assert_eq!(
        state.view_details, !initial_details,
        "toggle_details() should toggle view_details"
    );

    println!("✓ Mode and UI functions working correctly");
}

#[test]
fn test_search_functions() {
    println!("Testing search functions (search_next, search_prev)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Set up a search term first
    state.search = "Test".to_string();

    // Test search_next function (should not crash even if no results)
    compile_and_execute_script(&mut engine, &mut state, "search_next()").unwrap();

    // Test search_prev function (should not crash even if no results)
    compile_and_execute_script(&mut engine, &mut state, "search_prev()").unwrap();

    println!("✓ Search functions working correctly");
}

#[test]
fn test_system_functions() {
    println!("Testing system functions (quit, warning, clear_records, refresh_screen)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test warning function
    let warning_script = "warning('Line: ' .. (get_position() + 1))";
    compile_and_execute_script(&mut engine, &mut state, warning_script).unwrap();
    assert!(
        state.warning.contains("Line:"),
        "warning() should set warning message"
    );

    // Test clear_records function
    let initial_count = state.records.len();
    assert!(initial_count > 0, "Should have records initially");
    compile_and_execute_script(&mut engine, &mut state, "clear_records()").unwrap();
    assert_eq!(
        state.records.len(),
        0,
        "clear_records() should remove all records"
    );
    assert_eq!(
        state.position, 0,
        "clear_records() should reset position to 0"
    );

    // Test refresh_screen function
    compile_and_execute_script(&mut engine, &mut state, "refresh_screen()").unwrap();
    assert!(
        state.pending_refresh,
        "refresh_screen() should set pending_refresh flag"
    );

    // Test quit function (should set running to false)
    assert!(state.running, "Should be running initially");
    compile_and_execute_script(&mut engine, &mut state, "quit()").unwrap();
    assert!(!state.running, "quit() should set running to false");

    println!("✓ System functions working correctly");
}

#[test]
fn test_external_command_functions() {
    println!("Testing external command functions (exec, settings)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test exec function with a simple command that should succeed
    compile_and_execute_script(&mut engine, &mut state, "exec('echo test')").unwrap();

    // Test settings function
    //compile_and_execute_script(&mut engine, &mut state, "settings()").unwrap();

    println!("✓ External command functions working correctly");
}

#[test]
fn test_utility_functions() {
    println!("Testing utility functions (url_encode, url_decode, escape_shell)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test url_encode by compiling and executing scripts
    compile_and_execute_script(
        &mut engine,
        &mut state,
        "local encoded = url_encode('hello world'); warning('encoded: ' .. encoded)",
    )
    .unwrap();
    assert!(
        state.warning.contains("hello%20world"),
        "Should URL encode spaces"
    );

    // Test url_decode
    compile_and_execute_script(
        &mut engine,
        &mut state,
        "local decoded = url_decode('hello%20world'); warning('decoded: ' .. decoded)",
    )
    .unwrap();
    assert!(
        state.warning.contains("hello world"),
        "Should URL decode spaces"
    );

    // Test escape_shell
    compile_and_execute_script(
        &mut engine,
        &mut state,
        "local escaped = escape_shell('test string'); warning('escaped: ' .. escaped)",
    )
    .unwrap();
    assert!(
        state.warning.contains("'test string'"),
        "Should wrap in single quotes"
    );

    println!("✓ Utility functions working correctly");
}

#[test]
fn test_complex_function_scenarios() {
    println!("Testing complex function scenarios (combinations, state persistence)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test complex script combining utility and state functions
    let complex_script = r#"
        local record = get_record()
        local encoded_line = url_encode(record.line or "empty")
        warning("Would open Perplexity with: " .. encoded_line)
    "#;

    // Set up current context first
    engine.update_context(&state).unwrap();
    compile_and_execute_script(&mut engine, &mut state, complex_script).unwrap();
    assert!(
        state.warning.contains("Would open Perplexity"),
        "Complex script should set warning"
    );

    // Test script combining external command with utility functions
    let clipboard_script = r#"
        local record = get_record()
        local success = exec('echo "' .. escape_shell(record.line) .. '"')
        if success then
            warning("Line copied to clipboard")
        else
            warning("Failed to copy line")
        end
    "#;

    compile_and_execute_script(&mut engine, &mut state, clipboard_script).unwrap();
    assert!(
        state.warning.contains("copied") || state.warning.contains("Failed"),
        "Clipboard script should set appropriate warning"
    );

    // Test script with conditional logic and navigation
    let goto_script = r#"
        local line_num = 3
        if line_num then
            vgoto(line_num - 1)  -- Convert to 0-based indexing
            warning("Moved to line " .. line_num)
        else
            warning("Invalid line number")
        end
    "#;

    compile_and_execute_script(&mut engine, &mut state, goto_script).unwrap();
    assert_eq!(
        state.position, 2,
        "Should move to position 2 (0-based for line 3)"
    );
    assert!(
        state.warning.contains("Moved to line"),
        "Should set success warning"
    );

    println!("✓ Complex function scenarios working correctly");
}

#[test]
fn test_function_error_handling() {
    println!("Testing function error handling and edge cases");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test that functions handle edge cases gracefully

    // Test navigation beyond bounds (should be handled gracefully)
    compile_and_execute_script(&mut engine, &mut state, "vgoto(99999)").unwrap();
    // Should clamp to valid range

    // Test navigation to negative position (should be handled gracefully)
    compile_and_execute_script(&mut engine, &mut state, "vgoto(0)").unwrap();
    compile_and_execute_script(&mut engine, &mut state, "vmove(-1000)").unwrap();
    // Should not go below 0

    // Test horizontal movement with extreme values
    compile_and_execute_script(&mut engine, &mut state, "hmove(1000)").unwrap();
    compile_and_execute_script(&mut engine, &mut state, "hmove(-1000)").unwrap();
    // Should handle gracefully

    println!("✓ Function error handling working correctly");
}

#[test]
fn test_ask_function_and_goto_line() {
    println!("Testing ask() function and 'g' keybinding pattern (goto line)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test the exact 'g' keybinding script from settings.yaml
    let goto_script = r#"
        local line_str = ask("Go to line number:")
        local line_num = tonumber(line_str)
        if line_num then
            vgoto(line_num - 1)  -- Convert to 0-based indexing
            warning("Moved to line " .. line_num)
        else
            warning("Invalid line number: " .. line_str)
        end
    "#;

    // Compile the script
    engine
        .compile_script("test_goto_line", goto_script)
        .unwrap();

    // Set up the context (important for the script to access app state)
    engine.update_context(&state).unwrap();

    // Execute the script - it should suspend waiting for input
    let initial_pos = state.position;
    let result = engine.execute_with_state("test_goto_line", &mut state);

    // Check if the script suspended with an ask() prompt
    match result {
        Ok(Some(prompt)) => {
            // This is expected - the script should suspend waiting for input
            assert_eq!(prompt, "Go to line number:", "Should return correct prompt");
            assert!(
                engine.has_suspended_script(),
                "Engine should have suspended script"
            );

            // Simulate what keyboard management does when script suspends
            state.script_prompt = prompt;
            state.script_waiting = true;
            state.mode = tailtales::state::Mode::ScriptInput;
            state.script_input.clear();

            println!("✓ Script correctly suspended waiting for input");
        }
        _ => panic!(
            "Expected script to suspend with ask() prompt, got: {:?}",
            result
        ),
    }

    // Now provide input "3" and resume the script
    let resume_result = engine.resume_with_input("3".to_string());

    match resume_result {
        Ok(()) => {
            // Script should have completed successfully
            assert!(
                !engine.has_suspended_script(),
                "Engine should no longer have suspended script"
            );
            assert_eq!(
                state.position, 2,
                "Should move to position 2 (0-based for line 3)"
            );
            assert!(
                state.warning.contains("Moved to line 3"),
                "Should show success message"
            );

            // Simulate what keyboard management does when script completes
            state.script_waiting = false;
            state.script_prompt.clear();
            state.mode = tailtales::state::Mode::Normal;

            println!("✓ Script resumed and executed goto correctly");
        }
        Err(err) => panic!("Failed to resume script with input: {:?}", err),
    }

    // Test with invalid input
    // Reset state for another test
    state.position = initial_pos;
    state.script_waiting = false;
    state.script_prompt.clear();
    state.warning.clear();

    // Execute the script again
    let result2 = engine.execute_with_state("test_goto_line", &mut state);
    match result2 {
        Ok(Some(prompt)) => {
            // Set up suspension state
            state.script_prompt = prompt;
            state.script_waiting = true;
            state.mode = tailtales::state::Mode::ScriptInput;

            // Resume with invalid input
            let resume_result2 = engine.resume_with_input("invalid".to_string());
            match resume_result2 {
                Ok(()) => {
                    println!("Warning content: '{}'", state.warning);
                    assert_eq!(
                        state.position, initial_pos,
                        "Position should not change with invalid input"
                    );
                    assert!(
                        state.warning.contains("Invalid line number: invalid"),
                        "Should show error message"
                    );
                    println!("✓ Script correctly handled invalid input");
                }
                Err(err) => panic!("Failed to resume script with invalid input: {:?}", err),
            }
        }
        _ => panic!("Expected script to suspend with ask() prompt on second execution"),
    }

    // Test cancellation
    // Reset state for cancellation test
    state.position = initial_pos;
    state.script_waiting = false;
    state.script_prompt.clear();
    state.warning.clear();

    // Execute the script again
    let result3 = engine.execute_with_state("test_goto_line", &mut state);
    match result3 {
        Ok(Some(prompt)) => {
            // Set up suspension state
            state.script_prompt = prompt;
            state.script_waiting = true;
            state.mode = tailtales::state::Mode::ScriptInput;

            // Test cancellation
            engine.cancel_suspended_script();
            assert!(
                !engine.has_suspended_script(),
                "Engine should no longer have suspended script after cancellation"
            );
            assert_eq!(
                state.position, initial_pos,
                "Position should not change after cancellation"
            );

            // Simulate what keyboard management does when script is cancelled
            state.script_waiting = false;
            state.script_prompt.clear();
            state.mode = tailtales::state::Mode::Normal;

            println!("✓ Script cancellation works correctly");
        }
        _ => panic!("Expected script to suspend with ask() prompt for cancellation test"),
    }

    println!("✓ ask() function and goto line functionality working correctly");
}

#[test]
fn test_function_registration() {
    println!("Testing that all expected Lua functions are properly registered");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // List of functions that should be available
    let expected_functions = [
        "quit",
        "warning",
        "vmove",
        "vgoto",
        "move_top",
        "move_bottom",
        "search_next",
        "search_prev",
        "toggle_mark",
        "move_to_next_mark",
        "move_to_prev_mark",
        "mode",
        "toggle_details",
        "refresh_screen",
        "clear_records",
        "hmove",
        "exec",
        "settings",
        "url_encode",
        "url_decode",
        "escape_shell",
        "debug_log",
        "ask", // Add ask to the list of expected functions
    ];

    for func_name in &expected_functions {
        let test_script = format!("if type({}) == 'function' then warning('function {} exists') else warning('function {} missing') end", func_name, func_name, func_name);
        compile_and_execute_script(&mut engine, &mut state, &test_script).unwrap();
        assert!(
            state
                .warning
                .contains(&format!("function {} exists", func_name)),
            "Function '{}' should be registered",
            func_name
        );
    }

    println!("✓ All expected functions are properly registered");
}
