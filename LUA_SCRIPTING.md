# LUA Scripting Migration

## Current State

There is an ad hoc command system at `state.rs` that just executes commands and is text based.
It allows very basic string interpolation from the current record using placeholders like `{{line}}`, `{{lineqs}}`, and field names from parsed records.

The command system is used for:
1. **Keybindings**: Defined in `settings.yaml` under the `keybindings` section, mapping keys to command strings
2. **Command mode**: Users can enter command mode and execute the same commands interactively

Current command processing flow:
- Commands are parsed using `sh_style_split()` for shell-like argument parsing
- String interpolation via `placeholder_render()` replaces `{{key}}` with record field values
- Commands are matched against hardcoded strings in `handle_one_command_line()`
- Each command has fixed parameters and basic string/numeric argument parsing

### Current Commands
The system supports commands like: `quit`, `clear`, `vmove <n>`, `vgoto <n>`, `toggle_mark <color>`, `exec <shell_command>`, `warning <message>`, `mode <mode>`, etc.

### Current Interpolation
- `{{line}}` - Original log line
- `{{lineqs}}` - URL-encoded sanitized version of the line
- `{{field_name}}` - Any field extracted by parsers
- `{{line_number}}` - Current line number

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

**Tests:** LUA007, LUA008, LUA009

#### 4. Async Support
- Scripts can yield execution using `coroutine.yield()`
- Maintain script state for resumption
- Support pausing scripts that need user input

**Tests:** LUA010, LUA011, LUA012

#### 5. Enhanced API
- All existing commands become Lua functions: `quit()`, `vmove(n)`, `toggle_mark(color)`, etc.
- New `ask(prompt)` function that yields and waits for user input
- Access to record fields: `current.line`, `current.timestamp`, etc.
- Application state access: `app.position`, `app.mode`, etc.

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
  "=": "warning('Line: ' .. app.position)"
  "control-c": |
    local success = exec('wl-copy "' .. current.line .. '"') or
                   exec('echo "' .. current.line .. '" | xclip -i -selection clipboard')
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
    exec('wl-copy "' .. current.line .. '"')
    warning("Copied to clipboard")
elseif action == "search" then
    local query = ask("Search query:")
    search(query)
elseif action == "open" then
    exec('xdg-open "https://google.com/search?q=' .. url_encode(current.line) .. '"')
