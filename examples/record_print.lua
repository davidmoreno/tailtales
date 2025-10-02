-- Example script demonstrating get_record() and get_record_data() functions
-- This shows how to access record data by position

-- Function to pretty print a record by position
function print_record(position)
    local record = get_record(position)
    
    if not record then
        print("Record " .. tostring(position) .. ": nil (no record at this position)")
        return
    end
    
    print("Record " .. tostring(position) .. ":")
    print("  Original: " .. record.original)
    print("  Index: " .. tostring(record.index))
    
    -- Print all available fields
    local field_count = 0
    for key, value in pairs(record) do
        if key ~= "original" and key ~= "index" then
            print("  " .. key .. ": " .. tostring(value))
            field_count = field_count + 1
        end
    end
    
    if field_count == 0 then
        print("  (no additional fields)")
    end
    
    print("") -- Empty line for readability
end

-- Function to pretty print current record
function print_current_record()
    local record = get_record()
    
    if not record then
        print("Current record: nil")
        return
    end
    
    print("Current record:")
    print("  Original: " .. record.original)
    print("  Index: " .. tostring(record.index))
    
    -- Print all available fields
    local field_count = 0
    for key, value in pairs(record) do
        if key ~= "original" and key ~= "index" then
            print("  " .. key .. ": " .. tostring(value))
            field_count = field_count + 1
        end
    end
    
    if field_count == 0 then
        print("  (no additional fields)")
    end
    
    print("") -- Empty line for readability
end

documentation.print_record = {
    name = "print_record",
    description = "Pretty print a record by position",
    parameters = {"position (number) - Record position"},
    return_value = "none",
    category = "records"
}

documentation.print_current_record = {
    name = "print_current_record",
    description = "Pretty print the current record",
    parameters = {},
    return_value = "none",
    category = "records"
}