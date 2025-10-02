//! Completion system for TailTales
//!
//! This module provides tab completion functionality for both command mode
//! and Lua REPL mode, with clean formatting and single responsibility functions.

use crate::lua_engine::LuaEngine;
use crate::state::{Mode, TuiState};

/// Calculate the common prefix from a list of completions
pub fn calculate_common_prefix(completions: &[String]) -> String {
    if completions.is_empty() {
        return String::new();
    }

    if completions.len() == 1 {
        return completions[0].clone();
    }

    let mut common_prefix = String::new();
    let mut pos = 0;
    let mut done = false;

    while !done {
        let mut c: Option<char> = None;
        for completion in completions {
            if pos >= completion.len() {
                done = true;
                break;
            }
            let current_char = completion.chars().nth(pos).unwrap();
            if c.is_none() {
                c = Some(current_char);
            } else if c.unwrap() != current_char {
                done = true;
                break;
            }
        }
        if !done {
            common_prefix.push(c.unwrap());
            pos += 1;
        }
    }

    common_prefix
}

/// Format completions into a clean table grid
pub fn format_completions_table(completions: &[String]) -> Vec<String> {
    if completions.is_empty() {
        return vec!["No completions found".to_string()];
    }

    if completions.len() == 1 {
        return vec![completions[0].clone()];
    }

    // Calculate optimal column width and number of columns
    let max_width = completions.iter().map(|c| c.len()).max().unwrap_or(0);
    let terminal_width = 80; // Default terminal width
    let padding = 2;
    let column_width = max_width + padding;
    let num_columns = (terminal_width / column_width).max(1);

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for (i, completion) in completions.iter().enumerate() {
        let padded_completion = format!("{:<width$}", completion, width = max_width);

        if i % num_columns == 0 && !current_line.is_empty() {
            lines.push(current_line.trim_end().to_string());
            current_line.clear();
        }

        current_line.push_str(&padded_completion);
        current_line.push(' ');
    }

    if !current_line.is_empty() {
        lines.push(current_line.trim_end().to_string());
    }

    lines
}

/// Handle completion logic for command mode
pub fn handle_command_completion(state: &mut TuiState, lua_engine: &mut LuaEngine) {
    let current = state.command.trim();

    // Get completions from Lua VM
    let completions = match lua_engine.get_completions_from_lua(current) {
        Ok(lua_completions) => lua_completions,
        Err(_) => {
            state.next_mode = Mode::Command;
            state.set_warning("Completion system unavailable".to_string());
            return;
        }
    };

    // Handle completion results
    if completions.is_empty() {
        state.next_mode = Mode::Command;
        state.set_warning("No completions found".to_string());
        return;
    }

    let common_prefix = calculate_common_prefix(&completions);

    if common_prefix != state.command {
        // Auto-complete with common prefix
        state.command = common_prefix;
        state.text_edit_position = state.command.len();
    } else if completions.len() == 1 {
        // Auto-complete with single completion
        state.command = completions[0].clone();
        state.text_edit_position = state.command.len();
    } else {
        // Show multiple completions in a formatted table
        let formatted_lines = format_completions_table(&completions);
        let display_text = formatted_lines.join("\n");
        state.next_mode = Mode::Command;
        state.set_warning(display_text);
    }
}

/// Handle completion logic for REPL mode
pub fn handle_repl_completion(state: &mut TuiState, lua_engine: &mut LuaEngine) {
    let current = state.lua_console.input.trim();

    // Get completions from Lua VM
    let completions = match lua_engine.get_completions_from_lua(current) {
        Ok(lua_completions) => lua_completions,
        Err(_) => {
            state.lua_console.add_error(
                "Completion system unavailable".to_string(),
                state.visible_width,
            );
            // Auto-scroll to show the error message
            let visible_lines = state.visible_height.saturating_sub(2);
            let total_lines = state.lua_console.output_history.len() + 1; // +1 for current input line
            if total_lines > visible_lines {
                state.lua_console.scroll_offset = total_lines.saturating_sub(visible_lines);
            }
            return;
        }
    };

    // Handle completion results
    if completions.is_empty() {
        state
            .lua_console
            .add_output("No completions found".to_string(), state.visible_width);
        // Auto-scroll to show the message
        let visible_lines = state.visible_height.saturating_sub(2);
        let total_lines = state.lua_console.output_history.len() + 1; // +1 for current input line
        if total_lines > visible_lines {
            state.lua_console.scroll_offset = total_lines.saturating_sub(visible_lines);
        }
        return;
    }

    let common_prefix = calculate_common_prefix(&completions);

    if common_prefix != state.lua_console.input {
        // Auto-complete with common prefix
        state.lua_console.input = common_prefix;
        state.text_edit_position = state.lua_console.input.len();
    } else if completions.len() == 1 {
        // Auto-complete with single completion
        state.lua_console.input = completions[0].clone();
        state.text_edit_position = state.lua_console.input.len();
    } else {
        // Show multiple completions in a formatted table in REPL output
        let formatted_lines = format_completions_table(&completions);
        for line in formatted_lines {
            state.lua_console.add_output(line, state.visible_width);
        }

        // Auto-scroll to show the completion lines
        let visible_lines = state.visible_height.saturating_sub(2);
        let total_lines = state.lua_console.output_history.len() + 1; // +1 for current input line
        if total_lines > visible_lines {
            state.lua_console.scroll_offset = total_lines.saturating_sub(visible_lines);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_common_prefix() {
        // Test empty list
        assert_eq!(calculate_common_prefix(&[]), "");

        // Test single item
        assert_eq!(calculate_common_prefix(&["hello()".to_string()]), "hello()");

        // Test multiple items with common prefix
        let completions = vec!["get_record()".to_string(), "get_position()".to_string()];
        assert_eq!(calculate_common_prefix(&completions), "get_");

        // Test items with no common prefix
        let completions = vec!["quit()".to_string(), "warning()".to_string()];
        assert_eq!(calculate_common_prefix(&completions), "");

        // Test items with partial common prefix
        let completions = vec!["vmove()".to_string(), "vgoto()".to_string()];
        assert_eq!(calculate_common_prefix(&completions), "v");
    }

    #[test]
    fn test_format_completions_table() {
        // Test empty list
        let result = format_completions_table(&[]);
        assert_eq!(result, vec!["No completions found"]);

        // Test single item
        let result = format_completions_table(&["quit()".to_string()]);
        assert_eq!(result, vec!["quit()"]);

        // Test multiple items
        let completions = vec![
            "quit()".to_string(),
            "warning()".to_string(),
            "vmove()".to_string(),
            "vgoto()".to_string(),
        ];
        let result = format_completions_table(&completions);
        assert!(result.len() >= 1);
        assert!(result[0].contains("quit()"));
        assert!(result[0].contains("warning()"));
    }
}
