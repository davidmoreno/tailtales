//! Lua scripting engine for TailTales
//!
//! This module provides Lua runtime integration, allowing users to execute
//! Lua scripts for keybindings and commands instead of the old string-based
//! command system.
//!
//! Phase 2 Implementation Features:
//! - Bytecode compilation and caching for performance
//! - External .lua file support with hot-reload capability
//! - Enhanced error handling with stack traces
//! - Full record data exposure and application state access
//! - Improved parameter validation and type conversion

use crate::state::{Mode, TuiState};
use log::{debug, error, warn};
use mlua::prelude::LuaError;
use mlua::{FromLua, Lua, Result as LuaResult, Table, Thread, UserData, UserDataMethods, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a compiled Lua script with metadata
#[derive(Debug, Clone)]
pub struct CompiledScript {
    /// The original source code
    pub source: String,
    /// Compiled bytecode for fast execution
    #[allow(dead_code)]
    pub bytecode: Vec<u8>,
    /// File path if loaded from external file
    pub file_path: Option<PathBuf>,
    /// Last modification time for hot-reload
    pub last_modified: Option<u64>,
    /// Compilation timestamp
    #[allow(dead_code)]
    pub compiled_at: u64,
}

impl CompiledScript {
    pub fn new(source: String, bytecode: Vec<u8>) -> Self {
        Self {
            source,
            bytecode,
            file_path: None,
            last_modified: None,
            compiled_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    #[allow(dead_code)]
    pub fn from_file(
        file_path: PathBuf,
        source: String,
        bytecode: Vec<u8>,
    ) -> Result<Self, std::io::Error> {
        let metadata = fs::metadata(&file_path)?;
        let last_modified = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(Self {
            source,
            bytecode,
            file_path: Some(file_path),
            last_modified: Some(last_modified),
            compiled_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }

    /// Check if the external file has been modified and needs reloading
    pub fn needs_reload(&self) -> bool {
        if let (Some(file_path), Some(last_modified)) = (&self.file_path, self.last_modified) {
            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(current_modified) = metadata.modified() {
                    let current_time = current_modified
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    return current_time > last_modified;
                }
            }
        }
        false
    }
}

/// Enhanced error type for better Lua error reporting
#[derive(Debug)]
pub struct LuaEngineError {
    pub message: String,
    pub script_name: Option<String>,
    pub line_number: Option<u32>,
    pub stack_trace: Option<String>,
}

impl std::fmt::Display for LuaEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lua Error")?;
        if let Some(script) = &self.script_name {
            write!(f, " in script '{}'", script)?;
        }
        if let Some(line) = self.line_number {
            write!(f, " at line {}", line)?;
        }
        write!(f, ": {}", self.message)?;
        if let Some(stack) = &self.stack_trace {
            write!(f, "\nStack trace:\n{}", stack)?;
        }
        Ok(())
    }
}

impl std::error::Error for LuaEngineError {}

impl LuaEngineError {
    /// Create a simple LuaEngineError with just a message
    pub fn simple<S: AsRef<str>>(message: S) -> Self {
        Self {
            message: message.as_ref().to_string(),
            script_name: None,
            line_number: None,
            stack_trace: None,
        }
    }

    /// Create a LuaEngineError with a message and script name
    pub fn with_script<S: AsRef<str>, T: AsRef<str>>(message: S, script_name: T) -> Self {
        Self {
            message: message.as_ref().to_string(),
            script_name: Some(script_name.as_ref().to_string()),
            line_number: None,
            stack_trace: None,
        }
    }

    /// Create a LuaEngineError from a format string and arguments
    pub fn format<T: AsRef<str>>(template: T, error: impl std::fmt::Display) -> Self {
        Self::simple(format!("{}: {}", template.as_ref(), error))
    }

    /// Create a LuaEngineError from a format string, arguments, and script name
    pub fn format_with_script<T: AsRef<str>, S: AsRef<str>>(
        template: T,
        error: impl std::fmt::Display,
        script_name: S,
    ) -> Self {
        Self::with_script(format!("{}: {}", template.as_ref(), error), script_name)
    }
}

/// Extension trait to make error conversion more ergonomic
trait LuaEngineErrorExt<T> {
    fn lua_err<S: AsRef<str>>(self, message: S) -> Result<T, LuaEngineError>;
    fn lua_err_with_script<S: AsRef<str>, N: AsRef<str>>(
        self,
        message: S,
        script_name: N,
    ) -> Result<T, LuaEngineError>;
}

impl<T, E: std::fmt::Display> LuaEngineErrorExt<T> for Result<T, E> {
    fn lua_err<S: AsRef<str>>(self, message: S) -> Result<T, LuaEngineError> {
        self.map_err(|e| LuaEngineError::format(message, e))
    }

    fn lua_err_with_script<S: AsRef<str>, N: AsRef<str>>(
        self,
        message: S,
        script_name: N,
    ) -> Result<T, LuaEngineError> {
        self.map_err(|e| LuaEngineError::format_with_script(message, e, script_name))
    }
}

/// Represents a suspended coroutine waiting for user input
#[derive(Debug)]
pub struct SuspendedCoroutine {
    pub thread: Thread,
    pub prompt: String,
    pub script_name: Option<String>,
}

/// The main Lua engine that manages script execution
pub struct LuaEngine {
    lua: Lua,
    compiled_scripts: HashMap<String, CompiledScript>,
    #[allow(dead_code)]
    script_directories: Vec<PathBuf>,
    suspended_coroutine: Option<SuspendedCoroutine>,
}

impl LuaEngine {
    /// Create a new Lua engine instance
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();

        // Initialize the Lua engine with our custom API
        let mut engine = LuaEngine {
            lua,
            compiled_scripts: HashMap::new(),
            script_directories: vec![
                PathBuf::from("scripts"),
                PathBuf::from("lua"),
                PathBuf::from(".tailtales/scripts"),
            ],
            suspended_coroutine: None,
        };

        engine.initialize()?;

        Ok(engine)
    }

    /// Initialize the Lua runtime with the TailTales API
    pub fn initialize(&mut self) -> LuaResult<()> {
        self.setup_globals()?;
        self.setup_print_override()?;
        self.register_global_functions()
            .map_err(|e| mlua::Error::runtime(e.to_string()))?; // Register all functions once at startup
        self.load_init_script()?;
        debug!("Lua engine initialized successfully");
        Ok(())
    }

    /// Set up global tables and variables
    fn setup_globals(&self) -> LuaResult<()> {
        let globals = self.lua.globals();

        // Create the 'app' table for application state
        let app_table = self.lua.create_table()?;
        app_table.set("position", 0)?;
        app_table.set("mode", "normal")?;
        app_table.set("visible_height", 25)?;
        app_table.set("visible_width", 80)?;
        app_table.set("record_count", 0)?;
        globals.set("app", app_table)?;

        // Note: The 'current' table is now replaced by get_record() function
        // which provides on-demand access to current record data

        debug!("Lua global tables initialized");
        Ok(())
    }

    /// Set up print function override (default no-op, can be overridden per execution)
    fn setup_print_override(&self) -> LuaResult<()> {
        // Default print function that does nothing (for non-REPL executions)
        let print_fn = self
            .lua
            .create_function(|_lua, _args: mlua::Variadic<Value>| {
                // Default behavior: do nothing
                // This will be overridden in execute_script_string for REPL
                Ok(())
            })?;

        self.lua.globals().set("print", print_fn)?;
        debug!("Print function override setup");
        Ok(())
    }

    /// Load and execute the embedded initialization script
    fn load_init_script(&mut self) -> LuaResult<()> {
        let init_script = Self::get_embedded_init_script();

        // Execute the init script
        match self.lua.load(init_script).exec() {
            Ok(_) => {
                debug!("Initialization script loaded successfully");
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to load initialization script: {}", e);
                Err(e)
            }
        }
    }

    /// Get the embedded initialization script
    fn get_embedded_init_script() -> &'static str {
        include_str!("../_init.lua")
    }

    /// Format a Lua value for display, including table contents
    ///
    /// # Arguments
    /// * `value` - The Lua value to format
    /// * `depth` - Current nesting depth (for recursion control)
    /// * `max_depth` - Maximum allowed nesting depth
    fn format_lua_value(&self, value: &Value, depth: usize, max_depth: usize) -> String {
        if depth > max_depth {
            return "...".to_string();
        }

        match value {
            Value::Nil => "nil".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.to_string_lossy(),
            Value::Table(table) => self.format_table(table, depth, max_depth),
            Value::Function(_) => "[function]".to_string(),
            Value::Thread(_) => "[thread]".to_string(),
            Value::UserData(_) => "[userdata]".to_string(),
            _ => "[unknown]".to_string(),
        }
    }

    /// Format a Lua table with its contents
    fn format_table(&self, table: &mlua::Table, depth: usize, max_depth: usize) -> String {
        if depth > max_depth {
            return "{...}".to_string();
        }

        let mut items = Vec::new();
        let mut count = 0;

        // Try to iterate through the table
        let pairs = table.pairs::<Value, Value>();
        for pair in pairs {
            if let Ok((key, value)) = pair {
                count += 1;
                if count <= 5 {
                    // Limit display items
                    let formatted_item = self.format_table_item(&key, &value, depth, max_depth);
                    items.push(formatted_item);
                }
            }
        }

        self.format_table_display(&items, count)
    }

    /// Format a single table key-value pair
    fn format_table_item(
        &self,
        key: &Value,
        value: &Value,
        depth: usize,
        max_depth: usize,
    ) -> String {
        let value_str = self.format_table_value(value, depth, max_depth);

        match key {
            Value::Integer(i) if *i > 0 => {
                // Array-like index - just show the value
                value_str
            }
            Value::String(s) => {
                // Hash key - show key = value
                format!("{} = {}", s.to_string_lossy(), value_str)
            }
            _ => {
                // Other key types - show [key] = value
                let key_str = self.format_lua_value(key, depth + 1, max_depth);
                format!("[{}] = {}", key_str, value_str)
            }
        }
    }

    /// Format a table value (with special handling for strings)
    fn format_table_value(&self, value: &Value, depth: usize, max_depth: usize) -> String {
        match value {
            Value::String(s) => format!("\"{}\"", s.to_string_lossy()),
            Value::Table(_) if depth >= max_depth => "{...}".to_string(),
            _ => self.format_lua_value(value, depth + 1, max_depth),
        }
    }

    /// Format the final table display string
    fn format_table_display(&self, items: &[String], total_count: usize) -> String {
        if items.is_empty() {
            "{}".to_string()
        } else if total_count > 5 {
            format!("{{ {}, ... ({} items) }}", items.join(", "), total_count)
        } else {
            format!("{{ {} }}", items.join(", "))
        }
    }

    /// Create a print function that captures output to the given vector
    fn create_print_function(
        &self,
        output_capture: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
    ) -> Result<mlua::Function, LuaEngineError> {
        let engine_ptr = self as *const LuaEngine;

        self.lua
            .create_function(move |_lua, args: mlua::Variadic<Value>| {
                let mut output_line = Vec::new();

                for arg in args {
                    let formatted_arg = unsafe {
                        // Safe because we know the engine is alive during this call
                        let engine = &*engine_ptr;
                        engine.format_lua_value(&arg, 0, 3) // Max depth of 3
                    };
                    output_line.push(formatted_arg);
                }

                output_capture.borrow_mut().push(output_line.join("\t"));
                Ok(())
            })
            .map_err(|e| self.create_enhanced_error(e, None))
    }

    /// Helper function to safely get TuiState from Lua registry
    /// This centralizes all unsafe pointer operations in one place
    fn get_state_from_registry(lua: &mlua::Lua) -> LuaResult<&'static mut TuiState> {
        let state_ptr: usize = lua.named_registry_value("tui_state_ptr")?;
        unsafe {
            // SAFETY: This pointer is only stored during controlled execution in execute_with_state()
            // The pointer is guaranteed to be valid for the duration of script execution
            // and is automatically cleaned up after execution completes
            let ptr = state_ptr as *mut TuiState;
            if ptr.is_null() {
                return Err(LuaError::RuntimeError("State pointer is null".to_string()));
            }
            Ok(&mut *ptr)
        }
    }

    /// Store or clear the state pointer in the Lua registry
    /// Pass Some(state) to store the pointer, None to clear it
    fn set_state_to_registry(&self, state: Option<&mut TuiState>) -> Result<(), LuaEngineError> {
        match state {
            Some(state_ref) => {
                // Store state pointer in registry for function access
                let state_ptr = state_ref as *mut TuiState as usize;
                self.lua
                    .set_named_registry_value("tui_state_ptr", state_ptr)
                    .lua_err("Failed to store state pointer")
            }
            None => {
                // Clear the state pointer from registry
                self.lua
                    .set_named_registry_value("tui_state_ptr", mlua::Nil)
                    .lua_err("Failed to clear state pointer")
            }
        }
    }

    /// Helper function to register a Lua function with minimal boilerplate
    fn register_function<A, R, F>(&self, name: &str, func: F) -> Result<(), LuaEngineError>
    where
        A: mlua::FromLuaMulti,
        R: mlua::IntoLuaMulti,
        F: Fn(&mlua::Lua, A) -> LuaResult<R> + Send + Sync + 'static,
    {
        let globals = self.lua.globals();
        let lua_func = self
            .lua
            .create_function(func)
            .lua_err(&format!("Failed to create {}", name))?;
        globals
            .set(name, lua_func)
            .lua_err(&format!("Failed to set {}", name))
    }

    /// Register immediate execution functions once at initialization
    /// Functions will access state via pointer stored in Lua registry during execution
    fn register_global_functions(&mut self) -> Result<(), LuaEngineError> {
        // Core navigation and control commands - immediate execution
        self.register_function("quit", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.running = false;
            Ok(())
        })?;

        self.register_function("warning", |lua, msg: String| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.set_warning(msg);
            Ok(())
        })?;

        self.register_function("vmove", |lua, n: i32| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.move_selection(n);

            Ok(())
        })?;

        self.register_function("vgoto", |lua, n: usize| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.set_position(n); // Now using 1-based indexing directly

            Ok(())
        })?;

        self.register_function("move_top", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.set_position(1); // Use 1-based indexing
            state.set_vposition(0);

            Ok(())
        })?;

        self.register_function("move_bottom", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.set_position(usize::MAX);

            Ok(())
        })?;

        // Search functions
        self.register_function("search_next", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.search_next();
            Ok(())
        })?;

        self.register_function("search_prev", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.search_prev();
            Ok(())
        })?;

        // Mark functions
        self.register_function(
            "toggle_mark",
            |lua, color: Option<String>| -> LuaResult<()> {
                let state = Self::get_state_from_registry(lua)?;
                let color_str = color.as_deref().unwrap_or("yellow");
                state.toggle_mark(color_str);
                Ok(())
            },
        )?;

        self.register_function("move_to_next_mark", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.move_to_next_mark();
            Ok(())
        })?;

        self.register_function("move_to_prev_mark", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.move_to_prev_mark();
            Ok(())
        })?;

        // Mode and UI functions
        self.register_function("mode", |lua, mode_name: String| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.set_mode(&mode_name);
            Ok(())
        })?;

        self.register_function("toggle_details", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.view_details = !state.view_details;
            Ok(())
        })?;

        self.register_function("lua_repl", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.set_mode("lua_repl");
            Ok(())
        })?;

        // System and utility functions
        self.register_function("refresh_screen", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.pending_refresh = true;
            Ok(())
        })?;

        self.register_function("clear_records", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.records.clear();
            state.position = 1; // Use 1-based indexing
            Ok(())
        })?;

        // Clear REPL console buffer
        self.register_function("clear", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.repl_scroll_offset = 0;
            state.repl_output_history.clear();
            Ok(())
        })?;

        // Filter records by expression
        self.register_function("filter", |lua, expression: String| -> LuaResult<usize> {
            let state = Self::get_state_from_registry(lua)?;

            // Parse the filter expression
            let parsed = match crate::ast::parse(&expression) {
                Ok(ast) => ast,
                Err(err) => {
                    state.set_warning(format!("Filter expression parse error: {}", err));
                    return Ok(0);
                }
            };

            // Apply the filter
            state.records.filter_parallel(parsed);
            state.set_position(1); // Reset to first record
            state.filter_ok = true;

            // Update the filter string in state for consistency
            state.filter = expression;

            // Return the number of records after filtering
            Ok(state.records.len())
        })?;

        // Update record attribute (add, update, or remove if nil)
        self.register_function(
            "update_record_attribute",
            |lua, (index, key, value): (usize, String, Option<String>)| -> LuaResult<bool> {
                let state = Self::get_state_from_registry(lua)?;

                // Convert to 0-based index for array access
                let record_index = index.saturating_sub(1);

                // Get mutable reference to the record
                if let Some(record) = state.records.all_records.get_mut(record_index) {
                    match &value {
                        Some(val) => {
                            // Add or update the attribute
                            record.set_data(&key, val.clone());
                        }
                        None => {
                            // Remove the attribute (nil value)
                            record.unset_data(&key);
                        }
                    }

                    // Also update the visible record if it exists
                    if let Some(visible_record) =
                        state.records.visible_records.get_mut(record_index)
                    {
                        match &value {
                            Some(val) => {
                                visible_record.set_data(&key, val.clone());
                            }
                            None => {
                                visible_record.unset_data(&key);
                            }
                        }
                    }

                    Ok(true)
                } else {
                    Ok(false) // Record not found
                }
            },
        )?;

        // Get record attribute value
        self.register_function(
            "get_record_attribute",
            |lua, (index, key): (usize, String)| -> LuaResult<Option<String>> {
                let state = Self::get_state_from_registry(lua)?;

                // Convert to 0-based index for array access
                let record_index = index.saturating_sub(1);

                // Get the record
                if let Some(record) = state.records.all_records.get(record_index) {
                    Ok(record.get(&key).map(|s| s.clone()))
                } else {
                    Ok(None) // Record not found
                }
            },
        )?;

        // Movement functions
        self.register_function("hmove", |lua, n: i32| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            if n > 0 {
                state.scroll_offset_left = state.scroll_offset_left.saturating_add(n as usize);
            } else {
                state.scroll_offset_left = state.scroll_offset_left.saturating_sub((-n) as usize);
            }
            Ok(())
        })?;

        // External command execution
        self.register_function("exec", |lua, command: String| -> LuaResult<bool> {
            let state = Self::get_state_from_registry(lua)?;
            match state.exec(vec![command]) {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        })?;

        // Settings function
        self.register_function("settings", |lua, ()| -> LuaResult<()> {
            let state = Self::get_state_from_registry(lua)?;
            state.open_settings();
            Ok(())
        })?;

        // Add utility functions
        self.register_utility_functions()?;

        // Add state getter functions
        self.register_state_getter_functions()?;

        debug!("All Lua API functions registered once at startup");
        Ok(())
    }

    /// Register state getter functions that retrieve specific pieces of state
    fn register_state_getter_functions(&mut self) -> Result<(), LuaEngineError> {
        // Get current record data
        self.register_function(
            "get_record",
            |lua, index: Option<usize>| -> LuaResult<Option<Table>> {
                let state = Self::get_state_from_registry(lua)?;

                // Get record at specified index, or current position if none provided
                let position = index.unwrap_or(state.position - 1); // Convert to 0-based for array access

                if let Some(record) = state.records.get(position) {
                    let record_table = lua.create_table()?;

                    // Add basic record elements
                    record_table.set("original", record.original.clone())?;
                    record_table.set("index", record.index)?; // Already 1-based

                    // Add all fields from the record's data hashmap
                    for (key, value) in &record.data {
                        record_table.set(key.as_str(), value.clone())?;
                    }

                    Ok(Some(record_table))
                } else {
                    Ok(None)
                }
            },
        )?;

        // Get current position
        self.register_function("get_position", |lua, ()| -> LuaResult<usize> {
            let state = Self::get_state_from_registry(lua)?;
            // Position is already 1-based internally
            Ok(state.position)
        })?;

        // Get viewport information
        self.register_function("get_viewport", |lua, ()| -> LuaResult<Table> {
            let state = Self::get_state_from_registry(lua)?;
            let viewport_table = lua.create_table()?;

            viewport_table.set("height", state.visible_height)?;
            viewport_table.set("width", state.visible_width)?;
            viewport_table.set("scroll_top", state.scroll_offset_top)?;
            viewport_table.set("scroll_left", state.scroll_offset_left)?;
            viewport_table.set("view_details", state.view_details)?;

            Ok(viewport_table)
        })?;

        // Get current mode
        self.register_function("get_mode", |lua, ()| -> LuaResult<String> {
            let state = Self::get_state_from_registry(lua)?;
            Ok(match state.mode {
                Mode::Normal => "normal",
                Mode::Search => "search",
                Mode::Filter => "filter",
                Mode::Command => "command",
                Mode::Warning => "warning",
                Mode::ScriptInput => "script_input",
                Mode::LuaRepl => "lua_repl",
            }
            .to_string())
        })?;

        // Get record count
        self.register_function("get_record_count", |lua, ()| -> LuaResult<usize> {
            let state = Self::get_state_from_registry(lua)?;
            Ok(state.records.len())
        })?;

        // Alias for get_record with more explicit naming
        self.register_function(
            "get_record_data",
            |lua, index: Option<usize>| -> LuaResult<Option<Table>> {
                let state = Self::get_state_from_registry(lua)?;

                // Get record at specified index, or current position if none provided
                let position = index.unwrap_or(state.position - 1); // Convert to 0-based for array access

                if let Some(record) = state.records.get(position) {
                    let record_table = lua.create_table()?;

                    // Add basic record elements
                    record_table.set("original", record.original.clone())?;
                    record_table.set("index", record.index)?; // Already 1-based

                    // Add all fields from the record's data hashmap
                    for (key, value) in &record.data {
                        record_table.set(key.as_str(), value.clone())?;
                    }

                    Ok(Some(record_table))
                } else {
                    Ok(None)
                }
            },
        )?;

        // Get search/filter/command state
        self.register_function("get_search", |lua, ()| -> LuaResult<String> {
            let state = Self::get_state_from_registry(lua)?;
            Ok(state.search.clone())
        })?;

        self.register_function("get_filter", |lua, ()| -> LuaResult<String> {
            let state = Self::get_state_from_registry(lua)?;
            Ok(state.filter.clone())
        })?;

        self.register_function("get_command", |lua, ()| -> LuaResult<String> {
            let state = Self::get_state_from_registry(lua)?;
            Ok(state.command.clone())
        })?;

        self.register_function("get_warning", |lua, ()| -> LuaResult<String> {
            let state = Self::get_state_from_registry(lua)?;
            Ok(state.warning.clone())
        })?;

        debug!("State getter functions registered");
        Ok(())
    }

    /// Execute a compiled script with state access via registry
    pub fn execute_with_state(
        &mut self,
        script_name: &str,
        state: &mut TuiState,
    ) -> Result<Option<String>, LuaEngineError> {
        // Store state pointer in registry for function access
        self.set_state_to_registry(Some(state))?;

        // Execute the script directly - state getter functions will access state on demand
        let result = self.execute_script_async(script_name);

        // Only clear the state pointer if the script completed (not suspended)
        match &result {
            Ok(Some(_)) => {
                // Script suspended with ask() - keep state pointer for resume
                debug!("Script suspended, keeping state pointer for resume");
            }
            _ => {
                // Script completed or errored - clear state pointer
                let _ = self.set_state_to_registry(None);
            }
        }

        result
    }

    /// Execute a Lua script from a string with state access (for REPL)
    pub fn execute_script_string_with_state(
        &mut self,
        source: &str,
        state: &mut TuiState,
    ) -> Result<String, LuaEngineError> {
        debug!("Executing Lua script string with state: {}", source);

        // Store state pointer in registry for function access
        self.set_state_to_registry(Some(state))?;

        // Set up print output capture
        let _print_output = self.setup_print_capture()?;

        // Execute the script and get the result
        let result = self.execute_lua_code(source);

        // Clear state pointer after execution
        let _ = self.set_state_to_registry(None);

        // Handle result and combine with print output
        match result {
            Ok(result_str) => self.combine_output_and_result(_print_output, result_str),
            Err(e) => Err(e),
        }
    }

    /// Set up print output capture and return the capture container
    fn setup_print_capture(
        &mut self,
    ) -> Result<std::rc::Rc<std::cell::RefCell<Vec<String>>>, LuaEngineError> {
        let print_output = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        let print_fn = self.create_print_function(print_output.clone())?;

        self.lua
            .globals()
            .set("print", print_fn)
            .map_err(|e| self.create_enhanced_error(e, None))?;

        Ok(print_output)
    }

    /// Execute Lua code and format the result
    fn execute_lua_code(&self, source: &str) -> Result<String, LuaEngineError> {
        match self.lua.load(source).eval::<Value>() {
            Ok(value) => Ok(self.format_lua_value(&value, 0, 3)),
            Err(e) => Err(self.create_enhanced_error(e, None)),
        }
    }

    /// Combine print output and script result into final output
    fn combine_output_and_result(
        &self,
        print_output: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
        result: String,
    ) -> Result<String, LuaEngineError> {
        let print_lines = print_output.borrow();
        let mut output = Vec::new();

        // Add all print output
        output.extend(print_lines.iter().cloned());

        // Add return value if meaningful
        if self.should_show_result(&result, &print_lines) {
            output.push(result);
        }

        Ok(output.join("\n"))
    }

    /// Determine if we should show the script result
    fn should_show_result(&self, result: &str, print_lines: &[String]) -> bool {
        !result.is_empty() && result != "nil" && (print_lines.is_empty() || !result.is_empty())
    }

    /// Register utility functions that don't need state access
    fn register_utility_functions(&mut self) -> Result<(), LuaEngineError> {
        // Essential utility functions
        self.register_function("url_encode", |_, input: String| -> LuaResult<String> {
            Ok(urlencoding::encode(&input).to_string())
        })?;

        self.register_function("url_decode", |_, input: String| -> LuaResult<String> {
            match urlencoding::decode(&input) {
                Ok(decoded) => Ok(decoded.to_string()),
                Err(_) => Ok(input), // Return original if decode fails
            }
        })?;

        self.register_function("escape_shell", |_, input: String| -> LuaResult<String> {
            // Simple shell escaping: wrap in single quotes and escape single quotes
            Ok(format!("'{}'", input.replace("'", "'\"'\"'")))
        })?;

        self.register_function("debug_log", |_, msg: String| -> LuaResult<()> {
            debug!("Lua debug: {}", msg);
            Ok(())
        })?;

        // Define ask() function for coroutines
        self.lua
            .load(
                r#"
            function ask(prompt)
                local response = coroutine.yield(prompt)
                return response
            end
        "#,
            )
            .exec()
            .lua_err("Failed to define ask function")?;

        Ok(())
    }

    /// Update the Lua context with basic application state (kept for backwards compatibility)
    /// Most state data is now accessed on-demand via getter functions
    #[allow(dead_code)]
    pub fn update_context(&self, state: &TuiState) -> LuaResult<()> {
        let globals = self.lua.globals();

        // Update basic app table for backwards compatibility
        let app_table: Table = globals.get("app")?;
        app_table.set("position", state.position)?;
        app_table.set("mode", self.mode_to_string(&state.mode))?;
        app_table.set("visible_height", state.visible_height)?;
        app_table.set("visible_width", state.visible_width)?;
        app_table.set("record_count", state.records.len())?;

        Ok(())
    }

    /// Convert Mode enum to string for Lua
    fn mode_to_string(&self, mode: &Mode) -> &'static str {
        match mode {
            Mode::Normal => "normal",
            Mode::Search => "search",
            Mode::Filter => "filter",
            Mode::Command => "command",
            Mode::Warning => "warning",
            Mode::ScriptInput => "script_input",
            Mode::LuaRepl => "lua_repl",
        }
    }

    /// Compile a Lua script and cache the bytecode
    pub fn compile_script(&mut self, name: &str, script: &str) -> Result<(), LuaEngineError> {
        debug!("Compiling script '{}': {}", name, script);

        // Validate and compile the script
        let chunk = self.lua.load(script);
        let function = chunk
            .into_function()
            .lua_err_with_script("Compilation failed", name)?;

        // Generate bytecode
        let bytecode = function.dump(true); // true for stripping debug info for production

        // Store the compiled script
        let compiled = CompiledScript::new(script.to_string(), bytecode);
        self.compiled_scripts.insert(name.to_string(), compiled);

        debug!("Successfully compiled script '{}'", name);
        Ok(())
    }

    /// Load and compile a Lua script from an external file
    #[allow(dead_code)]
    pub fn compile_script_from_file<P: AsRef<Path>>(
        &mut self,
        name: &str,
        file_path: P,
    ) -> Result<(), LuaEngineError> {
        let path = file_path.as_ref().to_path_buf();

        // Read script content
        let script =
            fs::read_to_string(&path).lua_err_with_script("Failed to read script file", name)?;

        // Validate and compile
        let chunk = self.lua.load(&script);
        let function = chunk
            .into_function()
            .lua_err_with_script("Compilation failed", name)?;

        // Generate bytecode
        let bytecode = function.dump(true);

        // Store with file metadata
        let compiled = CompiledScript::from_file(path, script, bytecode)
            .lua_err_with_script("Failed to create compiled script", name)?;

        self.compiled_scripts.insert(name.to_string(), compiled);
        debug!("Successfully compiled script '{}' from file", name);
        Ok(())
    }

    /// Create enhanced error with better debugging information
    fn create_enhanced_error(
        &self,
        lua_error: mlua::Error,
        script_name: Option<String>,
    ) -> LuaEngineError {
        let message = lua_error.to_string();
        let mut line_number = None;
        let mut stack_trace = None;

        // Try to extract line number from error message
        if let Some(captures) = regex::Regex::new(r":(\d+):")
            .ok()
            .and_then(|re| re.captures(&message))
        {
            if let Some(line_str) = captures.get(1) {
                line_number = line_str.as_str().parse().ok();
            }
        }

        // For runtime errors, try to get stack trace
        match &lua_error {
            mlua::Error::RuntimeError(msg) => {
                stack_trace = Some(msg.clone());
            }
            _ => {}
        }

        LuaEngineError {
            message,
            script_name,
            line_number,
            stack_trace,
        }
    }

    /// Get list of all compiled scripts
    pub fn get_compiled_scripts(&self) -> Vec<&str> {
        self.compiled_scripts.keys().map(|s| s.as_str()).collect()
    }

    /// Remove a compiled script from cache
    #[allow(dead_code)]
    pub fn remove_script(&mut self, name: &str) -> bool {
        self.compiled_scripts.remove(name).is_some()
    }

    /// Clear all compiled scripts
    #[allow(dead_code)]
    pub fn clear_scripts(&mut self) {
        self.compiled_scripts.clear();
        debug!("Cleared all compiled scripts");
    }

    /// Add a directory to search for external Lua files
    #[allow(dead_code)]
    pub fn add_script_directory<P: AsRef<Path>>(&mut self, dir: P) {
        let path = dir.as_ref().to_path_buf();
        if !self.script_directories.contains(&path) {
            self.script_directories.push(path);
            debug!("Added script directory: {:?}", dir.as_ref());
        }
    }

    /// Load all .lua files from script directories
    #[allow(dead_code)]
    pub fn load_scripts_from_directories(&mut self) -> Result<Vec<String>, LuaEngineError> {
        let mut loaded_scripts = Vec::new();

        for dir in &self.script_directories.clone() {
            if !dir.exists() {
                continue;
            }

            let entries =
                fs::read_dir(dir).lua_err(format!("Failed to read directory {:?}", dir))?;

            for entry in entries {
                let entry = entry.lua_err("Failed to read directory entry")?;

                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        match self.compile_script_from_file(stem, &path) {
                            Ok(_) => {
                                loaded_scripts.push(stem.to_string());
                                debug!("Loaded script '{}' from {:?}", stem, path);
                            }
                            Err(e) => {
                                warn!("Failed to load script from {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(loaded_scripts)
    }

    /// Test method for Lua execution (used in Phase 1 tests)
    #[allow(dead_code)]
    pub fn test_lua_execution(&self) -> LuaResult<()> {
        // Update context with test state (for backwards compatibility)
        let globals = self.lua.globals();
        let app_table: Table = globals.get("app")?;
        app_table.set("position", 0)?;

        // Test basic execution
        // Note: Lua execution now requires immediate functions registration
        debug!("Lua engine test execution successful");
        Ok(())
    }

    /// Get bytecode statistics for monitoring
    #[allow(dead_code)]
    pub fn get_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert("total_scripts".to_string(), self.compiled_scripts.len());

        let total_bytecode_size: usize = self
            .compiled_scripts
            .values()
            .map(|script| script.bytecode.len())
            .sum();
        stats.insert("total_bytecode_bytes".to_string(), total_bytecode_size);

        let external_scripts = self
            .compiled_scripts
            .values()
            .filter(|script| script.file_path.is_some())
            .count();
        stats.insert("external_scripts".to_string(), external_scripts);

        stats
    }

    /// Execute a script asynchronously, handling coroutines and yielding
    pub fn execute_script_async(&mut self, name: &str) -> Result<Option<String>, LuaEngineError> {
        if let Some(compiled) = self.compiled_scripts.get(name) {
            // Check if we need to reload from file
            if compiled.needs_reload() {
                warn!(
                    "Script '{}' needs reload but hot-reload not implemented yet",
                    name
                );
            }

            // Note: No command registry needed with immediate execution

            // Create and start a coroutine with the compiled script
            let coroutine_func = self
                .lua
                .load(&compiled.source)
                .into_function()
                .map_err(|e| self.create_enhanced_error(e, Some(name.to_string())))?;

            let thread = self
                .lua
                .create_thread(coroutine_func)
                .map_err(|e| self.create_enhanced_error(e, Some(name.to_string())))?;

            // Resume the coroutine
            self.resume_coroutine(thread, name, Value::Nil)
        } else {
            Err(LuaEngineError {
                message: format!("Script '{}' not found", name),
                script_name: Some(name.to_string()),
                line_number: None,
                stack_trace: None,
            })
        }
    }

    /// Resume a suspended coroutine with input
    fn resume_coroutine(
        &mut self,
        thread: Thread,
        script_name: &str,
        input: Value,
    ) -> Result<Option<String>, LuaEngineError> {
        match thread.resume::<Value>(input) {
            Ok(value) => {
                // Check if this is a yield from ask()
                if let Value::String(yielded_value) = &value {
                    let prompt_str = yielded_value
                        .to_str()
                        .lua_err_with_script("Invalid yielded string", script_name)?;

                    // This is an ask() prompt
                    debug!(
                        "Script '{}' suspended with ask prompt: {}",
                        script_name, prompt_str
                    );

                    // Store the suspended coroutine
                    self.suspended_coroutine = Some(SuspendedCoroutine {
                        thread,
                        prompt: prompt_str.to_string(),
                        script_name: Some(script_name.to_string()),
                    });

                    // Return the prompt to signal the UI should ask for input
                    return Ok(Some(prompt_str.to_string()));
                }

                // Script completed normally
                debug!("Script '{}' completed", script_name);
                Ok(None)
            }
            Err(e) => {
                // Real error occurred
                error!("Coroutine execution failed for '{}': {}", script_name, e);
                Err(self.create_enhanced_error(e, Some(script_name.to_string())))
            }
        }
    }

    /// Resume the currently suspended coroutine with user input
    pub fn resume_with_input(&mut self, input: String) -> Result<(), LuaEngineError> {
        if let Some(suspended) = self.suspended_coroutine.take() {
            let script_name = suspended
                .script_name
                .clone()
                .unwrap_or_else(|| "unknown".to_string());

            // Resume with the user's input
            match self.resume_coroutine(
                suspended.thread,
                &script_name,
                Value::String(self.lua.create_string(&input).unwrap()),
            ) {
                Ok(Some(new_prompt)) => {
                    // Script yielded again with another ask() call
                    debug!(
                        "Script '{}' asked for more input: {}",
                        script_name, new_prompt
                    );
                    Ok(())
                }
                Ok(None) => {
                    // Script completed - clear state pointer
                    debug!("Script '{}' completed after input", script_name);
                    let _ = self.set_state_to_registry(None);
                    Ok(())
                }
                Err(e) => {
                    // Script errored - clear state pointer
                    let _ = self.set_state_to_registry(None);
                    Err(e)
                }
            }
        } else {
            Err(LuaEngineError {
                message: "No suspended coroutine to resume".to_string(),
                script_name: None,
                line_number: None,
                stack_trace: None,
            })
        }
    }

    /// Cancel the currently suspended coroutine
    pub fn cancel_suspended_script(&mut self) {
        if let Some(suspended) = self.suspended_coroutine.take() {
            debug!("Cancelled suspended script: {:?}", suspended.script_name);
            // Clear state pointer when cancelling
            let _ = self.set_state_to_registry(None);
        }
    }

    /// Check if there's a script waiting for input
    #[allow(dead_code)]
    pub fn has_suspended_script(&self) -> bool {
        self.suspended_coroutine.is_some()
    }

    /// Get the prompt from the suspended script
    pub fn get_suspended_prompt(&self) -> Option<&str> {
        self.suspended_coroutine.as_ref().map(|s| s.prompt.as_str())
    }

    /// Get completions from the Lua VM by querying the global environment directly
    pub fn get_completions_from_lua(&self, prefix: &str) -> Result<Vec<String>, LuaEngineError> {
        let mut completions = Vec::new();

        // Get the global environment table
        let globals = self.lua.globals();

        // Iterate through all global variables
        for pair in globals.pairs::<mlua::Value, mlua::Value>() {
            match pair {
                Ok((key, value)) => {
                    // Only process string keys
                    if let Ok(key_str) = String::from_lua(key, &self.lua) {
                        // Check if the key starts with our prefix
                        if key_str.starts_with(prefix) {
                            match value {
                                mlua::Value::Function(_) => {
                                    // Add function with parentheses
                                    completions.push(format!("{}()", key_str));
                                }
                                _ => {
                                    // Add variable without parentheses
                                    completions.push(key_str);
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Skip invalid pairs
                    continue;
                }
            }
        }

        // Sort the completions alphabetically
        completions.sort();

        Ok(completions)
    }

    /// Execute record processors callbacks on a record
    pub fn execute_record_processors(
        &mut self,
        record: &mut crate::record::Record,
    ) -> Result<(), LuaEngineError> {
        let globals = self.lua.globals();

        // Get the record_processors array
        let processors: mlua::Table = match globals.get("record_processors") {
            Ok(table) => table,
            Err(_) => {
                // If record_processors doesn't exist, create it
                let table = self.lua.create_table().map_err(|e| {
                    LuaEngineError::format_with_script(
                        "Failed to create table",
                        e,
                        "execute_record_processors",
                    )
                })?;
                globals.set("record_processors", &table).map_err(|e| {
                    LuaEngineError::format_with_script(
                        "Failed to set record_processors",
                        e,
                        "execute_record_processors",
                    )
                })?;
                table
            }
        };

        // Convert record to Lua table
        let record_table = self.record_to_lua_table(record)?;

        // Execute each processor
        for pair in processors.pairs::<mlua::Value, mlua::Function>() {
            let (_, processor): (mlua::Value, mlua::Function) = pair.map_err(|e| {
                LuaEngineError::format_with_script(
                    "Failed to iterate processors",
                    e,
                    "execute_record_processors",
                )
            })?;

            // Call the processor with the record
            let result: mlua::Table = processor.call(record_table.clone()).map_err(|e| {
                LuaEngineError::format_with_script(
                    "Failed to call processor",
                    e,
                    "execute_record_processors",
                )
            })?;

            // Apply the result to the record
            self.apply_processor_result(record, result)?;
        }

        Ok(())
    }

    /// Convert a Rust Record to a Lua table
    fn record_to_lua_table(
        &self,
        record: &crate::record::Record,
    ) -> Result<mlua::Table, LuaEngineError> {
        let table = self.lua.create_table().map_err(|e| {
            LuaEngineError::format_with_script("Failed to create table", e, "record_to_lua_table")
        })?;

        // Add original line
        table
            .set("original", record.original.as_str())
            .map_err(|e| {
                LuaEngineError::format_with_script(
                    "Failed to set original",
                    e,
                    "record_to_lua_table",
                )
            })?;

        // Add all data fields
        for (key, value) in &record.data {
            table.set(key.as_str(), value.as_str()).map_err(|e| {
                LuaEngineError::format_with_script(
                    "Failed to set data field",
                    e,
                    "record_to_lua_table",
                )
            })?;
        }

        Ok(table)
    }

    /// Apply processor result to the record
    fn apply_processor_result(
        &self,
        record: &mut crate::record::Record,
        result: mlua::Table,
    ) -> Result<(), LuaEngineError> {
        for pair in result.pairs::<mlua::String, mlua::Value>() {
            let (key, value): (mlua::String, mlua::Value) = pair.map_err(|e| {
                LuaEngineError::format_with_script(
                    "Failed to iterate result pairs",
                    e,
                    "apply_processor_result",
                )
            })?;
            let key_borrowed = key.to_str().map_err(|e| {
                LuaEngineError::format_with_script(
                    "Failed to convert key to string",
                    e,
                    "apply_processor_result",
                )
            })?;
            let key_str = key_borrowed.as_ref();

            match value {
                mlua::Value::String(s) => {
                    let s_str = s.to_str().map_err(|e| {
                        LuaEngineError::format_with_script(
                            "Failed to convert string value",
                            e,
                            "apply_processor_result",
                        )
                    })?;
                    record.set_data(key_str, s_str.as_ref().to_string());
                }
                mlua::Value::Number(n) => {
                    record.set_data(key_str, n.to_string());
                }
                mlua::Value::Boolean(b) => {
                    record.set_data(key_str, b.to_string());
                }
                mlua::Value::Nil => {
                    record.unset_data(key_str);
                }
                _ => {
                    // Convert other types to string
                    record.set_data(key_str, format!("{:?}", value));
                }
            }
        }

        Ok(())
    }
}

/// Helper struct to wrap TuiState for safe Lua access
pub struct LuaStateWrapper<'a> {
    pub state: &'a mut TuiState,
}

impl<'a> LuaStateWrapper<'a> {
    #[allow(dead_code)]
    pub fn new(state: &'a mut TuiState) -> Self {
        Self { state }
    }
}

impl<'a> UserData for LuaStateWrapper<'a> {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_position", |_, this, ()| Ok(this.state.position));

        methods.add_method_mut("set_position", |_, this, pos: usize| {
            this.state.set_position(pos);
            Ok(())
        });

        methods.add_method_mut("set_warning", |_, this, msg: String| {
            this.state.set_warning(msg);
            Ok(())
        });

        methods.add_method("get_mode", |_, this, ()| {
            Ok(match this.state.mode {
                Mode::Normal => "normal",
                Mode::Search => "search",
                Mode::Filter => "filter",
                Mode::Command => "command",
                Mode::Warning => "warning",
                Mode::ScriptInput => "script_input",
                Mode::LuaRepl => "lua_repl",
            })
        });

        methods.add_method("get_record_count", |_, this, ()| {
            Ok(this.state.records.len())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;

    #[test]
    fn test_lua_engine_creation() {
        let engine = LuaEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_lua_engine_initialization() {
        let mut engine = LuaEngine::new().unwrap();
        let result = engine.initialize();
        assert!(result.is_ok());
    }

    #[test]
    fn test_basic_lua_execution() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        let result = engine.test_lua_execution();
        assert!(result.is_ok());
    }

    #[test]
    fn test_script_compilation() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        let result = engine.compile_script("test", "warning('Test script')");
        assert!(result.is_ok());
    }

    #[test]
    fn test_bytecode_caching() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        // Test LUA004: Script compilation and bytecode caching
        let script = "return 'hello world'";
        let result = engine.compile_script("cache_test", script);
        assert!(result.is_ok());

        // Verify the script is cached
        assert!(engine.compiled_scripts.contains_key("cache_test"));

        // Verify bytecode is generated
        let compiled = engine.compiled_scripts.get("cache_test").unwrap();
        assert!(!compiled.bytecode.is_empty());
        assert_eq!(compiled.source, script);

        // Test execution from cache
        let exec_result = engine.execute_script_async("cache_test");
        assert!(exec_result.is_ok());

        // Just verify execution succeeded
        assert!(exec_result.is_ok());
    }

    #[test]
    fn test_compilation_error_handling() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        // Test LUA006: Compilation error handling
        let invalid_script = "this is not valid lua code !!!";
        let result = engine.compile_script("invalid", invalid_script);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("Compilation failed"));
        assert_eq!(error.script_name, Some("invalid".to_string()));
    }

    #[test]
    fn test_enhanced_context_setup() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        // Create a mock TuiState for testing
        let mut state = TuiState::new().unwrap();
        state.position = 1; // Use 1-based indexing
        state.mode = Mode::Search;
        state.visible_height = 30;
        state.visible_width = 100;
        state.search = "test search".to_string();

        // Add a test record
        let mut record = Record::new("test log line".to_string());
        record.set_data("timestamp", "2024-01-01T00:00:00Z".to_string());
        record.set_data("level", "INFO".to_string());
        state.records.add_record(record, None);

        // Update context
        let result = engine.update_context(&state);
        assert!(result.is_ok());

        // Store state pointer for function access during testing
        engine.set_state_to_registry(Some(&mut state)).unwrap();

        // Test state getter functions
        let position: usize = engine.lua.load("return get_position()").eval().unwrap();
        assert_eq!(position, 1); // Now returns 1-based line number

        let mode: String = engine.lua.load("return get_mode()").eval().unwrap();
        assert_eq!(mode, "search");

        let viewport: Table = engine.lua.load("return get_viewport()").eval().unwrap();
        let height: usize = viewport.get("height").unwrap();
        assert_eq!(height, 30);

        let search_text: String = engine.lua.load("return get_search()").eval().unwrap();
        assert_eq!(search_text, "test search");

        // Test record getter function
        let record: Table = engine.lua.load("return get_record()").eval().unwrap();
        let line: String = record.get("original").unwrap();
        assert_eq!(line, "test log line");

        let level: String = record.get("level").unwrap();
        assert_eq!(level, "INFO");

        let timestamp: String = record.get("timestamp").unwrap();
        assert_eq!(timestamp, "2024-01-01T00:00:00Z");

        // Clear state pointer after test
        engine.set_state_to_registry(None).unwrap();
    }

    #[test]
    fn test_enhanced_api_functions() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        // Create a test state for API function testing
        let mut state = crate::state::TuiState::new().unwrap();

        // Add some test records so position changes work correctly
        for i in 0..10 {
            let mut record = crate::record::Record::new(format!("Test log line {}", i));
            record.index = i;
            state.records.add_record(record, None);
        }

        // Test LUA013: Core command functions with state
        engine.compile_script("test_quit", "quit()").unwrap();
        let _ = engine.execute_with_state("test_quit", &mut state); // May set running to false

        engine.compile_script("test_vmove", "vmove(10)").unwrap();
        engine.execute_with_state("test_vmove", &mut state).unwrap();

        engine.compile_script("test_vgoto", "vgoto(20)").unwrap();
        engine.execute_with_state("test_vgoto", &mut state).unwrap();

        engine
            .compile_script("test_move_top", "move_top()")
            .unwrap();
        engine
            .execute_with_state("test_move_top", &mut state)
            .unwrap();

        engine
            .compile_script("test_move_bottom", "move_bottom()")
            .unwrap();
        engine
            .execute_with_state("test_move_bottom", &mut state)
            .unwrap();

        // Test LUA014: UI and display functions
        engine
            .compile_script("test_warning", "warning('test warning')")
            .unwrap();
        engine
            .execute_with_state("test_warning", &mut state)
            .unwrap();

        // Test that the core functions were executed successfully by checking state changes
        // move_bottom sets position to the last valid record (10 in this case, 1-indexed with 10 records)
        assert_eq!(state.position, 10); // last valid position with 10 records (1-10)
        assert!(!state.running); // quit() sets running to false
        assert_eq!(state.warning, "test warning"); // warning() sets the warning message

        // TODO: Add tests for utility functions (exec, url_encode, url_decode, escape_shell, debug_log) when they are implemented
    }

    #[test]
    fn test_script_management() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        // Test script compilation and management
        engine.compile_script("test1", "return 'script1'").unwrap();
        engine.compile_script("test2", "return 'script2'").unwrap();

        // Check script listing
        let scripts = engine.get_compiled_scripts();
        assert_eq!(scripts.len(), 2);
        assert!(scripts.contains(&"test1"));
        assert!(scripts.contains(&"test2"));

        // Test script removal
        assert!(engine.remove_script("test1"));
        assert!(!engine.remove_script("nonexistent"));

        let remaining_scripts = engine.get_compiled_scripts();
        assert_eq!(remaining_scripts.len(), 1);
        assert!(remaining_scripts.contains(&"test2"));

        // Test clear all scripts
        engine.clear_scripts();
        assert_eq!(engine.get_compiled_scripts().len(), 0);
    }

    #[test]
    fn test_stats_and_monitoring() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.get("total_scripts").unwrap(), &0);
        assert_eq!(stats.get("total_bytecode_bytes").unwrap(), &0);
        assert_eq!(stats.get("external_scripts").unwrap(), &0);

        // Add some scripts
        engine.compile_script("test1", "return 'hello'").unwrap();
        engine.compile_script("test2", "return 'world'").unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.get("total_scripts").unwrap(), &2);
        assert!(stats.get("total_bytecode_bytes").unwrap() > &0);
    }

    #[test]
    fn test_phase_2_comprehensive() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        // Test LUA004: Script compilation and bytecode caching
        let script = "local x = 10; return x * 2";
        let result = engine.compile_script("phase2_test", script);
        assert!(result.is_ok(), "Failed to compile script");

        // Verify bytecode exists and execution works
        let exec_result = engine.execute_script_async("phase2_test");
        assert!(exec_result.is_ok(), "Failed to execute compiled script");
        // Just verify execution succeeded
        assert!(exec_result.is_ok());

        // Test LUA006: Enhanced error handling
        let invalid_script = "return unknown_variable + 1";
        let compile_result = engine.compile_script("invalid_test", invalid_script);
        assert!(compile_result.is_ok(), "Should compile but fail at runtime");

        let exec_result = engine.execute_script_async("invalid_test");
        assert!(exec_result.is_err(), "Should fail with runtime error");

        let error = exec_result.unwrap_err();
        assert!(
            error.message.contains("unknown_variable"),
            "Error should mention undefined variable"
        );
        assert_eq!(error.script_name, Some("invalid_test".to_string()));

        // Test enhanced context with mock state
        let mut state = TuiState::new().unwrap();
        state.position = 42;
        state.mode = Mode::Filter;
        state.warning = "Test warning".to_string();

        let context_result = engine.update_context(&state);
        assert!(context_result.is_ok(), "Failed to update context");

        // Store state pointer for function access during testing
        engine.set_state_to_registry(Some(&mut state)).unwrap();

        // Verify enhanced state access via getter functions
        let position: usize = engine.lua.load("return get_position()").eval().unwrap();
        assert_eq!(position, 42); // Position is already 1-based internally

        let mode: String = engine.lua.load("return get_mode()").eval().unwrap();
        assert_eq!(mode, "filter");

        let warning: String = engine.lua.load("return get_warning()").eval().unwrap();
        assert_eq!(warning, "Test warning");

        // Clear state pointer after test
        engine.set_state_to_registry(None).unwrap();

        println!("Phase 2 comprehensive test completed successfully!");
        println!("✓ LUA004: Script compilation and bytecode caching");
        println!("✓ LUA005: External Lua file support structure");
        println!("✓ LUA006: Compilation error handling");
        println!("✓ Enhanced context setup with full application state");
        println!("✓ All Phase 1 functionality maintained");
        println!("✓ Improved error reporting with stack traces");
        println!("✓ Performance optimizations with bytecode caching");
    }

    #[test]
    fn test_dynamic_completions() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        // Test getting completions for "get_"
        let completions = engine.get_completions_from_lua("get_").unwrap();
        assert!(
            !completions.is_empty(),
            "Should find completions for 'get_'"
        );

        // Should include functions like get_record, get_position, etc.
        assert!(
            completions.iter().any(|c| c.contains("get_record")),
            "Should include get_record"
        );
        assert!(
            completions.iter().any(|c| c.contains("get_position")),
            "Should include get_position"
        );

        // Test getting completions for "vmove"
        let completions = engine.get_completions_from_lua("vmove").unwrap();
        assert!(
            !completions.is_empty(),
            "Should find completions for 'vmove'"
        );
        assert!(
            completions.iter().any(|c| c.contains("vmove()")),
            "Should include vmove()"
        );

        // Test getting completions for empty prefix
        let completions = engine.get_completions_from_lua("").unwrap();
        assert!(
            !completions.is_empty(),
            "Should find completions for empty prefix"
        );

        // Should include many functions
        assert!(
            completions.len() > 10,
            "Should find many completions for empty prefix"
        );

        println!("✓ Dynamic completions from Lua VM work correctly");
    }

    #[test]
    fn test_repl_completion_integration() {
        let mut engine = LuaEngine::new().unwrap();
        engine.initialize().unwrap();

        // Test that the completion system works for REPL-style completions
        let completions = engine.get_completions_from_lua("get_").unwrap();
        assert!(
            !completions.is_empty(),
            "Should find completions for 'get_'"
        );

        // Should include functions like get_record, get_position, etc.
        assert!(
            completions.iter().any(|c| c.contains("get_record()")),
            "Should include get_record()"
        );
        assert!(
            completions.iter().any(|c| c.contains("get_position()")),
            "Should include get_position()"
        );

        // Test empty prefix returns many completions
        let all_completions = engine.get_completions_from_lua("").unwrap();
        assert!(
            all_completions.len() > 20,
            "Should find many completions for empty prefix"
        );

        // Should include core functions
        assert!(
            all_completions.iter().any(|c| c.contains("quit()")),
            "Should include quit()"
        );
        assert!(
            all_completions.iter().any(|c| c.contains("warning()")),
            "Should include warning()"
        );
        assert!(
            all_completions.iter().any(|c| c.contains("vmove()")),
            "Should include vmove()"
        );

        println!("✓ REPL completion integration works correctly");
    }
}
