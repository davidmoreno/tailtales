use std::cmp::min;

use crossterm::event::{self, KeyCode, KeyEvent};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    ast,
    completions::{handle_command_completion, handle_repl_completion},
    lua_console::ConsoleLine,
    lua_engine::LuaEngine,
    settings::Settings,
    state::{Mode, TuiState},
};
use log::debug;

/**
 * This module is responsible for managing the keyboard input and output
 */

pub fn handle_key_event(key_event: KeyEvent, state: &mut TuiState, lua_engine: &mut LuaEngine) {
    match state.mode {
        Mode::Normal => {
            handle_normal_mode(key_event, state, lua_engine);
        }
        Mode::Search => {
            handle_search_mode(key_event, state);
        }
        Mode::Filter => {
            handle_filter_mode(key_event, state);
        }
        Mode::Command => {
            handle_command_mode(key_event, state, lua_engine);
        }
        Mode::ScriptInput => {
            handle_script_input_mode(key_event, state, lua_engine);
        }
        Mode::LuaRepl => {
            handle_lua_repl_mode(key_event, state, lua_engine);
        }
        Mode::Warning => {
            // Any key will dismiss the warning
            state.mode = state.next_mode;
            state.next_mode = Mode::Normal;
            handle_key_event(key_event, state, lua_engine); // pass through
        }
    }
}

pub fn handle_normal_mode(key_event: KeyEvent, state: &mut TuiState, lua_engine: &mut LuaEngine) {
    let keyname: &str = match key_event.code {
        // numbers add to number
        KeyCode::Char(x) => &String::from(x).to_lowercase(),
        KeyCode::F(x) => &String::from(x as char),

        x => &x.to_string().to_lowercase(),
    };
    let keyname = if key_event.modifiers.contains(event::KeyModifiers::SHIFT) {
        &format!("shift-{}", keyname)
    } else {
        keyname
    };
    let keyname = if key_event.modifiers.contains(event::KeyModifiers::CONTROL) {
        &format!("control-{}", keyname)
    } else {
        keyname
    };
    // F1 - F12 are \u{1}... \u{c}
    let keyname = match key_event.code {
        KeyCode::F(1) => "F1",
        KeyCode::F(2) => "F2",
        KeyCode::F(3) => "F3",
        KeyCode::F(4) => "F4",
        KeyCode::F(5) => "F5",
        KeyCode::F(6) => "F6",
        KeyCode::F(7) => "F7",
        KeyCode::F(8) => "F8",
        KeyCode::F(9) => "F9",
        KeyCode::F(10) => "F10",
        KeyCode::F(11) => "F11",
        KeyCode::F(12) => "F12",
        _ => keyname,
    };

    if state.settings.keybindings.contains_key(keyname) {
        let script_name = Settings::get_keybinding_script_name(keyname);

        // Execute compiled Lua script (try async first for ask() support)
        if lua_engine
            .get_compiled_scripts()
            .contains(&script_name.as_str())
        {
            // Execute the script with efficient immediate execution
            match lua_engine.execute_with_state(&script_name, state) {
                Ok(Some(prompt)) => {
                    // Script asking for input - handle in state
                    state.script_prompt = prompt;
                    state.script_waiting = true;
                    state.mode = Mode::ScriptInput;
                    state.script_input.clear();
                }
                Ok(None) => {
                    // Script completed immediately - no need to process commands since they executed immediately
                    debug!(
                        "Script '{}' completed with immediate execution",
                        script_name
                    );
                }
                Err(e) => {
                    state.set_warning(format!(
                        "Lua script execution failed for key '{}': {}",
                        keyname, e
                    ));
                }
            }
        } else {
            state.set_warning(format!(
                "No compiled Lua script found for key: {:?}",
                keyname
            ));
        }
    } else {
        state.set_warning(format!("Unknown keybinding: {:?}", keyname));
    }
}

pub fn handle_search_mode(key_event: KeyEvent, state: &mut TuiState) {
    match key_event.code {
        KeyCode::Esc => {
            state.mode = Mode::Normal;
        }
        KeyCode::Char('\n') => {
            state.mode = Mode::Normal;
            state.search_fwd();
        }
        KeyCode::Backspace => {
            state.search.pop();
        }
        KeyCode::Enter => {
            state.mode = Mode::Normal;
            state.search_fwd();
        }
        KeyCode::F(3) => {
            state.search_next();
        }
        _ => {
            handle_textinput(&mut state.search, &mut state.text_edit_position, key_event);
            state.search_ast = ast::parse(&state.search).ok();
            state.search_fwd();
        }
    }
}

