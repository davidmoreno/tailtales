use std::time;

use crate::{
    ast,
    lua_console::LuaConsole,
    recordlist::{self, load_parsers},
    settings::{RulesSettings, Settings},
};

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Mode {
    Normal,
    Search,
    Filter,
    Command,
    Warning,
    ScriptInput,
    LuaRepl,
}

pub struct TuiState {
    pub settings: Settings,
    pub current_rule: RulesSettings,
    pub records: recordlist::RecordList,
    pub visible_height: usize,
    pub visible_width: usize,
    pub position: usize,
    pub scroll_offset_top: usize,
    pub scroll_offset_left: usize,
    pub running: bool,
    pub read_time: time::Duration,
    pub mode: Mode,
    pub next_mode: Mode,
    pub search: String,
    pub search_ast: Option<ast::AST>,
    pub filter: String,
    pub filter_ok: bool,
    pub command: String,
    pub warning: String,
    pub view_details: bool,
    pub text_edit_position: usize,
    pub pending_refresh: bool, // If true, the screen will be refreshed when the screen receives render request
    pub script_prompt: String,
    pub script_input: String,
    pub script_waiting: bool,
    // Lua console
    pub lua_console: LuaConsole,
}

impl TuiState {
    pub fn new() -> Result<TuiState, Box<dyn std::error::Error>> {
        let settings = Settings::new()?;

        let current_rule = RulesSettings::default();
        let mut records = recordlist::RecordList::new();

        if let Err(err) = load_parsers(&current_rule, &mut records.parsers) {
            return Err(format!("Could not load parsers: {:?}", err).into());
        }

        Ok(TuiState {
            settings,
            current_rule,
            records,
            visible_height: 25,
            visible_width: 80,
            position: 1, // Start with 1-based indexing
            scroll_offset_top: 0,
            scroll_offset_left: 0,
            running: true,
            read_time: time::Duration::new(0, 0),
            mode: Mode::Normal,
            next_mode: Mode::Normal,
            search: String::new(),
            search_ast: None,
            filter: String::new(),
            filter_ok: true,
            command: String::new(),
            warning: String::new(),
            view_details: false, // Default view_details value
            text_edit_position: 0,
            pending_refresh: false,
            script_prompt: String::new(),
            script_input: String::new(),
            script_waiting: false,
            // Initialize Lua console
            lua_console: LuaConsole::new(),
        })
    }

    pub fn search_next(&mut self) {
        let current = self.position;
        self.set_position_wrap(self.position as i32 + 1);
        if !self.search_fwd() {
            // if not found, go back to the original position
            self.set_position(current);
        }
    }

    pub fn search_fwd(&mut self) -> bool {
        let search_ast = self.search_ast.as_ref();
        if search_ast.is_none() {
            return false;
        }
        let search_ast = search_ast.unwrap();
        let mut current = self.position - 1; // Convert to 0-based for search

        let maybe_position = self.records.search_forward(search_ast, current);
        if maybe_position.is_none() {
            return false;
        }
        current = maybe_position.unwrap();
        self.set_position(current + 1); // Convert back to 1-based
        true
    }

    pub fn search_prev(&mut self) {
        let current = self.position;
        self.set_position_wrap(self.position as i32 - 1);
        if !self.search_bwd() {
            self.set_position(current);
        }
    }

    pub fn search_bwd(&mut self) -> bool {
        let search_ast = self.search_ast.as_ref();
        if search_ast.is_none() {
            return false;
        }
        let search_ast = search_ast.unwrap();
        let mut current = self.position - 1; // Convert to 0-based for search

        let maybe_position = self.records.search_backwards(search_ast, current);
        if maybe_position.is_none() {
            return false;
        }
        current = maybe_position.unwrap();
        self.set_position(current + 1); // Convert back to 1-based
        true
    }

    pub fn handle_filter(&mut self) {
        let parsed = ast::parse(&self.filter);
        match parsed {
            Ok(parsed) => {
                self.records.filter_parallel(parsed);
                self.set_position(1); // Use 1-based indexing
                self.filter_ok = true;
            }
            Err(_err) => {
                self.filter_ok = false;
                // panic!("TODO show error parsing: {}", err);
            }
        }
    }

