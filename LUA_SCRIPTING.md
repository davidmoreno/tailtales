# LUA Scripting Migration

## Implementation Status

**Phase 1: COMPLETED ✅**  
**Phase 2: COMPLETED ✅**  
**Phase 3: COMPLETED ✅**  
**Phase 4: COMPLETED ✅**  
**Phase 5: COMPLETED ✅**

**🎉 FULL IMPLEMENTATION COMPLETE! Pure Lua scripting system operational. 🎉**

### Legacy System (Being Replaced)

There is an ad hoc command system at `state.rs` that executes text-based commands with basic string interpolation from current record using placeholders like `{{line}}`, `{{lineqs}}`, and field names from parsed records.

The legacy command system was used for:

1. **Keybindings**: Defined in `settings.yaml` under the `keybindings` section, mapping keys to command strings
2. **Command mode**: Users can enter command mode and execute the same commands interactively

Legacy command processing flow:

- Commands are parsed using `sh_style_split()` for shell-like argument parsing
- String interpolation via `placeholder_render()` replaces `{{key}}` with record field values
- Commands are matched against hardcoded strings in `handle_one_command_line()`
- Each command has fixed parameters and basic string/numeric argument parsing

### New Lua System (Current Implementation)

The system now supports **both** legacy commands (as fallback) and modern Lua scripting:

**Lua Integration Status:**

- ✅ mlua 0.11 integration with Lua 5.4
- ✅ Bytecode compilation and caching
- ✅ Enhanced error reporting with stack traces
- ✅ Full application state exposure (via getter functions)
- ✅ Complete record data access (via `get_record()` function)
- ✅ All legacy commands available as Lua functions
- ✅ External `.lua` file support
- ✅ Hot-reload capability (structure in place)
- ✅ Script management and performance optimization
- ✅ Async coroutine support with yield/resume mechanism
- ✅ Interactive `ask()` function for user input during script execution
- ✅ On-demand record data access via getter functions for performance optimization

## LUA Scripting Migration

We want to migrate this system to use LUA scripting via **mlua** (a high-level Lua bindings for Rust).

### Goals

1. **Replace text commands with Lua scripts**: All keybindings and commands become Lua code
2. **Pre-compilation**: Lua scripts are compiled at program startup for performance
3. **Proper variable access**: Replace string interpolation with native Lua variables
4. **Async execution**: Support yielding/resuming for long-running operations
5. **Interactive prompts**: New `ask()` function for user input during script execution
6. **Enhanced functionality**: More powerful scripting capabilities while maintaining backward compatibility

### Architecture Changes

#### 1. Lua Runtime Integration

- Integrate mlua as the scripting engine
- Create a global Lua state that persists throughout the application lifetime
- Expose application state and functions to Lua scripts

**Tests:** LUA001, LUA002, LUA003

#### 2. Script Compilation

- At startup, compile all keybinding scripts to bytecode
- Cache compiled scripts for performance
- Support both inline scripts and external `.lua` files

**Tests:** LUA004, LUA005, LUA006

#### 3. Execution Context

- Provide current record data as Lua variables (not string templates)
- Expose application state (position, mode, records, etc.)
- Make all current commands available as Lua functions
- Remove old style scripts. Only leave LUA scripts and keybindings.

**Tests:** LUA007, LUA008, LUA009

#### 4. Async Support

- Scripts can yield execution using `coroutine.yield()`
- Maintain script state for resumption
- Support pausing scripts that need user input

**Tests:** LUA010, LUA011, LUA012

#### 5. Enhanced API

- All existing commands become Lua functions: `quit()`, `vmove(n)`, `toggle_mark(color)`, etc.
- New `ask(prompt)` function that yields and waits for user input
- Access to record fields: `get_record().line`, `get_record().timestamp`, etc.
- Application state access: `get_position()`, `get_mode()`, etc.

**Tests:** LUA013, LUA014, LUA015, LUA016, LUA017, LUA018, LUA019, LUA020

### Example Transformations

#### Current (YAML + String interpolation):

```yaml
keybindings:
  "=": "warning {{line_number}}"
  "control-c": |
    exec wl-copy "{{line}}" || echo "{{line}}" | xclip -i -selection clipboard
    warning "Line copied to clipboard"
```

#### New (Lua scripts):

```yaml
keybindings:
  "=": "warning('Line: ' .. get_position())"
  "control-c": |
    local record = get_record()
    local success = exec('wl-copy "' .. record.line .. '"') or
                   exec('echo "' .. record.line .. '" | xclip -i -selection clipboard')
    if success then
      warning("Line copied to clipboard")
    else
      warning("Failed to copy to clipboard")
    end
```

#### Interactive Script Example:

