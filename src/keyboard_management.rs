use std::cmp::min;

use crossterm::event::{self, KeyCode, KeyEvent};

use crate::{
    ast,
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
            handle_command_mode(key_event, state);
        }
        Mode::ScriptInput => {
            handle_script_input_mode(key_event, state, lua_engine);
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

pub fn handle_command_mode(key_event: KeyEvent, state: &mut TuiState) {
    match key_event.code {
        KeyCode::Tab => {
            show_completions(state);
        }
        KeyCode::Esc => {
            state.mode = Mode::Normal;
        }
        KeyCode::Char('\n') => {
            state.mode = Mode::Normal;
            state.handle_command();
        }
        KeyCode::Enter => {
            state.mode = Mode::Normal;
            state.handle_command();
        }
        _ => {
            handle_textinput(&mut state.command, &mut state.text_edit_position, key_event);
        }
    }
}

pub fn show_completions(state: &mut TuiState) {
    let (common_prefix, completions) = state.get_completions();

    if common_prefix != state.command {
        state.command = common_prefix;
        state.text_edit_position = state.command.len();
        return;
    }
    if completions.len() == 1 {
        state.command = completions[0].clone();
        state.text_edit_position = state.command.len();
    } else if completions.len() > 1 {
        let completions = completions.join(" █ ");
        state.next_mode = Mode::Command;
        state.set_warning(format!("{}", completions));
    } else {
        state.next_mode = Mode::Command;
        state.set_warning("No completions found".to_string());
    }
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
