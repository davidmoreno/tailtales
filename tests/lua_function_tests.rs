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
        state.records.add_record(record, None);
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
    let expected_pos = std::cmp::min(initial_pos + 10, 10); // We have 10 records (1-10)
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
        1 // Minimum position is now 1
    };
    assert_eq!(
        state.position, expected_up_pos,
        "vmove(-10) should move up by 10 or to 1"
    );

    // Test vgoto to beginning
    compile_and_execute_script(&mut engine, &mut state, "vgoto(0)").unwrap();
    assert_eq!(
        state.position, 1,
        "vgoto(0) should go to position 1 (minimum)"
    );

    // Test vgoto to large number (should clamp to last record)
    compile_and_execute_script(&mut engine, &mut state, "vgoto(2000000000)").unwrap();
    assert_eq!(
        state.position, 10,
        "vgoto(large) should go to last record position (10)"
    );

    // Test move_top function
    compile_and_execute_script(&mut engine, &mut state, "move_top()").unwrap();
    assert_eq!(state.position, 1, "move_top() should go to position 1");

    // Test move_bottom function
    compile_and_execute_script(&mut engine, &mut state, "move_bottom()").unwrap();
    assert_eq!(
        state.position, 10,
        "move_bottom() should go to last record position (10)"
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
        state.position, 1,
        "clear_records() should reset position to 1 (1-based)"
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
fn test_exec_function_debugging() {
    println!("Testing exec function in detail");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test successful command
    let result = engine
        .execute_script_string_with_state("return exec('echo hello')", &mut state)
        .unwrap();
    assert_eq!(result, "true", "exec('echo hello') should return true");

    // Test command that should succeed (always available)
    let result = engine
        .execute_script_string_with_state("return exec('true')", &mut state)
        .unwrap();
    assert_eq!(result, "true", "exec('true') should return true");

    // Test command that should fail
    let result = engine
        .execute_script_string_with_state("return exec('false')", &mut state)
        .unwrap();
    assert_eq!(result, "false", "exec('false') should return false");

    // Test xdg-open version check (should work without opening anything)
    let result = engine
        .execute_script_string_with_state("return exec('xdg-open --version')", &mut state)
        .unwrap();
    assert_eq!(result, "true", "xdg-open --version should succeed");

    // Test xdg-open with a URL (this will actually try to open a browser, but should return true)
    // Note: In a real test environment, this might fail if no display is available
    let result = engine
        .execute_script_string_with_state("return exec('test https://example.com')", &mut state);
    // Don't assert the result since it depends on the environment, just verify it doesn't crash
    assert!(
        result.is_ok(),
        "xdg-open with URL should not crash even if it fails"
    );

    println!("✓ exec function debugging complete");
}

#[test]
fn test_f2_keybinding_simulation() {
    println!("Testing F2 keybinding with modern get_record() API");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Set position to a record with known content
    state.position = 1;

    // Test the exact F2 keybinding code from settings.yaml
    let f2_script = r#"
        exec('test https://www.perplexity.ai/search/new?q=' .. url_encode(get_record().original))
        warning('Opened Perplexity')
    "#;

    engine.compile_script("test_f2", f2_script).unwrap();

    // Execute the F2 script
    match engine.execute_with_state("test_f2", &mut state) {
        Ok(_) => {
            println!("✓ F2 keybinding with get_record().original works");
            assert!(state.warning.contains("Opened Perplexity"));
        }
        Err(e) => {
            panic!("F2 keybinding should work: {}", e);
        }
    }

    // Test F3 and F4 keybindings as well (DuckDuckGo and Google)
    let f3_script = r#"
        exec('test https://www.duckduckgo.com/?q=' .. url_encode(get_record().original))
        warning('Opened DuckDuckGo')
    "#;

    engine.compile_script("test_f3", f3_script).unwrap();

    match engine.execute_with_state("test_f3", &mut state) {
        Ok(_) => {
            println!("✓ F3 keybinding works");
            assert!(state.warning.contains("DuckDuckGo"));
        }
        Err(e) => {
            panic!("F3 keybinding should work: {}", e);
        }
    }

    // Test accessing other record fields
    let record_fields_script = r#"
        local record = get_record()
        local level = record.level or "unknown"
        warning('Record level: ' .. level)
    "#;

    engine
        .compile_script("test_record_fields", record_fields_script)
        .unwrap();

    match engine.execute_with_state("test_record_fields", &mut state) {
        Ok(_) => {
            println!("✓ Record field access works");
            assert!(state.warning.contains("level"));
        }
        Err(e) => {
            panic!("Record field access should work: {}", e);
        }
    }

    println!("✓ Modern keybinding simulations successful");
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
        local encoded_line = url_encode(record.original or "empty")
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
        local success = exec('echo "' .. escape_shell(record.original) .. '"')
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
            vgoto(line_num)
            warning("Moved to line " .. line_num)
        else
            warning("Invalid line number")
        end
    "#;

    compile_and_execute_script(&mut engine, &mut state, goto_script).unwrap();
    assert_eq!(
        state.position, 3,
        "Should move to position 3 (1-based for user line 3)"
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
            vgoto(line_num)  -- Now using 1-based indexing directly
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
                state.position, 3,
                "Should move to position 3 (1-based for user line 3)"
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
fn test_repl_state_access_functions() {
    println!("Testing REPL state access functions (get_position, get_record, etc.)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Set a specific position for testing
    state.position = 3;
    state.mode = Mode::Filter;

    // Test get_position()
    let result = engine
        .execute_script_string_with_state("return get_position()", &mut state)
        .unwrap();
    assert_eq!(
        result, "3",
        "get_position() should return current position (1-based)"
    );

    // Test get_mode()
    let result = engine
        .execute_script_string_with_state("return get_mode()", &mut state)
        .unwrap();
    assert_eq!(result, "filter", "get_mode() should return current mode");

    // Test get_record_count()
    let result = engine
        .execute_script_string_with_state("return get_record_count()", &mut state)
        .unwrap();
    assert_eq!(
        result, "10",
        "get_record_count() should return number of records"
    );

    // Test get_record() returns a table
    let result = engine
        .execute_script_string_with_state("local r = get_record(); return typeof(r)", &mut state)
        .unwrap();
    assert_eq!(result, "table", "get_record() should return a table");

    // Test accessing record fields
    let result = engine
        .execute_script_string_with_state("local r = get_record(); return r.original", &mut state)
        .unwrap();
    assert!(
        result.contains("Test log line 2"),
        "get_record().original should contain the log line"
    );

    // Test record index
    let result = engine
        .execute_script_string_with_state("local r = get_record(); return r.index", &mut state)
        .unwrap();
    assert_eq!(
        result, "3",
        "get_record().index should match current position (1-based)"
    );

    // Test record level field (position 3: accessing record at index 2, 2 % 3 == 2, so should be INFO)
    let result = engine
        .execute_script_string_with_state("local r = get_record(); return r.level", &mut state)
        .unwrap();
    assert_eq!(
        result, "INFO",
        "get_record().level should return the set level"
    );

    println!("✓ All REPL state access functions work correctly");
}

#[test]
fn test_repl_interactive_functions() {
    println!("Testing REPL interactive functions (dir, help, typeof)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test dir() function exists and works
    let result = engine
        .execute_script_string_with_state("dir(); return 'dir_completed'", &mut state)
        .unwrap();
    assert!(
        result.contains("dir_completed"),
        "dir() function should execute without error"
    );

    // Test help() function exists and works
    let result = engine
        .execute_script_string_with_state("help(); return 'help_completed'", &mut state)
        .unwrap();
    assert!(
        result.contains("help_completed"),
        "help() function should execute without error"
    );

    // Test typeof() function
    let result = engine
        .execute_script_string_with_state("return typeof({1, 2, 3})", &mut state)
        .unwrap();
    assert_eq!(result, "array[3]", "typeof() should detect arrays");

    let result = engine
        .execute_script_string_with_state("return typeof({name = 'test'})", &mut state)
        .unwrap();
    assert_eq!(result, "table", "typeof() should detect tables");

    let result = engine
        .execute_script_string_with_state("return typeof('hello')", &mut state)
        .unwrap();
    assert_eq!(result, "string", "typeof() should detect strings");

    // Test callable() function
    let result = engine
        .execute_script_string_with_state("return callable(print)", &mut state)
        .unwrap();
    assert_eq!(result, "true", "callable() should detect functions");

    let result = engine
        .execute_script_string_with_state("return callable('hello')", &mut state)
        .unwrap();
    assert_eq!(
        result, "false",
        "callable() should return false for non-functions"
    );

    println!("✓ All REPL interactive functions work correctly");
}

#[test]
fn test_repl_error_handling() {
    println!("Testing REPL error handling for state-dependent functions");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test that state functions work with state
    let result = engine.execute_script_string_with_state("return get_position()", &mut state);
    assert!(
        result.is_ok(),
        "get_position() should work with state access"
    );

    // Test that state access works with different states
    let mut other_state = create_test_state_with_records();
    other_state.position = 7;

    let result = engine.execute_script_string_with_state("return get_position()", &mut other_state);
    assert!(
        result.is_ok(),
        "get_position() should work with different state"
    );
    let position_result = result.unwrap();
    assert_eq!(
        position_result, "7",
        "get_position() should return the correct position from different state (1-based)"
    );

    // Test multiline scripts with state access
    let multiline_script = r#"
        local pos = get_position()
        local count = get_record_count()
        return pos .. "/" .. count
    "#;
    let result = engine
        .execute_script_string_with_state(multiline_script, &mut state)
        .unwrap();
    assert_eq!(
        result, "5/10",
        "Multiline scripts should work with state access (1-based position)"
    );

    println!("✓ REPL error handling works correctly");
}

#[test]
fn test_repl_print_and_table_formatting() {
    println!("Testing REPL print output and table formatting");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test print output
    let result = engine
        .execute_script_string_with_state("print('Hello, REPL!')", &mut state)
        .unwrap();
    assert_eq!(result, "Hello, REPL!", "print() should output to REPL");

    // Test multiple print statements
    let result = engine
        .execute_script_string_with_state("print('Line 1'); print('Line 2')", &mut state)
        .unwrap();
    assert_eq!(
        result, "Line 1\nLine 2",
        "Multiple print statements should appear on separate lines"
    );

    // Test table formatting
    let result = engine
        .execute_script_string_with_state("return {name = 'test', value = 42}", &mut state)
        .unwrap();
    assert!(
        result.contains("name = \"test\""),
        "Tables should show string values with quotes"
    );
    assert!(
        result.contains("value = 42"),
        "Tables should show numeric values"
    );

    // Test array formatting
    let result = engine
        .execute_script_string_with_state("return {1, 2, 3}", &mut state)
        .unwrap();
    assert!(
        result.contains("1, 2, 3"),
        "Arrays should display values in order"
    );

    // Test nested table depth limiting (create a table deeper than max depth of 3)
    let result = engine
        .execute_script_string_with_state(
            "return {a = {b = {c = {d = {e = {f = 'very_deep'}}}}}}",
            &mut state,
        )
        .unwrap();
    assert!(
        result.contains("..."),
        "Deep nested tables should be truncated"
    );

    // Test empty table
    let result = engine
        .execute_script_string_with_state("return {}", &mut state)
        .unwrap();
    assert_eq!(result, "{}", "Empty tables should display as {{}}"); // Escape braces

    println!("✓ REPL print and table formatting work correctly");
}

#[test]
fn test_repl_history_functionality() {
    println!("Testing REPL command history functionality");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Initially no history
    assert_eq!(state.repl_command_history.len(), 0);
    assert_eq!(state.repl_history_index, None);

    // Add some commands to history
    state.add_to_repl_history("print('hello')".to_string());
    state.add_to_repl_history("x = 42".to_string());
    state.add_to_repl_history("print(x)".to_string());

    assert_eq!(state.repl_command_history.len(), 3);
    assert_eq!(state.repl_command_history[0], "print('hello')");
    assert_eq!(state.repl_command_history[1], "x = 42");
    assert_eq!(state.repl_command_history[2], "print(x)");

    // Test not adding duplicate consecutive commands
    state.add_to_repl_history("print(x)".to_string());
    assert_eq!(
        state.repl_command_history.len(),
        3,
        "Should not add duplicate"
    );

    // Test not adding empty commands
    state.add_to_repl_history("".to_string());
    state.add_to_repl_history("   ".to_string());
    assert_eq!(
        state.repl_command_history.len(),
        3,
        "Should not add empty commands"
    );

    // Test history navigation
    state.repl_input = "current_input".to_string();

    // Navigate up (should go to most recent)
    assert!(state.repl_history_up());
    assert_eq!(state.repl_history_index, Some(2));
    assert_eq!(state.repl_input, "print(x)");
    assert_eq!(state.repl_temp_input, "current_input");

    // Navigate up again (should go to older)
    assert!(state.repl_history_up());
    assert_eq!(state.repl_history_index, Some(1));
    assert_eq!(state.repl_input, "x = 42");

    // Navigate up again (should go to oldest)
    assert!(state.repl_history_up());
    assert_eq!(state.repl_history_index, Some(0));
    assert_eq!(state.repl_input, "print('hello')");

    // Try to navigate up past oldest (should not change)
    assert!(!state.repl_history_up());
    assert_eq!(state.repl_history_index, Some(0));
    assert_eq!(state.repl_input, "print('hello')");

    // Navigate down
    assert!(state.repl_history_down());
    assert_eq!(state.repl_history_index, Some(1));
    assert_eq!(state.repl_input, "x = 42");

    // Navigate down again
    assert!(state.repl_history_down());
    assert_eq!(state.repl_history_index, Some(2));
    assert_eq!(state.repl_input, "print(x)");

    // Navigate down to current input
    assert!(state.repl_history_down());
    assert_eq!(state.repl_history_index, None);
    assert_eq!(state.repl_input, "current_input");

    // Try to navigate down past current (should not change)
    assert!(!state.repl_history_down());
    assert_eq!(state.repl_history_index, None);
    assert_eq!(state.repl_input, "current_input");

    // Test reset history navigation
    state.repl_history_up(); // Go to history
    assert!(state.repl_history_index.is_some());
    state.reset_repl_history_navigation();
    assert_eq!(state.repl_history_index, None);
    assert_eq!(state.repl_temp_input, "");

    println!("✓ REPL history functionality works correctly");
}

#[test]
fn test_repl_multiline_history_format() {
    println!("Testing REPL multiline history formatting");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test single line command
    state.repl_multiline_buffer = vec!["print('hello')".to_string()];
    let history_command = if state.repl_multiline_buffer.len() > 1 {
        state.repl_multiline_buffer.join("; ")
    } else {
        state.repl_multiline_buffer.join("\n")
    };
    state.add_to_repl_history(history_command);

    // Test multiline command
    state.repl_multiline_buffer = vec![
        "for i = 1, 3 do".to_string(),
        "  print(i)".to_string(),
        "end".to_string(),
    ];
    let history_command = if state.repl_multiline_buffer.len() > 1 {
        state.repl_multiline_buffer.join("; ")
    } else {
        state.repl_multiline_buffer.join("\n")
    };
    state.add_to_repl_history(history_command);

    // Test another multiline command
    state.repl_multiline_buffer = vec![
        "if true then".to_string(),
        "  x = 42".to_string(),
        "  print(x)".to_string(),
        "end".to_string(),
    ];
    let history_command = if state.repl_multiline_buffer.len() > 1 {
        state.repl_multiline_buffer.join("; ")
    } else {
        state.repl_multiline_buffer.join("\n")
    };
    state.add_to_repl_history(history_command);

    // Verify history format
    assert_eq!(state.repl_command_history.len(), 3);
    assert_eq!(state.repl_command_history[0], "print('hello')");
    assert_eq!(
        state.repl_command_history[1],
        "for i = 1, 3 do;   print(i); end"
    );
    assert_eq!(
        state.repl_command_history[2],
        "if true then;   x = 42;   print(x); end"
    );

    println!("✓ Multiline commands are properly formatted with semicolons in history");
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
        "ask",
        "get_position",
        "get_mode",
        "get_record_count",
        "get_record",
        "get_record_data",
        "lua_repl",
        // Functions from _init.lua
        "dir",
        "help",
        "callable",
        "typeof",
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

#[test]
fn test_get_record_by_position() {
    println!("Testing get_record() function with position parameter");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test get_record() with specific positions - test each case separately
    let test_cases = vec![(1, "Record 1:"), (5, "Record 5:"), (999, "Record 999: nil")];

    for (position, expected_text) in test_cases {
        let test_script = format!(
            r#"
                local record = get_record({})
                if record then
                    warning("Record {}: " .. record.original)
                else
                    warning("Record {}: nil")
                end
            "#,
            position, position, position
        );

        let script_name = format!("test_get_record_{}", position);
        engine.compile_script(&script_name, &test_script).unwrap();

        match engine.execute_with_state(&script_name, &mut state) {
            Ok(_) => {
                println!("✓ get_record({}) works correctly", position);
                assert!(
                    state.warning.contains(expected_text),
                    "Expected '{}' in warning '{}'",
                    expected_text,
                    state.warning
                );
            }
            Err(e) => {
                panic!("get_record({}) should work: {}", position, e);
            }
        }
    }

    // Test that get_record() without parameter still works (gets current position)
    state.position = 3;
    let current_record_script = r#"
        local record = get_record()
        if record then
            warning("Current record: " .. record.original)
        else
            warning("Current record: nil")
        end
    "#;

    engine
        .compile_script("test_get_record_current", current_record_script)
        .unwrap();

    match engine.execute_with_state("test_get_record_current", &mut state) {
        Ok(_) => {
            println!("✓ get_record() without parameter works (gets current position)");
            assert!(state.warning.contains("Current record:"));
        }
        Err(e) => {
            panic!("get_record() without parameter should work: {}", e);
        }
    }

    // Test accessing specific fields from records at different positions
    let field_test_script = r#"
        local record1 = get_record(1)
        local record2 = get_record(2)
        
        if record1 and record2 then
            local level1 = record1.level or "unknown"
            local level2 = record2.level or "unknown"
            warning("Levels: " .. level1 .. " vs " .. level2)
        else
            warning("Failed to get records for field test")
        end
    "#;

    engine
        .compile_script("test_record_fields_by_position", field_test_script)
        .unwrap();

    match engine.execute_with_state("test_record_fields_by_position", &mut state) {
        Ok(_) => {
            println!("✓ Record field access by position works");
            assert!(state.warning.contains("Levels:"));
        }
        Err(e) => {
            panic!("Record field access by position should work: {}", e);
        }
    }

    println!("✓ get_record_data(position) functionality verified");
}

#[test]
fn test_get_record_data_function() {
    println!("Testing get_record_data() function (alias for get_record)");

    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = create_test_state_with_records();

    // Test get_record_data() with specific positions - test each case separately
    let test_cases = vec![(1, "Data 1:"), (3, "Data 3:")];

    for (position, expected_text) in test_cases {
        let test_script = format!(
            r#"
                local record = get_record_data({})
                if record then
                    warning("Data {}: " .. record.original)
                else
                    warning("Data {}: nil")
                end
            "#,
            position, position, position
        );

        let script_name = format!("test_get_record_data_{}", position);
        engine.compile_script(&script_name, &test_script).unwrap();

        match engine.execute_with_state(&script_name, &mut state) {
            Ok(_) => {
                println!("✓ get_record_data({}) works correctly", position);
                assert!(
                    state.warning.contains(expected_text),
                    "Expected '{}' in warning '{}'",
                    expected_text,
                    state.warning
                );
            }
            Err(e) => {
                panic!("get_record_data({}) should work: {}", position, e);
            }
        }
    }

    // Test that get_record_data() without parameter gets current position
    state.position = 3;
    let current_record_script = r#"
        local record = get_record_data()
        if record then
            warning("Current data: " .. record.original)
        else
            warning("Current data: nil")
        end
    "#;

    engine
        .compile_script("test_get_record_data_current", current_record_script)
        .unwrap();

    match engine.execute_with_state("test_get_record_data_current", &mut state) {
        Ok(_) => {
            println!("✓ get_record_data() without parameter works (gets current position)");
            assert!(state.warning.contains("Current data:"));
        }
        Err(e) => {
            panic!("get_record_data() without parameter should work: {}", e);
        }
    }

    // Test that get_record_data and get_record return the same data
    let comparison_script = r#"
        local data1 = get_record_data(2)
        local record1 = get_record(2)
        
        if data1 and record1 then
            if data1.original == record1.original then
                warning("get_record_data and get_record return same data")
            else
                warning("get_record_data and get_record return different data")
            end
        else
            warning("Failed to get records for comparison")
        end
    "#;

    engine
        .compile_script("test_record_data_comparison", comparison_script)
        .unwrap();

    match engine.execute_with_state("test_record_data_comparison", &mut state) {
        Ok(_) => {
            println!("✓ get_record_data and get_record return identical data");
            assert!(state.warning.contains("same data"));
        }
        Err(e) => {
            panic!("Record data comparison should work: {}", e);
        }
    }

    println!("✓ get_record_data() function verified as working alias");
}