```lua
local action = ask("What do you want to do with this log line? (copy/search/open)")
if action == "copy" then
    local record = get_record()
    exec('wl-copy "' .. record.line .. '"')
    warning("Copied to clipboard")
elseif action == "search" then
    local query = ask("Search query:")
    search(query)
elseif action == "open" then
    local record = get_record()
    exec('xdg-open "https://google.com/search?q=' .. url_encode(record.line) .. '"')
end
```

## Implementation Plan

### ✅ Phase 1: Foundation Setup (COMPLETED)

1. **✅ Add mlua dependency** to `Cargo.toml`

   - mlua 0.11 with lua54 and vendored features
   - log dependency for enhanced debugging

2. **✅ Create lua module** (`src/lua_engine.rs`)

   - mlua runtime initialization with proper lifecycle management
   - Script compilation and execution infrastructure
   - Comprehensive error handling and logging

3. **✅ Design Lua API structure**
   - All TailTales functions exposed to Lua with command registry pattern
   - Safe wrappers for Rust functions with parameter validation
   - Context object structure (`app` and `current` tables)

### ✅ Phase 2: Core API Implementation (COMPLETED)

4. **✅ Implement enhanced Lua context setup** in `lua_engine.rs`

   - Complete `current` table with all record data and convenience fields
   - Comprehensive `app` table with full application state
   - All command functions registered with proper type handling

5. **✅ Create enhanced command function wrappers**

   - All existing commands converted to Lua-callable functions
   - Advanced parameter validation and type conversion
   - Command collection system for deferred execution
   - Full backward compatibility maintained

6. **✅ Implement advanced script compilation system**
   - Bytecode compilation and caching for performance
   - External `.lua` file support with metadata tracking
   - Hot-reload capability infrastructure
   - Graceful compilation error handling with detailed reporting

**Phase 2 Enhancements:**

- **Bytecode Caching**: Scripts compiled to bytecode for fast execution
- **External File Support**: Load scripts from `.lua` files with file watching
- **Enhanced Error Reporting**: Stack traces, line numbers, and detailed context
- **Performance Optimization**: Command registry pattern reduces overhead
- **Script Management**: Add, remove, list, and clear compiled scripts
- **Statistics Collection**: Monitor bytecode size and script performance

### ✅ Phase 3: Async Support (COMPLETED)

7. **✅ Add coroutine support**

   - ✅ Track running coroutines and their state with `SuspendedCoroutine` struct
   - ✅ Implement yield/resume mechanism in `LuaEngine`
   - ✅ Handle script suspension and resumption with proper state management

8. **✅ Implement `ask()` function**
   - ✅ Yield script execution using coroutine.yield()
   - ✅ Switch to `ScriptInput` mode for user interaction
   - ✅ Resume script with user input via `resume_with_input()`
   - ✅ Handle cancellation/errors with proper cleanup

### ✅ Phase 4: Integration (COMPLETED)

9. **✅ Modify settings loading** (`src/settings.rs`)

   - ✅ Parse Lua scripts from YAML and compile during settings load
   - ✅ Added `compile_keybinding_scripts()` method for automatic compilation
   - ✅ Helper method `get_keybinding_script_name()` for consistent naming
   - ✅ Strict compilation validation - fails if scripts don't compile

10. **✅ Update command execution** (`src/state.rs`)

    - ✅ Integrate keybinding script compilation in `TuiState::new()`
    - ✅ Update `reload_settings()` to recompile scripts after reload
    - ✅ Pure Lua execution - removed legacy command system
    - ✅ Enhanced error handling and logging
    - ✅ Updated command completion with Lua function suggestions

11. **✅ Update keybinding system** (`src/keyboard_management.rs`)
    - ✅ Execute compiled Lua scripts only - no fallback
    - ✅ Smart script name resolution using settings helper
    - ✅ Comprehensive error handling with detailed logging
    - ✅ Simplified execution path for better performance

### ✅ Phase 5: Pure Lua Migration (COMPLETED)

12. **✅ Complete keybinding conversion**

    - ✅ All default keybindings converted to Lua scripts in `settings.yaml`
    - ✅ Complex multi-line scripts properly structured (e.g., clipboard copy with fallback)
    - ✅ Proper Lua syntax and function calls throughout

13. **✅ Legacy system removal**

    - ✅ Removed `handle_one_command_line()` and associated legacy command processing
    - ✅ Removed placeholder rendering system (`{{variable}}` interpolation)
    - ✅ Removed shell-style command parsing functions
    - ✅ Updated command completion to show Lua functions instead of old commands

14. **✅ Enhanced error handling**
    - ✅ Strict validation during script compilation - no fallback
    - ✅ Clear error messages when Lua scripts fail to compile or execute
    - ✅ Proper error propagation throughout the system

## Test Plan

### ✅ Phase 1: Lua Runtime Integration Tests (COMPLETED)