    pub fn set_warning(&mut self, warning: String) {
        self.warning = warning;
        self.mode = Mode::Warning;
    }

    pub fn set_mode(&mut self, mode: &str) {
        match mode {
            "normal" => {
                self.mode = Mode::Normal;
            }
            "search" => {
                self.mode = Mode::Search;
            }
            "filter" => {
                self.mode = Mode::Filter;
            }
            "command" => {
                self.mode = Mode::Command;
            }
            "script_input" => {
                self.mode = Mode::ScriptInput;
            }
            "lua_repl" => {
                self.mode = Mode::LuaRepl;
                // Ensure Lua console is initialized with welcome message
                self.ensure_lua_console_initialized();

                // Load command history from disk when entering REPL mode
                self.load_repl_history();
                self.reset_repl_history_navigation();
            }
            _ => {
                self.set_warning(format!("Unknown mode: {}", mode));
            }
        }
    }

    // Lua Console delegation methods
    /// Initialize the Lua console with welcome message if it's empty
    pub fn ensure_lua_console_initialized(&mut self) {
        self.lua_console.ensure_initialized(self.visible_width);
    }

    /// Add a command to REPL history
    pub fn add_to_repl_history(&mut self, command: String) {
        self.lua_console.add_to_history(command);
    }

    /// Navigate REPL history up (older commands)
    pub fn repl_history_up(&mut self) -> bool {
        let result = self.lua_console.history_up();
        if result {
            self.lua_console.text_edit_position = self.lua_console.input.len();
        }
        result
    }

    /// Navigate REPL history down (newer commands)
    pub fn repl_history_down(&mut self) -> bool {
        let result = self.lua_console.history_down();
        if result {
            self.lua_console.text_edit_position = self.lua_console.input.len();
        }
        result
    }

    /// Reset REPL history navigation
    pub fn reset_repl_history_navigation(&mut self) {
        self.lua_console.reset_history_navigation();
    }

    /// Check if Lua input is complete
    pub fn is_lua_input_complete(&self) -> bool {
        self.lua_console.is_input_complete()
    }

    /// Load REPL command history from disk
    pub fn load_repl_history(&mut self) {
        if let Some(history_path) = Self::get_repl_history_path() {
            if let Ok(contents) = std::fs::read_to_string(&history_path) {
                self.lua_console.command_history = contents
                    .lines()
                    .map(|line| line.to_string())
                    .filter(|line| !line.is_empty())
                    .collect();
            }
        }
    }

    /// Save REPL command history to disk
    /// Get the path for REPL history file
    fn get_repl_history_path() -> Option<std::path::PathBuf> {
        use xdg::BaseDirectories;

        if let Ok(xdg) = BaseDirectories::with_prefix("tailtales") {
            if let Ok(path) = xdg.place_config_file("history") {
                return Some(path);
            }
        }
        None
    }

    pub fn toggle_mark(&mut self, color: &str) {
        let color = color.to_string();
        let current = self.position - 1; // Convert to 0-based for array access
        let record = self.records.visible_records.get_mut(current).unwrap();
        let current_value = record.get("mark");
        if current_value.is_some() {
            if *current_value.unwrap() == color {
                record.unset_data("mark");
            } else {
                record.set_data("mark", color);
            }
        } else {
            record.set_data("mark", color);
        }
        self.set_position_wrap(self.position as i32 + 1);
    }

    pub fn move_selection(&mut self, delta: i32) {
        // I use i32 all around here as I may get some negatives
        let current = self.position as i32; // position is now 1-based
        let new = current + delta;
        let max = self.records.visible_records.len() as i32;

        if new < 1 {
            self.set_position(1); // Use 1-based indexing
        } else if new > max {
            self.set_position(max as usize); // Use 1-based indexing
        } else {
            self.set_position(new as usize);
        }
    }

