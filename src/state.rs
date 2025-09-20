use std::time;

use crate::{
    ast,
    lua_engine::LuaEngine,
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
    pub lua_engine: LuaEngine,
    pub script_prompt: String,
    pub script_input: String,
    pub script_waiting: bool,
}

impl TuiState {
    pub fn new() -> Result<TuiState, Box<dyn std::error::Error>> {
        let settings = Settings::new()?;
        let mut lua_engine =
            LuaEngine::new().map_err(|e| format!("Failed to initialize Lua engine: {}", e))?;

        // Compile keybinding scripts during initialization
        settings
            .compile_keybinding_scripts(&mut lua_engine)
            .map_err(|e| format!("Failed to compile keybinding scripts: {}", e))?;

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
            lua_engine,
            script_prompt: String::new(),
            script_input: String::new(),
            script_waiting: false,
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
    pub fn handle_command(&mut self) {
        let lines: Vec<String> = self.command.lines().map(String::from).collect();
        for line in lines {
            // Execute as Lua script only (try async first for ask() support)
            if let Err(err) = self.handle_lua_command_async(&line) {
                self.set_warning(format!("Error executing Lua command '{}': {}", line, err));
                return;
            }
        }
    }

    /// Handle Lua script execution for commands and keybindings
    #[allow(dead_code)]
    pub fn handle_lua_command(&mut self, script: &str) -> Result<(), String> {
        // Update Lua context with current state
        if let Err(e) = self.lua_engine.update_context(self) {
            return Err(format!("Failed to update Lua context: {}", e));
        }

        // Execute the Lua script and collect commands
        let commands = match self.lua_engine.execute_script_string(script) {
            Ok(commands) => commands,
            Err(e) => return Err(format!("Lua execution error: {}", e)),
        };

        // Process collected commands
        self.process_lua_commands(commands)
    }

    /// Execute a compiled Lua script by name
    #[allow(dead_code)]
    pub fn execute_lua_script(&mut self, script_name: &str) -> Result<(), String> {
        // Update Lua context with current state
        if let Err(e) = self.lua_engine.update_context(self) {
            return Err(format!("Failed to update Lua context: {}", e));
        }

        // Execute the named script and collect commands
        let commands = match self.lua_engine.execute_script(script_name) {
            Ok(commands) => commands,
            Err(e) => {
                return Err(format!(
                    "Lua script '{}' execution error: {}",
                    script_name, e
                ))
            }
        };

        // Process collected commands
        self.process_lua_commands(commands)
    }

    /// Compile and cache a Lua script for later execution
    #[allow(dead_code)]
    pub fn compile_lua_script(&mut self, name: &str, script: &str) -> Result<(), String> {
        match self.lua_engine.compile_script(name, script) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to compile Lua script '{}': {}", name, e)),
        }
    }