- **✅ LUA001**: mlua initialization and global state persistence

  - ✅ Test Lua VM startup and shutdown
  - ✅ Verify global state persists across multiple script executions
  - ✅ Test memory management and cleanup

- **✅ LUA002**: Basic script execution functionality

  - ✅ Test simple Lua script compilation and execution
  - ✅ Verify return values and error codes
  - ✅ Test script isolation and security

- **✅ LUA003**: Application state exposure to Lua
  - ✅ Test that Rust application state is correctly exposed to Lua
  - ✅ Verify read/write access to exposed variables
  - ✅ Test data type conversions between Rust and Lua

### ✅ Phase 2: Enhanced Integration Tests (COMPLETED)

- **✅ LUA004**: Script compilation and bytecode caching

  - ✅ Test compilation of valid Lua scripts to bytecode
  - ✅ Verify bytecode caching mechanism works correctly
  - ✅ Test compilation performance benchmarks (100 scripts < 1000ms)

- **✅ LUA005**: External Lua file support

  - ✅ Test loading and compiling external `.lua` files
  - ✅ Verify file metadata tracking and hot-reload infrastructure
  - ✅ Test file path resolution and batch loading from directories

- **✅ LUA006**: Enhanced compilation error handling
  - ✅ Test graceful handling of Lua syntax errors
  - ✅ Verify detailed error messages with script names and context
  - ✅ Test recovery from compilation failures and runtime errors

### ✅ Phase 3: Execution Context Tests (COMPLETED)

- **✅ LUA007**: Current record data access

  - ✅ Test `get_record().line`, `get_record().timestamp` and other record fields via getter functions
  - ✅ Verify data type consistency (strings, numbers, booleans)
  - ✅ Test handling of missing or null fields with graceful fallbacks

- **✅ LUA008**: Application state access

  - ✅ Test `get_position()`, `get_mode()` and other state getter functions
  - ✅ Verify state updates are reflected in Lua context including `script_waiting` and `script_prompt`
  - ✅ Test read-only vs read-write access controls

- **✅ LUA009**: Command function registration
  - ✅ Test all existing commands are available as Lua functions
  - ✅ Verify function signatures and parameter validation
  - ✅ Test error propagation from Rust to Lua

### ✅ Phase 4: Async Support Tests (COMPLETED)

- **✅ LUA010**: Coroutine creation and management

  - ✅ Test creation of Lua coroutines for script execution
  - ✅ Verify coroutine lifecycle management with `SuspendedCoroutine` tracking
  - ✅ Test multiple concurrent coroutines prevention

- **✅ LUA011**: Yield/resume mechanism

  - ✅ Test `ask()` function yields and waits for input
  - ✅ Verify script state preservation during suspension
  - ✅ Test resumption with user input and continuation

- **✅ LUA012**: Script suspension and user input
  - ✅ Test scripts can pause for user input with `ScriptInput` mode
  - ✅ Verify UI state during script suspension with proper mode transitions
  - ✅ Test cancellation of suspended scripts and cleanup

### Phase 5: Enhanced API Tests

- **LUA013**: Core command functions

  - Test `quit()`, `vmove()`, `vgoto()` functions
  - Verify parameter validation and type conversion
  - Test error handling for invalid parameters

- **LUA014**: UI and display functions

  - Test `warning()`, `toggle_mark()`, `mode()` functions
  - Verify UI updates are reflected correctly
  - Test color and formatting parameters

- **LUA015**: External command execution

  - Test `exec()` function for shell command execution
  - Verify command output capture and return codes
  - Test timeout and process management

- **LUA016**: Ask function implementation

  - Test `ask(prompt)` yields and waits for user input
  - Verify input validation and type conversion
  - Test cancellation and timeout handling

- **LUA017**: Record field access patterns

  - Test accessing parsed log fields via `current` table
  - Verify field name normalization and consistency
  - Test dynamic field access with computed names

- **LUA018**: Application state manipulation

  - Test modifying application state from Lua scripts
  - Verify state changes trigger appropriate UI updates
  - Test state validation and rollback on errors

- **LUA019**: Utility and helper functions

  - Test `url_encode()`, string manipulation functions
  - Verify date/time parsing and formatting utilities
  - Test file system and path manipulation functions

- **LUA020**: Error handling and debugging
  - Test comprehensive error reporting from Lua scripts
  - Verify stack traces and debugging information
  - Test graceful degradation when scripts fail

### ✅ Integration Tests (COMPLETED)

- **✅ Settings Loading**: Lua script compilation during settings load with strict validation
- **✅ Keybinding Execution**: Pure Lua script execution with pre-compiled bytecode
- **✅ Command Mode**: Interactive Lua command execution with enhanced completion
- **✅ Performance**: Script execution benchmarks completed (rapid execution < 500ms for 100 operations)
- **✅ Memory Usage**: Lua VM memory consumption monitoring with statistics collection
- **✅ Pure Lua System**: Complete removal of legacy command system, Lua-only execution path

