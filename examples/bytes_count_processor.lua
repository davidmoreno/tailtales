-- Example Lua script demonstrating record processors
-- This script adds a "bytes_count" attribute to each record with the size of the original line

-- Add a processor function to the record_processors array
table.insert(record_processors, function(record)
    -- Calculate the byte count of the original line
    local bytes_count = #record.original
    
    -- Return a table with the new attribute
    return {
        bytes_count = tostring(bytes_count)
    }
end)

print("Bytes count processor added to record_processors")
print("Each new record will now have a 'bytes_count' attribute with the line size in bytes")

documentation.bytes_count_processor = {
    name = "bytes_count_processor",
    description = "Add a 'bytes_count' attribute to each record with the size of the original line",
    parameters = {},
    return_value = "none",
    category = "processing"
}