    pub fn ensure_visible(&mut self, current: usize) {
        let visible_lines = self.visible_height as i32;
        let current_i32 = current as i32;

        let mut scroll_offset = self.scroll_offset_top as i32;
        // Make scroll_offset follow the selected_record. Must be between the third and the visible lines - 3
        if current_i32 > scroll_offset + visible_lines - 3 {
            scroll_offset = current_i32 - visible_lines + 3;
        }
        if current_i32 < scroll_offset + 3 {
            scroll_offset = current_i32 - 3;
        }
        // offset can not be negative
        if scroll_offset < 0 {
            scroll_offset = 0;
        }

        self.scroll_offset_top = scroll_offset as usize;
    }

    pub fn set_position(&mut self, position: usize) {
        let visible_len = self.records.visible_records.len();
        if visible_len == 0 {
            self.position = 1; // Use 1-based indexing
        } else if position > visible_len {
            self.position = visible_len; // Use 1-based indexing
        } else if position == 0 {
            self.position = 1; // Use 1-based indexing, 0 means go to minimum position
        } else if position < 1 {
            self.position = 1; // Use 1-based indexing
        } else {
            self.position = position;
        }
        self.ensure_visible(self.position - 1); // Convert to 0-based for internal calculations
    }

    pub fn set_position_wrap(&mut self, position: i32) {
        let max = self.records.visible_records.len() as i32;
        if max <= 0 {
            self.position = 1 // Use 1-based indexing
        } else if position > max {
            self.position = 1; // Wrap to first position (1-based)
        } else if position < 1 {
            self.position = max as usize; // Wrap to last position (1-based)
        } else {
            self.position = position as usize;
        }
        self.ensure_visible(self.position - 1); // Convert to 0-based for internal calculations
    }

    pub fn set_vposition(&mut self, position: i32) {
        if position < 0 {
            self.scroll_offset_left = 0;
        } else {
            self.scroll_offset_left = position as usize;
        }
    }

