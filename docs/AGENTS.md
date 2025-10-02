# AI Agent Development Guide for TailTales

This document provides essential information for AI agents working on the TailTales codebase. It covers project structure, testing requirements, coding standards, and important considerations for maintaining code quality.

## Project Overview

TailTales is a TUI (Terminal User Interface) log viewer written in Rust that supports:

- Real-time log file monitoring and streaming
- Multiple log format parsing (logfmt, regex, patterns, CSV)
- Advanced filtering and search capabilities
- Lua scripting integration for extensibility
- Interactive REPL for debugging and automation

## Critical Testing Requirements

### ⚠️ IMPORTANT: Never Run TailTales Directly

**DO NOT** run `cargo run` or execute the `tt` binary directly in your development environment. TailTales is a TUI application that takes over the terminal, which will:

- Break your AI agent's console interface
- Cause the agent to lose control of the terminal
- Potentially crash or hang the agent session

### Testing Strategy

**Always use unit tests and integration tests instead:**

```bash
# Run all tests
cargo test

# Run specific test modules
cargo test lua_function_tests
cargo test lua_console_initialization_tests
cargo test for_each_record_tests

# Run tests with output
cargo test -- --nocapture

# Run tests in release mode for performance testing
cargo test --release
```

### Test Coverage Requirements

All tests must pass before any changes are committed. The test suite includes:

1. **Lua Function Tests** (`tests/lua_function_tests.rs`)

   - Tests all Lua API functions (vmove, vgoto, toggle_mark, etc.)
   - Verifies state management and navigation
   - Tests async functionality (ask() function)
   - Validates error handling

2. **Lua Console Tests** (`tests/lua_console_initialization_tests.rs`)

   - Tests REPL initialization and state management
   - Verifies console output handling
   - Tests history functionality

3. **Record Processing Tests** (`tests/for_each_record_tests.rs`)

   - Tests the record processor callback system
   - Validates data transformation and filtering
   - Tests attribute removal and modification

4. **Integration Tests**
   - Direct argument parsing tests
   - Lua print output tests
   - End-to-end functionality validation

## Code Quality Standards

### SOLID Principles Adherence

The codebase follows SOLID principles with clear separation of concerns:

#### Single Responsibility Principle (SRP)

- **`application.rs`**: Main application lifecycle and event loop
- **`lua_engine.rs`**: Lua VM management and script execution
- **`state.rs`**: Application state management
- **`tuichrome.rs`**: UI rendering and terminal management
- **`keyboard_management.rs`**: Input handling and keybinding execution
- **`parser.rs`**: Log format parsing logic
- **`recordlist.rs`**: Record storage and management

#### Open/Closed Principle (OCP)

- Parser system supports multiple formats through trait implementations
- Lua scripting system allows extension without modifying core code
- Settings system supports custom rules and configurations

#### Liskov Substitution Principle (LSP)

- All parser implementations follow the same interface
- Lua function wrappers maintain consistent behavior

#### Interface Segregation Principle (ISP)

- Focused interfaces for specific functionality
- Minimal dependencies between modules

#### Dependency Inversion Principle (DIP)

- High-level modules depend on abstractions (traits)
- Dependency injection through constructor parameters

### Code Organization

```
src/
├── lib.rs              # Library exports and module declarations
├── main.rs             # Application entry point
├── application.rs      # Main application struct and lifecycle
├── args.rs            # Command-line argument parsing
├── ast.rs             # Abstract syntax tree for filtering
├── completions.rs     # Command completion system
├── events.rs          # Event system definitions
├── keyboard_input.rs  # Raw keyboard input handling
├── keyboard_management.rs # Keybinding and command execution
├── lua_console.rs     # Lua REPL console implementation
├── lua_engine.rs      # Lua VM integration and API
├── parser.rs          # Log format parsing (logfmt, regex, etc.)
├── record.rs          # Individual log record data structure
├── recordlist.rs      # Collection and management of records
├── regex_cache.rs     # Regex compilation caching
├── settings.rs        # Configuration management
├── state.rs           # Application state management
├── tuichrome.rs       # Terminal UI rendering
└── utils.rs           # Utility functions
```

### Coding Conventions

1. **Error Handling**: Use `Result<T, E>` for fallible operations
2. **Documentation**: All public APIs must have doc comments
3. **Testing**: Write tests for new functionality
4. **Performance**: Use `cargo test --release` to verify performance
5. **Memory Management**: Be mindful of Lua VM memory usage

## Lua Integration Architecture

### Lua Engine (`src/lua_engine.rs`)

The Lua integration is sophisticated and follows these patterns:

- **Bytecode Compilation**: Scripts are compiled to bytecode for performance
- **State Management**: Application state is exposed via getter functions
- **Async Support**: Coroutine-based yielding for interactive scripts
- **Error Handling**: Comprehensive error reporting with stack traces

### Key Lua Functions Available

