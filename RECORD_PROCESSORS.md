# Record Processors Callback System

TailTales now supports a callback system that allows you to process each new record as it's added. This system is implemented through the `record_processors` array in Lua.

## How it Works

1. **Initialization**: The `record_processors` array is initialized in `_init.lua`
2. **Processing**: When a new record is added, all callbacks in the array are executed
3. **Input**: Each callback receives the current record (with builtin attributes already set)
4. **Output**: Each callback returns a table with new attributes to add/update/remove
5. **Removal**: Use `nil` values to remove attributes

## Usage

### Adding a Processor

```lua
-- Add a processor function to the record_processors array
table.insert(record_processors, function(record)
    -- Your processing logic here
    return {
        new_attribute = "value",
        another_attribute = "another_value"
    }
end)
```

### Example: Bytes Count Processor

```lua
-- Add a processor that calculates the byte count of each line
table.insert(record_processors, function(record)
    local bytes_count = #record.original
    return {
        bytes_count = tostring(bytes_count)
    }
end)
```

### Example: Remove Attributes

```lua
-- Remove unwanted attributes
table.insert(record_processors, function(record)
    return {
        unwanted_field = nil  -- This removes the attribute
    }
end)
```

## Record Structure

The record passed to processors contains:

- `original`: The raw log line
- `word_count`: Number of words (automatically added)
- `filename`: Source filename (if available)
- `line_number`: Line number in the file
- Any other attributes extracted by builtin extractors

## Integration Points

The callback system is integrated at the following points:

1. **New Records**: When new records are added via `TuiEvent::NewRecord`
2. **File Processing**: During parallel file reading
3. **Stream Processing**: When reading from stdin or command output

## Error Handling

- If a processor function fails, an error message is printed but processing continues
- The system is designed to be robust and not break the main application flow

## Performance Considerations

- Processors are executed synchronously for each record
- Keep processor functions lightweight for best performance
- Complex processing should be done in background threads if needed

## Testing

You can test the callback system using the provided example scripts:

```bash
# Load the bytes count processor and process a log file
./tt --lua examples/bytes_count_processor.lua your_log_file.log
```

The processor will automatically add a `bytes_count` attribute to each record showing the size of the original line in bytes.

## Usage with Command Line

The `--lua` flag allows you to execute a Lua script before processing log files:

```bash
# Execute a script and then process files normally
./tt --lua my_processors.lua access.log error.log

# Execute script and read from stdin
./tt --lua my_processors.lua -

# Execute script and run a command
./tt --lua my_processors.lua -- tail -f /var/log/app.log
```

The script executes in normal mode (not REPL mode), so you stay in the main view to see the processed records.