    pub fn exec(&mut self, args: Vec<String>) -> Result<(), String> {
        let mut allargs: Vec<String> = Vec::new();
        allargs.push("-c".to_string());
        // For sh -c, we just join all arguments as a single command string
        // The shell will parse the command and arguments properly
        let command_string = args.join(" ");

        allargs.push(command_string.clone());

        log::debug!("Executing command: sh -c '{}'", command_string);

        // Execute the command inside a shell
        let child = std::process::Command::new("sh")
            .args(&allargs) // Pass the command string to the shell
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match child {
            Ok(mut child) => {
                let exit_code = child.wait();
                match exit_code {
                    Ok(status) => {
                        let code = status.code().unwrap_or(-1);
                        log::debug!("Command exited with code: {}", code);
                        if code != 0 {
                            return Err(format!(
                                "Command '{}' exited with code {}",
                                command_string, code
                            ));
                        }
                    }
                    Err(e) => {
                        let error_msg =
                            format!("Failed to get exit code for '{}': {}", command_string, e);
                        log::error!("{}", error_msg);
                        self.set_warning(error_msg.clone());
                        return Err(error_msg);
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to execute command '{}': {}", command_string, e);
                log::error!("{}", error_msg);
                self.set_warning(error_msg.clone());
                return Err(error_msg);
            }
        };
        Ok(())
    }
    pub fn move_to_next_mark(&mut self) {
        let current = self.position;
        let max = self.records.visible_records.len();

        for new in current + 1..max {
            if self.records.visible_records[new].get("mark").is_some() {
                self.set_position(new);
                return;
            }
        }
        for new in 0..current {
            if self.records.visible_records[new].get("mark").is_some() {
                self.set_position(new);
                return;
            }
        }
        self.set_warning("mark not found".into());
    }

    pub fn move_to_prev_mark(&mut self) {
        let current = self.position;
        let max = self.records.visible_records.len();

        for new in (0..current).rev() {
            if self.records.visible_records[new].get("mark").is_some() {
                self.set_position(new);
                return;
            }
        }
        for new in (current + 1..max).rev() {
            if self.records.visible_records[new].get("mark").is_some() {
                self.set_position(new);
                return;
            }
        }
        self.set_warning("mark not found".into());
    }

    pub fn open_settings(&mut self) {
        let filename: Option<std::path::PathBuf> = Settings::local_settings_filename();

        let filename = if filename.is_none() {
            // Create a settings file
            let _ = self.settings.save_default_settings();

            Settings::local_settings_filename().unwrap()
        } else {
            filename.unwrap()
        };

        // Open the file with xdg-open, using this same terminal, and wait for it to finish.
        // stdin, stderr and stdout are inherited from the parent process.
        let _output = std::process::Command::new("xdg-open")
            .arg(filename.to_str().unwrap())
            .spawn()
            .expect("failed to execute process")
            .wait()
            .expect("failed to wait for process");
    }

    pub fn refresh_screen(&mut self) {
        self.pending_refresh = true;
    }
    pub fn reload_settings(&mut self) {
        let filename = Settings::local_settings_filename().unwrap();
        let result = self.settings.read_from_yaml(filename.to_str().unwrap());
        match result {
            Ok(_) => {
                // Note: Keybinding scripts compilation now needs to be done from Application

                self.current_rule = self
                    .settings
                    .rules
                    .iter()
                    .find(|r| r.name == self.current_rule.name)
                    .unwrap()
                    .clone();
                match load_parsers(&self.current_rule, &mut self.records.parsers) {
                    Ok(_) => {}
                    Err(err) => {
                        self.set_warning(format!("Error loading parsers: {:?}", err));
                    }
                }
                self.records.reparse();
                self.set_warning("Settings reloaded".into());
                self.refresh_screen();
            }
            Err(err) => {
                self.set_warning(format!("Error reloading settings: {}", err));
            }
        }
    }
}

#[cfg(test)]
mod text_wrapping_tests {
    use super::*;
    use crate::lua_console::ConsoleLine;

    #[test]
    fn test_wrap_text_to_width() {
        let mut state = TuiState::new().unwrap();
        state.visible_width = 80; // Set a reasonable width for testing

        // Test short text (should not wrap)
        let short_text = "Hello world";
        let wrapped = state.lua_console.wrap_text_to_width(short_text, 76); // 80 - 4 for borders
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "Hello world");

        // Test long text (should wrap)
        let long_text = "This is a very long line of text that should definitely wrap when the width is limited to a reasonable size for console output";
        let wrapped = state.lua_console.wrap_text_to_width(long_text, 40);
        assert!(wrapped.len() > 1);

        // All wrapped lines should be within the width limit
        for line in &wrapped {
            assert!(
                line.len() <= 40,
                "Line '{}' is too long: {} chars",
                line,
                line.len()
            );
        }

        // Test empty text
        let empty_wrapped = state.lua_console.wrap_text_to_width("", 40);
        assert_eq!(empty_wrapped.len(), 1);
        assert_eq!(empty_wrapped[0], "");

        // Test very long single word (should not break word)
        let long_word = "supercalifragilisticexpialidocious";
        let wrapped_word = state.lua_console.wrap_text_to_width(long_word, 20);
        assert_eq!(wrapped_word.len(), 1);
        assert_eq!(wrapped_word[0], long_word);
    }

    #[test]
    fn test_add_to_lua_console_with_wrapping() {
        let mut state = TuiState::new().unwrap();
        state.visible_width = 50; // Set a small width for testing
        state.lua_console.output_history.clear();

        // Add a long message
        let long_message = "This is a very long error message that should be wrapped across multiple lines when added to the Lua console output history";
        state
            .lua_console
            .add_output(long_message.to_string(), state.visible_width);

        // Should have multiple lines in history
        assert!(state.lua_console.output_history.len() > 1);

        // All lines should be within the width limit
        for console_line in &state.lua_console.output_history {
            let line_len = match console_line {
                ConsoleLine::Stdout(msg) => msg.len(),
                ConsoleLine::Stderr(msg) => msg.len(),
            };
            assert!(
                line_len <= 46,
                "Line '{:?}' is too long: {} chars",
                console_line,
                line_len
            ); // 50 - 4 for borders
        }
    }
}