end
```

## Implementation Plan

### Phase 1: Foundation Setup
1. **Add mlua dependency** to `Cargo.toml`
2. **Create lua module** (`src/lua_engine.rs`)
   - Initialize mlua runtime
   - Create script compilation and execution functions
   - Basic error handling and logging

3. **Design Lua API structure**
   - Define which functions/variables to expose to Lua
   - Create safe wrappers for Rust functions
   - Design the context object structure

### Phase 2: Core API Implementation
4. **Implement Lua context setup** in `lua_engine.rs`
   - Expose `current` table with record data
   - Expose `app` table with application state
   - Register all command functions

5. **Create command function wrappers**
   - Convert existing commands to Lua-callable functions
   - Add parameter validation and type conversion
   - Maintain backward compatibility

6. **Implement script compilation system**
   - Compile scripts at startup
   - Cache compiled bytecode
   - Handle compilation errors gracefully

### Phase 3: Async Support
7. **Add coroutine support**
   - Track running coroutines and their state
   - Implement yield/resume mechanism
   - Handle script suspension and resumption

8. **Implement `ask()` function**
   - Yield script execution
   - Switch to input mode
   - Resume script with user input
   - Handle cancellation/errors

### Phase 4: Integration
9. **Modify settings loading** (`src/settings.rs`)
   - Parse Lua scripts from YAML
   - Compile scripts during settings load
   - Update settings validation

10. **Update command execution** (`src/state.rs`)
    - Replace `handle_one_command_line()` with Lua script execution
    - Remove hardcoded command matching
    - Update error handling for Lua errors

11. **Update keybinding system** (`src/keyboard_management.rs`)
    - Execute compiled Lua scripts instead of command strings
    - Handle script suspension/resumption
    - Maintain mode switching logic

### Phase 5: Testing & Migration
12. **Create migration utilities**
    - Helper functions to convert old command syntax
    - Validation tools for new Lua scripts
    - Backward compatibility layer if needed

13. **Update default configuration**
    - Convert all default keybindings to Lua
    - Provide example Lua scripts
    - Update documentation

14. **Add comprehensive testing**
    - Unit tests for Lua API functions
    - Integration tests for script execution
    - Performance benchmarks

## Test Plan

### Phase 1: Lua Runtime Integration Tests
- **LUA001**: mlua initialization and global state persistence
  - Test Lua VM startup and shutdown
  - Verify global state persists across multiple script executions
  - Test memory management and cleanup

- **LUA002**: Basic script execution functionality
  - Test simple Lua script compilation and execution
  - Verify return values and error codes
  - Test script isolation and security

- **LUA003**: Application state exposure to Lua
  - Test that Rust application state is correctly exposed to Lua
  - Verify read/write access to exposed variables
  - Test data type conversions between Rust and Lua

### Phase 2: Script Compilation Tests
- **LUA004**: Script compilation and bytecode caching
  - Test compilation of valid Lua scripts to bytecode
  - Verify bytecode caching mechanism works correctly
  - Test compilation performance benchmarks

- **LUA005**: External Lua file support
  - Test loading and compiling external `.lua` files
  - Verify file watching and hot-reload functionality
  - Test file path resolution and error handling

- **LUA006**: Compilation error handling
  - Test graceful handling of Lua syntax errors
  - Verify helpful error messages with line numbers
  - Test recovery from compilation failures

### Phase 3: Execution Context Tests
- **LUA007**: Current record data access
  - Test `current.line`, `current.timestamp` and other record fields
  - Verify data type consistency (strings, numbers, booleans)
  - Test handling of missing or null fields

- **LUA008**: Application state access
  - Test `app.position`, `app.mode` and other state variables
  - Verify state updates are reflected in Lua context
  - Test read-only vs read-write access controls

- **LUA009**: Command function registration
  - Test all existing commands are available as Lua functions
  - Verify function signatures and parameter validation
  - Test error propagation from Rust to Lua

### Phase 4: Async Support Tests
- **LUA010**: Coroutine creation and management
  - Test creation of Lua coroutines for script execution
  - Verify coroutine lifecycle management
  - Test multiple concurrent coroutines

- **LUA011**: Yield/resume mechanism
  - Test `coroutine.yield()` functionality
  - Verify script state preservation during suspension
  - Test resumption with different parameters

- **LUA012**: Script suspension and user input
  - Test scripts can pause for user input
  - Verify UI state during script suspension
  - Test cancellation of suspended scripts

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

### Integration Tests
- **Settings Loading**: YAML parsing with Lua scripts, compilation during startup
- **Keybinding Execution**: All keybindings work with Lua equivalents
- **Command Mode**: Interactive Lua command execution
- **Performance**: Script execution benchmarks vs old system
- **Memory Usage**: Lua VM memory consumption monitoring

## Dependencies

### New Dependencies
- `mlua` version 1.79 - High-level Lua bindings for Rust

### Modified Files
- `Cargo.toml` - Add Lua VM dependency
- `src/state.rs` - Replace command system
- `src/settings.rs` - Update configuration parsing
- `src/keyboard_management.rs` - Update keybinding execution
- `settings.yaml` - Convert to Lua scripts
- New: `src/lua_engine.rs` - Core Lua integration

## Backward Compatibility

There is no need for backward compatibility with the old command syntax.

## Future Enhancements

With Lua scripting in place, future possibilities include:
- Custom user scripts and plugins
- More complex automation and workflows
- Integration with external APIs
- Advanced filtering and processing logic
- User-defined commands and functions