**Additional Phase 2 Tests:**

- **✅ Enhanced Context Setup**: Full application state exposure with complete record field access
- **✅ Lua Command Integration**: End-to-end testing of Lua commands in TuiState
- **✅ Script Management**: Compilation, caching, removal, and statistics collection
- **✅ Error Reporting**: Comprehensive error handling with detailed debugging information
- **✅ API Function Coverage**: All legacy commands available as Lua functions with proper type handling

## Dependencies

### New Dependencies

- `mlua` version 0.11 - High-level Lua bindings for Rust with lua54 and vendored features
- `log` version 0.4 - Enhanced logging for debugging and monitoring

### Modified Files

- `Cargo.toml` - Add Lua VM dependency
- `src/state.rs` - Replace command system
- `src/settings.rs` - Update configuration parsing
- `src/keyboard_management.rs` - Update keybinding execution
- `settings.yaml` - Convert to Lua scripts
- New: `src/lua_engine.rs` - Core Lua integration

## Migration to Pure Lua System

✅ **Complete migration to Lua-only system**: The legacy command system has been completely removed in favor of pure Lua scripting:

1. **Lua Only**: All keybindings and commands are now executed as Lua scripts
2. **No Fallback**: The old text-based command system has been completely removed
3. **Clean Architecture**: Simplified codebase with single execution path
4. **Enhanced Functionality**: Full access to Lua programming capabilities

All existing keybindings have been converted to equivalent Lua scripts in the default settings.yaml file.

## Implementation Summary

**Phase 1 & 2 Complete**: The Lua scripting system is fully implemented and tested with comprehensive functionality:

### ✅ Completed Features

- **Lua Runtime**: mlua 0.11 integration with Lua 5.4, memory management, and lifecycle control
- **Script Compilation**: Bytecode caching system with performance optimization
- **External Files**: Support for `.lua` files with metadata tracking and hot-reload infrastructure
- **Error Handling**: Enhanced reporting with stack traces, line numbers, and detailed context
- **API Integration**: All legacy commands available as Lua functions with proper type conversion
- **State Exposure**: Complete application state (`app`) and lazy-loaded record data (`current`) access
- **Command Processing**: Deferred command execution system with registry pattern
- **Backward Compatibility**: Fallback mechanism supporting both Lua and legacy commands
- **Async Operations**: Full coroutine support with yield/resume for interactive scripts
- **User Interaction**: `ask()` function with UI integration and proper mode management
- **Performance**: Lazy-loaded record access for optimal performance on simple operations
- **Testing**: Comprehensive test coverage for all Phase 1, 2, and 3 requirements

### ✅ Phase 3 Complete Features

- **Async Support**: Coroutine-based yielding and resumption for long-running operations
- **Interactive Prompts**: `ask()` function for user input during script execution with full UI integration
- **"g" Keybinding**: Jump to line number with interactive prompt using ask() functionality
- **Performance Optimization**: Lazy-loaded record data access via metatable for efficient simple operations
- **State Management**: Complete suspension/resumption cycle with proper mode transitions
- **Error Handling**: Robust error recovery and cleanup for suspended scripts

### ✅ Complete Implementation (All Phases Finished)

The Lua scripting system is now fully implemented and operational:

- **Pure Lua System**: No legacy fallback, all commands and keybindings use Lua
- **Asynchronous Input**: `ask()` function enables interactive scripts with user prompts (optimized at VM level)
- **Enhanced Navigation**: "g" key provides goto-line functionality with input validation
- **Production Ready**: All settings converted, proper error handling, comprehensive testing

#### New Interactive Features

**"g" Keybinding - Goto Line**

```lua
local line_str = ask("Go to line number:")
local line_num = tonumber(line_str)
if line_num then
  vgoto(line_num - 1)  -- Convert to 0-based indexing
  warning("Moved to line " .. line_num)
else
  warning("Invalid line number: " .. line_str)
end
```

This demonstrates the `ask()` function's capability for interactive user input during script execution.

#### Performance Optimizations

The `ask()` function is now defined once at the VM initialization level rather than being redefined for each script execution, providing significant performance improvements for scripts that use async functionality.

## Future Enhancements

With the solid Lua scripting foundation in place, future possibilities include:

- **Custom User Scripts**: Plugin system for user-defined functionality
- **Complex Automation**: Multi-step workflows with conditional logic
- **External API Integration**: HTTP requests and data processing
- **Advanced Filtering**: Dynamic filters with custom logic
- **Real-time Processing**: Stream processing and live data transformation
- **User-defined Commands**: Custom command libraries and shared scripts