    /// Process commands collected from Lua script execution
    fn process_lua_commands(
        &mut self,
        commands: std::collections::HashMap<String, mlua::Value>,
    ) -> Result<(), String> {
        use mlua::Value;

        for (command, value) in commands {
            match command.as_str() {
                "quit" => {
                    self.running = false;
                }
                "warning" => {
                    if let Value::String(msg) = value {
                        let msg_str = match msg.to_str() {
                            Ok(s) => s.to_string(),
                            Err(_) => "".to_string(),
                        };
                        self.set_warning(msg_str);
                    }
                }
                "vmove" => {
                    if let Value::Integer(n) = value {
                        self.move_selection(n as i32);
                    }
                }
                "vgoto" => {
                    if let Value::Integer(n) = value {
                        self.set_position(n as usize);
                    }
                }
                "move_top" => {
                    self.set_position(0);
                    self.set_vposition(0);
                }
                "move_bottom" => {
                    self.set_position(usize::MAX);
                }
                "hmove" => {
                    if let Value::Integer(n) = value {
                        self.set_vposition(self.scroll_offset_left as i32 + n as i32);
                    }
                }
                "search_next" => {
                    self.search_next();
                }
                "search_prev" => {
                    self.search_prev();
                }
                "toggle_mark" => {
                    if let Value::String(color) = value {
                        let color_str = match color.to_str() {
                            Ok(s) => s.to_string(),
                            Err(_) => "yellow".to_string(),
                        };
                        self.toggle_mark(&color_str);
                    }
                }
                "move_to_next_mark" => {
                    self.move_to_next_mark();
                }
                "move_to_prev_mark" => {
                    self.move_to_prev_mark();
                }
                "mode" => {
                    if let Value::String(mode_str) = value {
                        let mode = match mode_str.to_str() {
                            Ok(s) => s.to_string(),
                            Err(_) => "normal".to_string(),
                        };
                        self.set_mode(&mode);
                    }
                }
                "toggle_details" => {
                    self.view_details = !self.view_details;
                }
                "refresh_screen" => {
                    self.refresh_screen();
                }
                "clear" => {
                    self.records.clear();
                    self.position = 0;
                    self.scroll_offset_top = 0;
                    self.scroll_offset_left = 0;
                }
                "clear_records" => {
                    self.records.clear();
                    self.set_position(0);
                    self.set_vposition(0);
                }
                "settings" => {
                    self.open_settings();
                }
                "reload_settings" => {
                    self.reload_settings();
                }
                "exec" => {
                    if let Value::String(cmd) = value {
                        let cmd_str = match cmd.to_str() {
                            Ok(s) => s.to_string(),
                            Err(_) => "".to_string(),
                        };
                        let args: Vec<String> =
                            cmd_str.split_whitespace().map(String::from).collect();
                        if let Err(e) = self.exec(args) {
                            self.set_warning(format!("Exec error: {}", e));
                        }
                    }
                }
                _ => {
                    // Ignore unknown commands for forward compatibility
                }
            }
        }

        Ok(())
    }

    /// Execute a Lua script asynchronously, handling coroutines and user input
    pub fn execute_lua_script_async(&mut self, script_name: &str) -> Result<(), String> {
        // Update Lua context with current state
        if let Err(e) = self.lua_engine.update_context(self) {
            return Err(format!("Failed to update Lua context: {}", e));
        }

        // Execute the script asynchronously
        match self.lua_engine.execute_script_async(script_name) {
            Ok(Some(prompt)) => {
                // Script is asking for user input
                self.script_prompt = prompt;
                self.script_waiting = true;
                self.mode = Mode::ScriptInput;
                self.script_input.clear();
                Ok(())
            }
            Ok(None) => {
                // Script completed immediately, process any collected commands immediately
                self.process_collected_commands(Some(script_name.to_string()))
            }
            Err(e) => Err(format!(
                "Failed to execute async script '{}': {}",
                script_name, e
            )),
        }
    }

    /// Execute a Lua script string asynchronously
    pub fn handle_lua_command_async(&mut self, script: &str) -> Result<(), String> {
        // Update Lua context with current state
        if let Err(e) = self.lua_engine.update_context(self) {
            return Err(format!("Failed to update Lua context: {}", e));
        }

        // Execute the script asynchronously
        match self.lua_engine.execute_script_string_async(script) {
            Ok(Some(prompt)) => {
                // Script is asking for user input
                self.script_prompt = prompt;
                self.script_waiting = true;
                self.mode = Mode::ScriptInput;
                self.script_input.clear();
                Ok(())
            }
            Ok(None) => {
                // Script completed immediately, process any collected commands immediately
                self.process_collected_commands(None)
            }
            Err(e) => Err(format!("Failed to execute async script: {}", e)),
        }
    }

    /// Resume a suspended script with user input
    pub fn resume_suspended_script(&mut self, input: String) -> Result<(), String> {
        if !self.script_waiting {
            return Err("No script is waiting for input".to_string());
        }

        match self.lua_engine.resume_with_input(input) {
            Ok(_) => {
                // Check if the script is asking for more input
                if let Some(new_prompt) = self.lua_engine.get_suspended_prompt() {
                    self.script_prompt = new_prompt.to_string();
                    self.script_input.clear();
                } else {
                    // Script completed, return to normal mode
                    self.script_waiting = false;
                    self.script_prompt.clear();
                    self.script_input.clear();
                    self.mode = Mode::Normal;

                    // Process any commands that were executed immediately
                    self.process_collected_commands(None)?;
                }
                Ok(())
            }
            Err(e) => {
                // Script failed, return to normal mode
                self.script_waiting = false;
                self.script_prompt.clear();
                self.script_input.clear();
                self.mode = Mode::Normal;
                Err(format!("Script execution failed: {}", e))
            }
        }
    }