```lua
-- Navigation
vmove(amount)           -- Move vertically
vgoto(line_number)      -- Go to specific line
hmove(amount)           -- Move horizontally

-- UI Control
mode(mode_name)         -- Switch modes (normal, command, search, filter)
toggle_details()        -- Toggle detailed view
warning(message)        -- Show warning message

-- Record Access
get_record()            -- Get current record data
get_position()          -- Get current position
get_mode()              -- Get current mode

-- External Commands
exec(command)           -- Execute shell command
settings()              -- Open settings file

-- Interactive
ask(prompt)             -- Ask user for input (yields execution)

-- Utilities
url_encode(text)        -- URL encode text
url_decode(text)        -- URL decode text
escape_shell(text)      -- Escape for shell usage
```

### Record Processor System

The callback system allows processing each record as it's added:

```lua
-- Add a processor function
table.insert(record_processors, function(record)
    -- Process the record
    return {
        new_attribute = "value",
        unwanted_field = nil  -- Remove attribute
    }
end)
```

## Documentation Requirements

### When to Update Documentation

1. **README.md**: Only update for user-facing changes that affect:

   - Command-line interface
   - Keybindings
   - Configuration options
   - New features visible to end users

2. **LUA_SCRIPTING.md**: Update for:

   - New Lua functions
   - API changes
   - Scripting examples
   - Performance considerations

3. **RECORD_PROCESSORS.md**: Update for:

   - New processor capabilities
   - API changes to the callback system
   - Examples and usage patterns

4. **AGENTS.md**: Update for:

   - New features
   - API changes
   - Examples and usage patterns
   - Performance considerations
   - New insights from development that might be useful to agents

5. **Code Documentation**: Update doc comments for:
   - All public functions and methods
   - Complex algorithms
   - Performance-critical code paths

### Documentation Standards

- Use clear, concise language
- Provide examples for complex functionality
- Include error handling information
- Document performance characteristics
- Update version numbers and dates

## Development Workflow

### Before Making Changes

1. **Run Tests**: Ensure all existing tests pass
2. **Understand Architecture**: Review relevant modules
3. **Check Documentation**: Understand current API contracts

### During Development

1. **Write Tests First**: Follow TDD principles
2. **Maintain SOLID Principles**: Keep modules focused
3. **Handle Errors Gracefully**: Use proper error handling
4. **Consider Performance**: Profile critical paths

### After Making Changes

1. **Run Full Test Suite**: `cargo test`
2. **Check Performance**: `cargo test --release`
3. **Update Documentation**: As needed per guidelines above
4. **Verify Lua Integration**: Test script compilation and execution

## Common Pitfalls to Avoid

### Lua Integration Issues

- **Memory Leaks**: Be careful with Lua table creation in loops
- **Error Propagation**: Ensure Lua errors are properly handled
- **State Consistency**: Keep Rust and Lua state synchronized
- **Performance**: Avoid expensive operations in hot paths

### Testing Issues

- **Never Run Binary**: Use unit tests instead
- **State Management**: Reset state between tests
- **Async Testing**: Handle coroutine suspension properly
- **Error Cases**: Test both success and failure scenarios

### Code Quality Issues

- **Circular Dependencies**: Avoid between modules
- **Large Functions**: Keep functions focused and small
- **Error Handling**: Don't ignore errors or use `unwrap()` inappropriately
- **Documentation**: Don't leave public APIs undocumented

## Performance Considerations

### Critical Performance Areas

1. **Lua Script Execution**: Scripts run frequently (on every keypress)
2. **Record Processing**: Large log files require efficient processing
3. **UI Rendering**: Smooth scrolling and real-time updates
4. **Memory Usage**: Lua VM memory management

### Performance Testing

```bash
# Run performance tests
cargo test --release -- --nocapture

# Profile specific functionality
cargo test lua_function_tests --release
```

## Debugging and Troubleshooting

### Common Issues

1. **Lua Script Compilation Errors**: Check syntax and function availability
2. **State Inconsistency**: Verify state updates are properly synchronized
3. **Memory Issues**: Monitor Lua VM memory usage
4. **Performance Degradation**: Profile hot paths and optimize

### Debug Tools

- **Lua REPL**: Use `lua_repl()` function for interactive debugging
- **Logging**: Use `debug_log()` function for debugging output
- **State Inspection**: Use `get_position()`, `get_mode()`, etc.

## Contributing Guidelines

### Code Review Checklist

- [ ] All tests pass
- [ ] No direct binary execution
- [ ] SOLID principles followed
- [ ] Documentation updated appropriately
- [ ] Error handling implemented
- [ ] Performance considerations addressed
- [ ] Lua integration tested
- [ ] Memory management considered

### Commit Message Format

```
type(scope): brief description

Detailed description of changes made.

- List of specific changes
- Any breaking changes
- Performance implications
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`

## Resources

### Key Files to Understand

- `src/lua_engine.rs`: Core Lua integration
- `src/state.rs`: Application state management
- `src/application.rs`: Main application lifecycle
- `tests/lua_function_tests.rs`: Comprehensive test examples
- `settings.yaml`: Default configuration and keybindings
- `_init.lua`: Lua initialization and utility functions

### External Dependencies

- **mlua**: Lua VM integration
- **ratatui**: Terminal UI framework
- **crossterm**: Terminal control
- **serde**: Serialization/deserialization
- **regex**: Pattern matching
- **notify**: File system watching

This guide should help AI agents work effectively with the TailTales codebase while maintaining code quality and avoiding common pitfalls.
