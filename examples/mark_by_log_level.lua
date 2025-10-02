-- Example Lua script demonstrating for_each_record functionality
-- This script shows how to mark records by log level and clean up marks

-- Mark records by log level with different colors
function mark_by_log_level()
    for_each_record(function(record)
        if record.level == "ERROR" then
            return {mark = "red white", severity = "critical"}
        elseif record.level == "WARN" then
            return {mark = "yellow black", severity = "warning"}
        elseif record.level == "INFO" then
            return {mark = "green black", severity = "info"}
        elseif record.level == "DEBUG" then
            return {mark = "blue white", severity = "debug"}
        end
        return nil
    end)
end

-- Clean up marks by removing them
function cleanup_marks()
    for_each_record(function(record)
        if record.mark then
            return {mark = "__REMOVE__"}
        end
        return nil -- Skip records that don't have marks
    end)
end

documentation.mark_by_log_level = {
    name = "mark_by_log_level",
    description = "Mark records by log level",
    parameters = {},
    return_value = "none",
    category = "processing"
}

documentation.cleanup_marks = { 
    name = "cleanup_marks",
    description = "Remove marks from records",
    parameters = {},
    return_value = "none",
    category = "processing"
}