pub fn handle_textinput(text: &mut String, position: &mut usize, keyevent: KeyEvent) {
    match keyevent.code {
        KeyCode::Char('u') if keyevent.modifiers.contains(event::KeyModifiers::CONTROL) => {
            text.clear();
            *position = 0;
        }
        // this is what gets received on control backspace
        KeyCode::Char('h') if keyevent.modifiers.contains(event::KeyModifiers::CONTROL) => {
            text.clear();
            *position = 0;
        }
        KeyCode::Backspace if keyevent.modifiers.contains(event::KeyModifiers::CONTROL) => {
            text.clear();
            *position = 0;
        }
        KeyCode::Left => {
            *position = if *position > 0 { *position - 1 } else { 0 };
        }
        KeyCode::Right => {
            *position = min(text.len(), *position + 1);
        }
        KeyCode::Home => {
            *position = 0;
        }
        KeyCode::End => {
            *position = text.len();
        }
        KeyCode::Delete => {
            if *position < text.len() {
                text.remove(*position);
            }
        }

        KeyCode::Backspace => {
            if *position > text.len() {
                *position = text.len();
            }
            // remove at position, and go back
            if *position > 0 {
                text.remove(*position - 1);
                *position -= 1;
            }
        }
        KeyCode::Char(c) => {
            if *position > text.len() {
                *position = text.len();
            }
            // insert at position, and advance
            text.insert(*position, c);
            *position += 1;
        }
        _ => {}
    };
}

pub fn handle_command_mode(key_event: KeyEvent, state: &mut TuiState, lua_engine: &mut LuaEngine) {
    match key_event.code {
        KeyCode::Tab => {
            handle_command_completion(state, lua_engine);
        }
        KeyCode::Esc => {
            state.mode = Mode::Normal;
        }
        KeyCode::Char('\n') => {
            state.mode = Mode::Normal;
            handle_command_execution(state, lua_engine);
        }
        KeyCode::Enter => {
            state.mode = Mode::Normal;
            handle_command_execution(state, lua_engine);
        }
        _ => {
            handle_textinput(&mut state.command, &mut state.text_edit_position, key_event);
        }
    }
}

/// Handle execution of user-entered commands from command mode
pub fn handle_command_execution(state: &mut TuiState, lua_engine: &mut LuaEngine) {
    let command = state.command.trim().to_string();
    if command.is_empty() {
        return;
    }

    debug!("Executing command from command mode: {}", command);

    // Create a unique script name for the command
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let script_name = format!("cmd_{}", timestamp);

    // Compile the command as a Lua script
    match lua_engine.compile_script(&script_name, &command) {
        Ok(_) => {
            // Execute the compiled script
            match lua_engine.execute_with_state(&script_name, state) {
                Ok(Some(prompt)) => {
                    // Script asking for input - handle in state
                    state.script_prompt = prompt;
                    state.script_waiting = true;
                    state.mode = Mode::ScriptInput;
                    state.script_input.clear();
                }
                Ok(None) => {
                    // Script completed immediately
                    debug!("Command '{}' completed successfully", command);
                }
                Err(e) => {
                    state.set_warning(format!("Command execution failed: {}", e));
                }
            }
        }
        Err(e) => {
            state.set_warning(format!("Command compilation failed: {}", e));
        }
    }

    // Clear the command after execution
    state.command.clear();
}

pub fn handle_filter_mode(key_event: KeyEvent, state: &mut TuiState) {
    match key_event.code {
        KeyCode::Esc => {
            state.mode = Mode::Normal;
            state.filter = String::new();
            state.handle_filter()
        }
        KeyCode::Char('\n') => {
            state.mode = Mode::Normal;
            state.handle_filter()
        }
        KeyCode::Enter => {
            state.mode = Mode::Normal;
            state.handle_filter()
        }
        _ => {
            handle_textinput(&mut state.filter, &mut state.text_edit_position, key_event);
            state.handle_filter();
        }
    }
}

