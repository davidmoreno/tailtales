//! TailTales - Flexible log viewer for logfmt and other formats
//!
//! This library provides the core functionality for TailTales, including
//! log parsing, filtering, searching, and Lua scripting integration.

pub mod application;
pub mod ast;
pub mod events;
pub mod keyboard_input;
pub mod keyboard_management;
pub mod lua_engine;
pub mod parser;
pub mod record;
pub mod recordlist;
pub mod regex_cache;
pub mod settings;
pub mod state;
pub mod tuichrome;
pub mod utils;

// Re-export commonly used types for convenience
pub use lua_engine::{LuaEngine, LuaEngineError};
pub use record::Record;
pub use state::{Mode, TuiState};
