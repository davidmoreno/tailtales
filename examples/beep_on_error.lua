-- Beep on Error Example Script
-- This script demonstrates how to use the beep() function to play a sound
-- whenever an error message appears in the records.

-- Function to check if a record contains an error message
function is_error_record(record)
    if not record then
        return false
    end
    
    -- Check various common error indicators
    local error_indicators = {
        "level", "severity", "status", "type", "message", "log_level"
    }
    
    for _, indicator in ipairs(error_indicators) do
        local value = record[indicator]
        if value then
            local str_value = tostring(value):lower()
            -- Check for error keywords
            if str_value:match("error") or 
               str_value:match("err") or
               str_value:match("fail") or
               str_value:match("exception") or
               str_value:match("critical") or
               str_value:match("fatal") or
               str_value:match("panic") then
                return true
            end
        end
    end
    
    -- Check if status code indicates error (4xx, 5xx)
    local status = record["status"]
    if status then
        local status_num = tonumber(status)
        if status_num and (status_num >= 400) then
            return true
        end
    end
    
    -- Check if HTTP status code indicates error
    local http_status = record["http_status"]
    if http_status then
        local status_num = tonumber(http_status)
        if status_num and (status_num >= 400) then
            return true
        end
    end
    
    return false
end


table.insert(record_processors, function(record)
    if is_error_record(record) then
        beep()
    end
    return {}
end)