pub fn handle_script_input_mode(
    key_event: KeyEvent,
    state: &mut TuiState,
    lua_engine: &mut LuaEngine,
) {
    match key_event.code {
        KeyCode::Esc => {
            // Cancel the suspended script
            lua_engine.cancel_suspended_script();
            state.script_waiting = false;
            state.script_prompt.clear();
            state.script_input.clear();
            state.mode = Mode::Normal;
        }
        KeyCode::Char('\n') | KeyCode::Enter => {
            // Submit the input to the suspended script
            let input = state.script_input.clone();
            if !state.script_waiting {
                state.set_warning("No script is waiting for input".to_string());
                return;
            }

            match lua_engine.resume_with_input(input) {
                Ok(_) => {
                    // Check if the script is asking for more input
                    if let Some(new_prompt) = lua_engine.get_suspended_prompt() {
                        state.script_prompt = new_prompt.to_string();
                        state.script_input.clear();
                    } else {
                        // Script completed, return to normal mode
                        state.script_waiting = false;
                        state.script_prompt.clear();
                        state.script_input.clear();
                        state.mode = Mode::Normal;

                        // No need to process commands - they executed immediately
                    }
                }
                Err(e) => {
                    // Script failed, return to normal mode
                    state.script_waiting = false;
                    state.script_prompt.clear();
                    state.script_input.clear();
                    state.mode = Mode::Normal;
                    state.set_warning(format!("Script execution failed: {}", e));
                }
            }
        }
        _ => {
            // Handle text input for the script prompt
            handle_textinput(
                &mut state.script_input,
                &mut state.text_edit_position,
                key_event,
            );
        }
    }
}