    /// Cancel the currently suspended script
    pub fn cancel_suspended_script(&mut self) {
        if self.script_waiting {
            self.lua_engine.cancel_suspended_script();
            self.script_waiting = false;
            self.script_prompt.clear();
            self.script_input.clear();
            self.mode = Mode::Normal;
        }
    }

    /// Check if a script is currently waiting for input
    #[allow(dead_code)]
    pub fn has_suspended_script(&self) -> bool {
        self.script_waiting && self.lua_engine.has_suspended_script()
    }

    /// Process collected Lua commands immediately for better performance
    fn process_collected_commands(&mut self, script_name: Option<String>) -> Result<(), String> {
        match self.lua_engine.collect_executed_commands(script_name) {
            Ok(commands) => {
                log::debug!(
                    "Collected {} commands for immediate processing",
                    commands.len()
                );
                for (cmd, value) in &commands {
                    log::debug!("Executing command: {} = {:?}", cmd, value);
                }
                self.process_lua_commands(commands)
            }
            Err(e) => Err(format!("Failed to collect commands: {}", e)),
        }
    }

    /// Execute a script with immediate state modifications enabled
    /// This solves the deferred execution problem by allowing Lua functions to modify state immediately
    pub fn execute_script_with_immediate_execution(&mut self, script: &str) -> Result<(), String> {
        // Update context before execution
        if let Err(e) = self.lua_engine.update_context(self) {
            return Err(format!("Failed to update Lua context: {}", e));
        }

        // Get a pointer to self for immediate execution
        let state_ptr = self as *mut TuiState;

        // Register immediate execution functions
        self.lua_engine
            .register_immediate_functions(state_ptr)
            .map_err(|e| format!("Failed to register immediate functions: {}", e))?;

        // Enable immediate mode
        self.lua_engine.set_immediate_mode(true);

        // Create a script that uses immediate functions instead of deferred ones
        let immediate_script = script
            .replace("vgoto(", "vgoto_immediate(")
            .replace("vmove(", "vmove_immediate(")
            .replace("warning(", "warning_immediate(");

        // Execute the script
        let result = match self
            .lua_engine
            .execute_script_string_async(&immediate_script)
        {
            Ok(Some(prompt)) => {
                // Script is asking for user input
                self.script_prompt = prompt;
                self.script_waiting = true;
                self.mode = Mode::ScriptInput;
                self.script_input.clear();
                Ok(())
            }
            Ok(None) => {
                // Script completed immediately - no need to process commands since they executed immediately
                Ok(())
            }
            Err(e) => Err(format!("Failed to execute immediate script: {}", e)),
        };

        // Clean up immediate functions
        let _ = self.lua_engine.clear_immediate_functions();
        self.lua_engine.set_immediate_mode(false);

        result
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
            _ => {
                self.set_warning(format!("Unknown mode: {}", mode));
            }
        }
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
                // Recompile keybinding scripts after settings reload
                if let Err(e) = self
                    .settings
                    .compile_keybinding_scripts(&mut self.lua_engine)
                {
                    self.set_warning(format!("Failed to recompile keybinding scripts: {}", e));
                    return;
                }

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

    /// Test method to demonstrate basic Lua execution
    /// Test method for Lua execution functionality
    #[allow(dead_code)]
    pub fn test_lua_execution(&mut self) -> Result<(), String> {
        // Update Lua context with current state
        self.lua_engine
            .update_context(self)
            .map_err(|e| format!("Failed to update Lua context: {}", e))?;

        // Test basic Lua execution
        self.lua_engine
            .test_lua_execution()
            .map_err(|e| format!("Failed to execute Lua test: {}", e))?;

        // Test executing a simple Lua command
        let result = self
            .lua_engine
            .execute_script_string("warning('Hello from Lua!')")
            .map_err(|e| format!("Failed to execute Lua script: {}", e))?;

        println!("Lua script result: {:?}", result);
        Ok(())
    }
}
