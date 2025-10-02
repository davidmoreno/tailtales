//! Test Lua print output redirection
//!
//! This test verifies that Lua print statements from external scripts
//! are captured and redirected to the Lua console instead of going to stderr.

use tailtales::lua_engine::LuaEngine;
use tailtales::state::{ConsoleLine, TuiState};

#[test]
fn test_lua_print_output_redirection() {
    println!("Testing Lua print output redirection");

    // Create a Lua engine and state
    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = TuiState::new().unwrap();

    // Clear any existing output history
    state.repl_output_history.clear();

    // Create a test script with print statements
    let test_script = r#"
        print("This is a test print statement")
        print("Another print statement")
        print("Script executed successfully")
    "#;

    // Execute the script using the same method as external scripts
    let result = engine.execute_script_string_with_state(test_script, &mut state);

    match result {
        Ok(output) => {
            println!("✓ Script executed successfully");
            println!("Captured output: '{}'", output);

            // Verify that the output contains the print statements
            assert!(output.contains("This is a test print statement"));
            assert!(output.contains("Another print statement"));
            assert!(output.contains("Script executed successfully"));

            // The output should be captured and returned as a string
            // (The caller is responsible for adding it to REPL history)
            let lines: Vec<&str> = output.split('\n').collect();
            assert_eq!(lines.len(), 3, "Should have 3 lines of output");

            println!("✓ Print output was captured correctly");
        }
        Err(e) => {
            panic!("Script execution failed: {}", e);
        }
    }
}

#[test]
fn test_bytes_count_processor_print_output() {
    println!("Testing bytes count processor print output");

    // Create a Lua engine and state
    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = TuiState::new().unwrap();

    // Clear any existing output history
    state.repl_output_history.clear();

    // Create a script similar to the bytes_count_processor.lua
    let processor_script = r#"
        -- Add a processor function to the record_processors array
        table.insert(record_processors, function(record)
            -- Calculate the byte count of the original line
            local bytes_count = #record.original
            
            -- Return a table with the new attribute
            return {
                bytes_count = tostring(bytes_count)
            }
        end)
        
        print("Bytes count processor added to record_processors")
        print("Each new record will now have a 'bytes_count' attribute with the line size in bytes")
    "#;

    // Execute the script using the same method as external scripts
    let result = engine.execute_script_string_with_state(processor_script, &mut state);

    match result {
        Ok(output) => {
            println!("✓ Processor script executed successfully");
            println!("Captured output: '{}'", output);

            // Verify that the output contains the expected print statements
            assert!(output.contains("Bytes count processor added to record_processors"));
            assert!(output.contains("Each new record will now have a 'bytes_count' attribute"));

            // Verify that the output was captured (not printed to stderr)
            // The output should be in the result string, not in stderr

            println!("✓ Processor script print output was captured correctly");
        }
        Err(e) => {
            panic!("Processor script execution failed: {}", e);
        }
    }
}

#[test]
fn test_lua_print_vs_return_value() {
    println!("Testing Lua print vs return value handling");

    // Create a Lua engine and state
    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = TuiState::new().unwrap();

    // Clear any existing output history
    state.repl_output_history.clear();

    // Test script with both print statements and return value
    let test_script = r#"
        print("This is printed output")
        print("More printed output")
        return "This is the return value"
    "#;

    // Execute the script
    let result = engine.execute_script_string_with_state(test_script, &mut state);

    match result {
        Ok(output) => {
            println!("✓ Script with print and return executed successfully");
            println!("Captured output: '{}'", output);

            // Verify that both print output and return value are captured
            assert!(output.contains("This is printed output"));
            assert!(output.contains("More printed output"));
            assert!(output.contains("This is the return value"));

            // The output should be formatted as separate lines
            let lines: Vec<&str> = output.split('\n').collect();
            assert!(lines.len() >= 3, "Should have at least 3 lines of output");

            println!("✓ Both print output and return value were captured correctly");
        }
        Err(e) => {
            panic!("Script execution failed: {}", e);
        }
    }
}

#[test]
fn test_external_script_print_output_integration() {
    println!("Testing external script print output integration");

    // Create a Lua engine and state
    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = TuiState::new().unwrap();

    // Clear any existing output history
    state.repl_output_history.clear();

    // Create a test script similar to bytes_count_processor.lua
    let external_script = r#"
        print("External script loaded")
        print("Setting up record processors")
        
        -- Add a processor function
        table.insert(record_processors, function(record)
            return {
                test_field = "test_value"
            }
        end)
        
        print("Record processor added successfully")
    "#;

    // Execute the script using the same method as external scripts
    let result = engine.execute_script_string_with_state(external_script, &mut state);

    match result {
        Ok(output) => {
            println!("✓ External script executed successfully");
            println!("Captured output: '{}'", output);

            // Verify that the output contains the expected print statements
            assert!(output.contains("External script loaded"));
            assert!(output.contains("Setting up record processors"));
            assert!(output.contains("Record processor added successfully"));

            // Simulate what main.rs does: add the output to REPL history
            if !output.is_empty() {
                state.add_to_lua_console("Script 'test_script.lua' output:".to_string());
                let output_lines: Vec<String> =
                    output.lines().map(|line| format!("  {}", line)).collect();
                state.add_lines_to_lua_console(output_lines);
            }

            // Add success message to Lua console (as main.rs now does)
            state.add_to_lua_console("Script 'test_script.lua' executed successfully.".to_string());

            // Now verify that the output was added to the REPL history
            assert!(!state.repl_output_history.is_empty());
            let history_text: String = state.repl_output_history
                .iter()
                .map(|line| match line {
                    ConsoleLine::Stdout(msg) => msg.clone(),
                    ConsoleLine::Stderr(msg) => msg.clone(),
                })
                .collect::<Vec<String>>()
                .join("\n");
            assert!(history_text.contains("External script loaded"));
            assert!(history_text.contains("Setting up record processors"));
            assert!(history_text.contains("Record processor added successfully"));
            assert!(history_text.contains("Script 'test_script.lua' executed successfully"));

            println!("✓ External script print output was captured and added to REPL history");
        }
        Err(e) => {
            panic!("External script execution failed: {}", e);
        }
    }
}

#[test]
fn test_lua_script_error_handling() {
    println!("Testing Lua script error handling");

    // Create a Lua engine and state
    let mut engine = LuaEngine::new().unwrap();
    engine.initialize().unwrap();
    let mut state = TuiState::new().unwrap();

    // Clear any existing output history
    state.repl_output_history.clear();

    // Create a script with a syntax error
    let error_script = r#"
        print("This will work")
        -- Syntax error: missing end
        if true then
            print("This won't be reached")
    "#;

    // Execute the script
    let result = engine.execute_script_string_with_state(error_script, &mut state);

    match result {
        Ok(_) => {
            panic!("Script should have failed with syntax error");
        }
        Err(e) => {
            println!("✓ Script failed as expected with error: {}", e);

            // Simulate what main.rs does for errors: add to REPL history
            state.add_error_to_lua_console(format!("Error executing script 'test_error.lua': {}", e));

            // Verify that the error was added to the REPL history
            assert!(!state.repl_output_history.is_empty());
            let history_text: String = state.repl_output_history
                .iter()
                .map(|line| match line {
                    ConsoleLine::Stdout(msg) => msg.clone(),
                    ConsoleLine::Stderr(msg) => msg.clone(),
                })
                .collect::<Vec<String>>()
                .join("\n");
            assert!(history_text.contains("Error executing script"));
            assert!(history_text.contains("test_error.lua"));

            println!("✓ Error message was added to REPL history");
        }
    }
}