pub fn handle_lua_repl_mode(key_event: KeyEvent, state: &mut TuiState, lua_engine: &mut LuaEngine) {
    match key_event.code {
        KeyCode::Esc | KeyCode::F(12) => {
            // Exit REPL mode back to Normal
            state.mode = Mode::Normal;
            state.lua_console.input.clear();
            state.lua_console.text_edit_position = 0;
            // Reset multiline state
            state.lua_console.multiline_buffer.clear();
            state.lua_console.is_multiline = false;
        }
        KeyCode::Char('c') if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
            // Cancel current multiline input (Ctrl+C)
            if state.lua_console.is_multiline {
                state
                    .lua_console
                    .output_history
                    .push(ConsoleLine::Stdout("^C".to_string()));
                state.lua_console.multiline_buffer.clear();
                state.lua_console.is_multiline = false;
                state.lua_console.input.clear();
                state.lua_console.text_edit_position = 0;
            }
        }
        KeyCode::Char('l') if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
            // Clear REPL console buffer (Ctrl+L)
            state.lua_console.output_history.clear();
        }
        KeyCode::Tab => {
            // Tab completion for Lua REPL
            handle_repl_completion(state, lua_engine);
        }
        KeyCode::Char('\n') | KeyCode::Enter => {
            let input = state.lua_console.input.trim().to_string();

            if input.is_empty() && !state.lua_console.is_multiline {
                // Empty input - add empty line with prompt to history for console interactivity check
                state
                    .lua_console
                    .output_history
                    .push(ConsoleLine::Stdout("> ".to_string()));

                // Clear current input line for next input
                state.lua_console.input.clear();
                state.lua_console.text_edit_position = 0;
                return;
            }

            // Add current line to multiline buffer or start one
            if state.lua_console.is_multiline {
                // Add continuation line to buffer
                let prompt = if state.lua_console.multiline_buffer.is_empty() {
                    "> "
                } else {
                    ">> "
                };
                state
                    .lua_console
                    .add_output(format!("{}{}", prompt, input), state.visible_width);
                state.lua_console.multiline_buffer.push(input.clone());
            } else {
                // This might be the start of a multiline construct
                state
                    .lua_console
                    .add_output(format!("> {}", input), state.visible_width);
                state.lua_console.multiline_buffer.push(input.clone());
                state.lua_console.is_multiline = true;
            }

            // Check if input is complete
            if state.is_lua_input_complete() {
                // Execute the complete multiline code
                let full_code = state.lua_console.multiline_buffer.join("\n");

                match lua_engine.execute_script_string_with_state(&full_code, state) {
                    Ok(result) => {
                        if !result.is_empty() {
                            // Split multi-line output into separate history entries with text wrapping
                            for line in result.lines() {
                                if !line.is_empty() {
                                    state
                                        .lua_console
                                        .add_output(line.to_string(), state.visible_width);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        state
                            .lua_console
                            .add_error(format!("Error: {}", e), state.visible_width);
                    }
                }

                // Add command to history with semicolon separators for multiline
                let history_command = if state.lua_console.multiline_buffer.len() > 1 {
                    state.lua_console.multiline_buffer.join("; ")
                } else {
                    full_code
                };
                state.add_to_repl_history(history_command);

                // Reset multiline state
                state.lua_console.multiline_buffer.clear();
                state.lua_console.is_multiline = false;

                // Reset history navigation
                state.reset_repl_history_navigation();

                // Keep output history reasonable size
                if state.lua_console.output_history.len() > 1000 {
                    state.lua_console.output_history.drain(0..500);
                }

                // Auto-scroll to bottom to show new output and input line
                let visible_lines = state.visible_height.saturating_sub(2);
                let total_lines = state.lua_console.output_history.len() + 1; // +1 for current input line
                if total_lines > visible_lines {
                    state.lua_console.scroll_offset = total_lines.saturating_sub(visible_lines);
                }
            }

            // Clear current input line for next input
            state.lua_console.input.clear();
            state.lua_console.text_edit_position = 0;
        }
        KeyCode::PageUp => {
            // Scroll up in output history
            state.lua_console.scroll_offset = state.lua_console.scroll_offset.saturating_sub(10);
        }
        KeyCode::PageDown => {
            // Scroll down in output history
            let visible_lines = state.visible_height.saturating_sub(2);
            let total_lines = state.lua_console.output_history.len() + 1; // +1 for input line
            let max_scroll = total_lines.saturating_sub(visible_lines);
            state.lua_console.scroll_offset =
                (state.lua_console.scroll_offset + 10).min(max_scroll);
        }
        // Alternative scrolling keys (Ctrl+Up/Down for output history scrolling)
        KeyCode::Up if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
            // Scroll up one line in output history
            state.lua_console.scroll_offset = state.lua_console.scroll_offset.saturating_sub(1);
        }
        KeyCode::Down if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
            // Scroll down one line in output history
            let visible_lines = state.visible_height.saturating_sub(2);
            let total_lines = state.lua_console.output_history.len() + 1; // +1 for input line
            let max_scroll = total_lines.saturating_sub(visible_lines);
            state.lua_console.scroll_offset = (state.lua_console.scroll_offset + 1).min(max_scroll);
        }
        KeyCode::Up => {
            // Navigate command history up (older commands)
            if state.repl_history_up() {
                // Auto-scroll to show input line
                let visible_lines = state.visible_height.saturating_sub(2);
                let total_lines = state.lua_console.output_history.len() + 1; // +1 for input line
                if total_lines > visible_lines {
                    state.lua_console.scroll_offset = total_lines.saturating_sub(visible_lines);
                }
            }
        }
        KeyCode::Down => {
            // Navigate command history down (newer commands)
            if state.repl_history_down() {
                // Auto-scroll to show input line
                let visible_lines = state.visible_height.saturating_sub(2);
                let total_lines = state.lua_console.output_history.len() + 1; // +1 for input line
                if total_lines > visible_lines {
                    state.lua_console.scroll_offset = total_lines.saturating_sub(visible_lines);
                }
            }
        }
        _ => {
            // Reset history navigation when user starts typing
            if matches!(
                key_event.code,
                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
            ) {
                state.reset_repl_history_navigation();
            }

            // Handle text input for the REPL
            handle_textinput(
                &mut state.lua_console.input,
                &mut state.lua_console.text_edit_position,
                key_event,
            );

            // Auto-scroll to keep input line visible while typing
            let visible_lines = state.visible_height.saturating_sub(2);
            let total_lines = state.lua_console.output_history.len() + 1; // +1 for current input line
            if total_lines > visible_lines {
                state.lua_console.scroll_offset = total_lines.saturating_sub(visible_lines);
            }
        }
    }
}
