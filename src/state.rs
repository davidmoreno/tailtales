use std::time;

use crate::{
    ast,
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
    // Lua REPL state
    pub repl_input: String,
    pub repl_output_history: Vec<String>,
    pub repl_scroll_offset: usize,
    pub repl_multiline_buffer: Vec<String>,
    pub repl_is_multiline: bool,
    pub repl_command_history: Vec<String>,
    pub repl_history_index: Option<usize>,
    pub repl_temp_input: String,
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
            position: 0,
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
            // Initialize REPL state
            repl_input: String::new(),
            repl_output_history: Vec::new(),
            repl_scroll_offset: 0,
            repl_multiline_buffer: Vec::new(),
            repl_is_multiline: false,
            repl_command_history: Vec::new(),
            repl_history_index: None,
            repl_temp_input: String::new(),
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
        let mut current = self.position;

        let maybe_position = self.records.search_forward(search_ast, current);
        if maybe_position.is_none() {
            return false;
        }
        current = maybe_position.unwrap();
        self.set_position(current);
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
        let mut current = self.position;

        let maybe_position = self.records.search_backwards(search_ast, current);
        if maybe_position.is_none() {
            return false;
        }
        current = maybe_position.unwrap();
        self.set_position(current);
        true
    }

    pub fn handle_filter(&mut self) {
        let parsed = ast::parse(&self.filter);
        match parsed {
            Ok(parsed) => {
                self.records.filter_parallel(parsed);
                self.set_position(0);
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
                // Add welcome message if REPL history is empty
                if self.repl_output_history.is_empty() {
                    self.repl_output_history.push(
                        "Welcome to Lua REPL! Type Lua code and press Enter to execute."
                            .to_string(),
                    );
                    self.repl_output_history.push(
                        "Supports multiline input: functions, if/do blocks, etc.".to_string(),
                    );
                    self.repl_output_history.push(
                        "Use print() to output text, dir() to explore, help() for assistance."
                            .to_string(),
                    );
                    self.repl_output_history
                        .push("Press Esc to exit, Ctrl+C to cancel multiline input.".to_string());
                    self.repl_output_history
                        .push("Use ↑/↓ arrows to navigate command history.".to_string());
                    self.repl_output_history.push("".to_string());
                }

                // Load command history from disk when entering REPL mode
                self.load_repl_history();
                self.reset_repl_history_navigation();
            }
            _ => {
                self.set_warning(format!("Unknown mode: {}", mode));
            }
        }
    }

    /// Check if the current Lua input is complete or needs more lines
    pub fn is_lua_input_complete(&self) -> bool {
        let mut all_input = self.repl_multiline_buffer.join("\n");
        if !all_input.is_empty() {
            all_input.push('\n');
        }
        all_input.push_str(&self.repl_input);

        // Simple heuristic: count various brackets and keywords
        let mut paren_count = 0;
        let mut bracket_count = 0;
        let mut brace_count = 0;
        let mut do_count = 0;
        let mut end_count = 0;
        let mut if_count = 0;
        let mut function_count = 0;
        let mut for_count = 0;
        let mut while_count = 0;
        let mut repeat_count = 0;
        let mut until_count = 0;

        // Simple tokenization - split by whitespace and check each token
        let tokens: Vec<&str> = all_input.split_whitespace().collect();

        for token in tokens {
            // Count brackets/parentheses
            for ch in token.chars() {
                match ch {
                    '(' => paren_count += 1,
                    ')' => paren_count -= 1,
                    '[' => bracket_count += 1,
                    ']' => bracket_count -= 1,
                    '{' => brace_count += 1,
                    '}' => brace_count -= 1,
                    _ => {}
                }
            }

            // Count keywords (simple exact match)
            match token {
                "do" => do_count += 1,
                "end" => end_count += 1,
                "if" => if_count += 1,
                "function" => function_count += 1,
                "for" => for_count += 1,
                "while" => while_count += 1,
                "repeat" => repeat_count += 1,
                "until" => until_count += 1,
                _ => {}
            }
        }

        // Check if all constructs are balanced
        let blocks_balanced =
            (do_count + if_count + function_count + for_count + while_count + repeat_count)
                <= (end_count + until_count);
        let brackets_balanced = paren_count == 0 && bracket_count == 0 && brace_count == 0;

        // Input is complete if brackets are balanced and blocks are balanced
        brackets_balanced && blocks_balanced
    }

    /// Load REPL command history from disk
    pub fn load_repl_history(&mut self) {
        if let Some(history_path) = Self::get_repl_history_path() {
            if let Ok(contents) = std::fs::read_to_string(&history_path) {
                self.repl_command_history = contents
                    .lines()
                    .map(|line| line.to_string())
                    .filter(|line| !line.is_empty())
                    .collect();
            }
        }
    }

    /// Save REPL command history to disk
    pub fn save_repl_history(&self) {
        if let Some(history_path) = Self::get_repl_history_path() {
            if let Some(parent) = history_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let contents = self.repl_command_history.join("\n");
            let _ = std::fs::write(&history_path, contents);
        }
    }

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

    /// Add a command to REPL history
    pub fn add_to_repl_history(&mut self, command: String) {
        if command.trim().is_empty() {
            return;
        }

        // Don't add duplicate consecutive commands
        if let Some(last) = self.repl_command_history.last() {
            if last == &command {
                return;
            }
        }

        self.repl_command_history.push(command);

        // Keep history size reasonable (last 1000 commands)
        if self.repl_command_history.len() > 1000 {
            self.repl_command_history.drain(0..500);
        }

        // Save to disk
        self.save_repl_history();
    }

    /// Navigate REPL history up (older commands)
    pub fn repl_history_up(&mut self) -> bool {
        if self.repl_command_history.is_empty() {
            return false;
        }

        match self.repl_history_index {
            None => {
                // First time accessing history - save current input and go to most recent
                self.repl_temp_input = self.repl_input.clone();
                self.repl_history_index = Some(self.repl_command_history.len() - 1);
                self.repl_input =
                    self.repl_command_history[self.repl_history_index.unwrap()].clone();
                self.text_edit_position = self.repl_input.len();
                true
            }
            Some(index) => {
                if index > 0 {
                    self.repl_history_index = Some(index - 1);
                    self.repl_input = self.repl_command_history[index - 1].clone();
                    self.text_edit_position = self.repl_input.len();
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Navigate REPL history down (newer commands)
    pub fn repl_history_down(&mut self) -> bool {
        match self.repl_history_index {
            None => false,
            Some(index) => {
                if index < self.repl_command_history.len() - 1 {
                    self.repl_history_index = Some(index + 1);
                    self.repl_input = self.repl_command_history[index + 1].clone();
                    self.text_edit_position = self.repl_input.len();
                    true
                } else {
                    // Back to current input
                    self.repl_history_index = None;
                    self.repl_input = self.repl_temp_input.clone();
                    self.text_edit_position = self.repl_input.len();
                    true
                }
            }
        }
    }

    /// Reset REPL history navigation
    pub fn reset_repl_history_navigation(&mut self) {
        self.repl_history_index = None;
        self.repl_temp_input.clear();
    }

    pub fn toggle_mark(&mut self, color: &str) {
        let color = color.to_string();
        let current = self.position;
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
        let mut current = self.position as i32;
        let mut new = current as i32 + delta;
        let max = self.records.visible_records.len() as i32 - 1;

        if new <= 0 {
            new = 0;
        }
        if new > max {
            new = max;
        }
        current = new;

        self.set_position(current as usize);
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
            self.position = 0;
        } else if position >= visible_len {
            self.position = visible_len - 1;
        } else {
            self.position = position;
        }
        self.ensure_visible(self.position);
    }

    pub fn set_position_wrap(&mut self, position: i32) {
        let max = self.records.visible_records.len() as i32;
        if max <= 1 {
            self.position = 0
        } else if position >= max {
            self.position = 0;
        } else if position < 0 {
            self.position = (max - 1) as usize;
        } else {
            self.position = position as usize;
        }
        self.ensure_visible(self.position);
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
        allargs.push(
            args.iter()
                .map(|arg| {
                    if arg.contains(" ") {
                        format!("\"{}\"", arg.replace("\"", "\\\""))
                    } else {
                        arg.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" "),
        );

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
                        if status.code().unwrap_or(0) != 0 {
                            return Err(format!(
                                "Command exited with code {}",
                                status.code().unwrap_or(0)
                            ));
                        }
                    }
                    Err(e) => {
                        self.set_warning(format!("Failed to get exit code: {}", e));
                        return Err(format!("Failed to get exit code: {}", e));
                    }
                }
            }
            Err(e) => {
                self.set_warning(format!(
                    "Failed to execute command: {}. Error: {}",
                    &allargs[1], e
                ));
                return Err(format!(
                    "Failed to execute command: {}. Error: {}",
                    &allargs[1], e
                ));
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

    pub fn get_completions(&self) -> (String, Vec<String>) {
        let current = self.command.trim();

        // Lua function names for completion
        let mut completions: Vec<&str> = vec![
            "quit()",
            "warning(",
            "vmove(",
            "vgoto(",
            "move_top()",
            "move_bottom()",
            "hmove(",
            "search_next()",
            "search_prev()",
            "toggle_mark(",
            "move_to_next_mark()",
            "move_to_prev_mark()",
            "mode(",
            "toggle_details()",
            "exec(",
            "refresh_screen()",
            "clear()",
            "clear_records()",
            "settings()",
            "reload_settings()",
            "ask(",
            "url_encode(",
            "url_decode(",
            "escape_shell(",
            "debug_log(",
            "current.",
            "app.",
        ];

        completions.retain(|&c| c.starts_with(current));
        let completions: Vec<String> = completions.iter().map(|&s| s.to_string()).collect();

        // Find common prefix
        let mut common_prefix = String::new();
        if completions.len() == 1 {
            common_prefix = completions[0].to_string();
        } else if completions.len() > 1 {
            let mut pos = 0;
            let mut done = false;

            while !done {
                let mut c: Option<char> = None;
                for completion in &completions {
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
        }
        return (common_prefix, completions);
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
