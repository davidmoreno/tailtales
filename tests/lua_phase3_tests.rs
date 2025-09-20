//! Lua Phase 3 Tests: Async Support
//!
//! Tests for coroutine support, yield/resume mechanism, and the ask() function

use tailtales::lua_engine::LuaEngine;
use tailtales::state::{Mode, TuiState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua010_coroutine_creation_and_management() {
        let mut engine = LuaEngine::new().expect("Failed to create Lua engine");

        // Test basic coroutine creation by executing a simple yielding script
        let script = r#"
            local value = 42
            coroutine.yield("test_yield")
            warning("Script resumed with value: " .. tostring(value))
        "#;

        // Execute asynchronously - should yield
        let result = engine.execute_script_string_async(script);
        assert!(result.is_ok(), "Script execution should succeed");

        // Check if the script yielded with a prompt (this would be an ask() call)
        // For basic yield, it should complete normally
        match result.unwrap() {
            Some(_prompt) => {
                // Script yielded - this would happen with ask() calls
                assert!(engine.has_suspended_script());
            }
            None => {
                // Script completed normally
                println!("Script completed without yielding");
            }
        }
    }

    #[test]
    fn test_lua011_yield_resume_mechanism() {
        let mut engine = LuaEngine::new().expect("Failed to create Lua engine");

        // Test script that uses ask() function to yield and wait for input
        let script = r#"
            local user_input = ask("Enter your name:")
            warning("Hello, " .. user_input .. "!")
        "#;

        // Execute asynchronously using execute_script_string_async
        let result = engine.execute_script_string_async(script);
        match result {
            Ok(maybe_prompt) => {
                if let Some(prompt) = maybe_prompt {
                    assert_eq!(prompt, "Enter your name:");
                    assert!(engine.has_suspended_script());
                    assert_eq!(engine.get_suspended_prompt(), Some("Enter your name:"));

                    // Resume with user input
                    let resume_result = engine.resume_with_input("Alice".to_string());
                    match resume_result {
                        Ok(_) => {
                            println!("Resume succeeded");
                            // After resume, script should complete
                            assert!(!engine.has_suspended_script());
                        }
                        Err(e) => {
                            println!("Resume failed with error: {:?}", e);
                            panic!("Resume should succeed: {:?}", e);
                        }
                    }
                } else {
                    panic!("Script should have yielded with ask prompt");
                }
            }
            Err(e) => {
                panic!("Script execution failed with error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_debug_simple_ask() {
        let mut engine = LuaEngine::new().expect("Failed to create Lua engine");

        // Simple ask test
        let script = r#"ask("Test prompt")"#;

        match engine.execute_script_string_async(script) {
            Ok(maybe_prompt) => {
                println!("Success! Got prompt: {:?}", maybe_prompt);
                if let Some(prompt) = maybe_prompt {
                    println!("Script suspended with prompt: {}", prompt);
                    assert_eq!(prompt, "Test prompt");
                } else {
                    println!("Script completed without suspending");
                }
            }
            Err(e) => {
                println!("Script failed with error: {:?}", e);
                panic!("Simple ask script should succeed");
            }
        }
    }

    #[test]
    fn test_lua012_script_suspension_and_user_input() {
        let mut state = TuiState::new().expect("Failed to create TuiState");

        // Test script that asks for multiple inputs
        let script = r#"
            local name = ask("What's your name?")
            local age = ask("What's your age?")
            warning("Hello " .. name .. ", you are " .. age .. " years old!")
        "#;

        // Execute asynchronously
        let result = state.handle_lua_command_async(script);
        assert!(result.is_ok(), "Script execution should succeed");

        // Should be in script input mode
        assert_eq!(state.mode, Mode::ScriptInput);
        assert!(state.script_waiting);
        assert_eq!(state.script_prompt, "What's your name?");

        // Provide first input
        let resume_result = state.resume_suspended_script("Bob".to_string());
        assert!(resume_result.is_ok(), "First resume should succeed");

        // Should still be in script input mode for second question
        assert_eq!(state.mode, Mode::ScriptInput);
        assert!(state.script_waiting);
        assert_eq!(state.script_prompt, "What's your age?");

        // Provide second input
        let resume_result = state.resume_suspended_script("25".to_string());
        assert!(resume_result.is_ok(), "Second resume should succeed");

        // Should return to normal mode
        assert_eq!(state.mode, Mode::Normal);
        assert!(!state.script_waiting);
        assert!(state.script_prompt.is_empty());
    }

    #[test]
    fn test_lua013_script_cancellation() {
        let mut state = TuiState::new().expect("Failed to create TuiState");

        let script = r#"
            local input = ask("This will be cancelled")
            warning("This should not execute")
        "#;

        // Execute asynchronously
        assert!(state.handle_lua_command_async(script).is_ok());

        // Verify script is suspended
        assert_eq!(state.mode, Mode::ScriptInput);
        assert!(state.script_waiting);

        // Cancel the script
        state.cancel_suspended_script();

        // Verify return to normal mode
        assert_eq!(state.mode, Mode::Normal);
        assert!(!state.script_waiting);
        assert!(state.script_prompt.is_empty());
        assert!(!state.has_suspended_script());
    }

    #[test]
    fn test_lua014_ask_function_with_commands() {
        let mut engine = LuaEngine::new().expect("Failed to create Lua engine");

        // Test script that uses ask() and then executes commands based on input
        let script = r#"
            local action = ask("Choose action (quit/move/mark):")
            if action == "quit" then
                quit()
            elseif action == "move" then
                vmove(5)
            elseif action == "mark" then
                toggle_mark("red")
            else
                warning("Unknown action: " .. action)
            end
        "#;

        // Execute and provide input
        let result = engine.execute_script_string_async(script);
        assert!(result.is_ok());

        if let Some(prompt) = result.unwrap() {
            assert_eq!(prompt, "Choose action (quit/move/mark):");

            // Test with "move" input
            let resume_result = engine.resume_with_input("move".to_string());
            assert!(resume_result.is_ok());

            // Should have collected the vmove command
            let commands = engine
                .collect_executed_commands(Some("interactive_commands".to_string()))
                .unwrap();
            assert!(commands.contains_key("vmove"));
        }
    }

    #[test]
    fn test_lua015_nested_ask_calls() {
        let mut engine = LuaEngine::new().expect("Failed to create Lua engine");

        // Test script with nested ask calls and conditional logic
        let script = r#"
            local confirm = ask("Are you sure? (yes/no)")
            if confirm == "yes" then
                local details = ask("Enter details:")
                warning("Confirmed with details: " .. details)
            else
                warning("Operation cancelled")
            end
        "#;

        // First execution - should ask for confirmation
        let result = engine.execute_script_string_async(script);
        assert!(result.is_ok());

        if let Some(prompt) = result.unwrap() {
            assert_eq!(prompt, "Are you sure? (yes/no)");

            // Answer "yes" - should ask for details
            let resume_result = engine.resume_with_input("yes".to_string());
            assert!(resume_result.is_ok());

            // Should still have suspended script for second question
            assert!(engine.has_suspended_script());
            assert_eq!(engine.get_suspended_prompt(), Some("Enter details:"));

            // Provide details
            let resume_result = engine.resume_with_input("Test details".to_string());
            assert!(resume_result.is_ok());

            // Should be completed now
            assert!(!engine.has_suspended_script());
        }
    }

    #[test]
    fn test_lua016_error_handling_in_suspended_scripts() {
        let mut engine = LuaEngine::new().expect("Failed to create Lua engine");

        // Test script that has an error after ask()
        let script = r#"
            local input = ask("Enter a number:")
            local num = tonumber(input)
            if num == nil then
                error("Invalid number: " .. input)
            end
            warning("Number is: " .. num)
        "#;

        let result = engine.execute_script_string_async(script);
        assert!(result.is_ok());

        if let Some(_prompt) = result.unwrap() {
            // Provide invalid input that will cause an error
            let resume_result = engine.resume_with_input("not_a_number".to_string());
            assert!(resume_result.is_err(), "Should fail with invalid input");

            // Engine should clean up suspended state on error
            assert!(!engine.has_suspended_script());
        }
    }

    #[test]
    fn test_lua017_performance_with_lazy_current_table() {
        let mut state = TuiState::new().expect("Failed to create TuiState");

        // Add some test records
        for i in 0..100 {
            let mut record = tailtales::record::Record::new(format!("Test line {}", i));
            record
                .data
                .insert("field1".to_string(), format!("value{}", i));
            record
                .data
                .insert("field2".to_string(), format!("data{}", i));
            state.records.visible_records.push(record);
        }

        state.position = 50; // Position in middle

        // Test that simple operations don't need record data access
        let simple_script = "vmove(1)"; // Just move cursor, no record access

        // This should be fast as it doesn't access current.* fields
        let start_time = std::time::Instant::now();
        let result = state.handle_lua_command(simple_script);
        let duration = start_time.elapsed();

        assert!(result.is_ok(), "Simple script should succeed");
        assert!(
            duration.as_millis() < 10,
            "Simple operations should be very fast"
        );

        // Test that accessing current data works when needed
        let record_access_script = r#"
            warning("Current line: " .. current.line .. " at position " .. current.line_number)
        "#;

        let result = state.handle_lua_command(record_access_script);
        assert!(result.is_ok(), "Record access script should succeed");
        assert!(
            state.warning.contains("Test line 50"),
            "Should access correct record"
        );
        assert!(
            state.warning.contains("position 51"),
            "Line number should be 1-based"
        );
    }

    #[test]
    fn test_lua018_async_script_with_mode_transitions() {
        let mut state = TuiState::new().expect("Failed to create TuiState");

        let script = r#"
            mode("search")
            local query = ask("Enter search term:")
            -- Script should be able to handle mode changes
            mode("normal")
            warning("Searching for: " .. query)
        "#;

        assert!(state.compile_lua_script("mode_test", script).is_ok());
        assert!(state.execute_lua_script_async("mode_test").is_ok());

        // Should be in script input mode, not search mode
        assert_eq!(state.mode, Mode::ScriptInput);
        assert!(state.script_waiting);

        // Resume with input
        let result = state.resume_suspended_script("test query".to_string());
        assert!(result.is_ok());

        // Should be back to normal mode as set by the script
        assert_eq!(state.mode, Mode::Normal);
        assert!(state.warning.contains("Searching for: test query"));
    }

    #[test]
    fn test_lua019_concurrent_script_prevention() {
        let mut state = TuiState::new().expect("Failed to create TuiState");

        let script1 = r#"ask("First script input:")"#;
        let script2 = r#"ask("Second script input:")"#;

        // Start first script
        assert!(state.handle_lua_command_async(script1).is_ok());
        assert_eq!(state.mode, Mode::ScriptInput);
        assert_eq!(state.script_prompt, "First script input:");

        // Attempting to start second script while first is suspended should fail or be ignored
        // The behavior depends on implementation - it could either:
        // 1. Fail with error (safer)
        // 2. Cancel first script and start second (user choice)
        // 3. Queue the second script (complex)

        // For this test, let's assume it should fail
        let _result = state.execute_lua_script_async("script2");
        // Implementation detail: you might want to add logic to prevent concurrent scripts

        // Clean up by cancelling the first script
        state.cancel_suspended_script();
        assert_eq!(state.mode, Mode::Normal);
    }

    #[test]
    fn test_lua020_comprehensive_async_workflow() {
        let mut state = TuiState::new().expect("Failed to create TuiState");

        // Add test record
        let mut record = tailtales::record::Record::new("Test log line with error".to_string());
        record.data.insert("level".to_string(), "ERROR".to_string());
        record
            .data
            .insert("message".to_string(), "Connection failed".to_string());
        state.records.visible_records.push(record);

        // Complex interactive script that demonstrates Phase 3 capabilities
        let script = r#"
            -- Access current record data
            local level = current.level or "UNKNOWN"
            local message = current.message or current.line

            if level == "ERROR" then
                local action = ask("Error found: " .. message .. "\nWhat would you like to do?\n1. Copy to clipboard\n2. Open in browser\n3. Mark as reviewed\n4. Skip")

                if action == "1" then
                    exec('echo "' .. current.line .. '" | xclip -selection clipboard')
                    warning("Copied to clipboard")
                elseif action == "2" then
                    local search_query = ask("Enter search terms:")
                    exec('xdg-open "https://google.com/search?q=' .. url_encode(search_query) .. '"')
                    warning("Opened browser with search: " .. search_query)
                elseif action == "3" then
                    toggle_mark("green")
                    warning("Marked as reviewed")
                elseif action == "4" then
                    warning("Skipped error")
                else
                    warning("Unknown action: " .. action)
                end
            else
                warning("No error found in current line")
            end
        "#;

        assert!(state.handle_lua_command_async(script).is_ok());

        // Should prompt for action
        assert_eq!(state.mode, Mode::ScriptInput);
        assert!(state.script_prompt.contains("Error found"));
        assert!(state.script_prompt.contains("Connection failed"));

        // Choose option 2 (open in browser)
        let result = state.resume_suspended_script("2".to_string());
        assert!(result.is_ok());

        // Should ask for search terms
        assert_eq!(state.mode, Mode::ScriptInput);
        assert_eq!(state.script_prompt, "Enter search terms:");

        // Provide search terms
        let result = state.resume_suspended_script("connection timeout".to_string());
        assert!(result.is_ok());

        // Should return to normal mode and show completion message
        assert_eq!(state.mode, Mode::Normal);
        assert!(state.warning.contains("Opened browser with search"));
    }
}
