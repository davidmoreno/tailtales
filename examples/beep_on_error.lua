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

-- Process each record and beep for errors
function process_records_with_beep()
    local total_records = get_record_count()
    if total_records == 0 then
        print("No records to process")
        return
    end
    
    local error_count = 0
    local processed = 0
    
    print("Processing records and beeping for errors...")
    print("Total records: " .. total_records)
    print("")
    
    for i = 1, total_records do
        local record = get_record(i)
        if record then
            processed = processed + 1
            
            if is_error_record(record) then
                error_count = error_count + 1
                print("ERROR FOUND at record " .. i .. " - BEEP!")
                
                -- Play beep sound
                if beep() then
                    print("  ✓ Beep played successfully")
                else
                    print("  ✗ Failed to play beep")
                end
                
                -- Show some details about the error
                local message = record["message"] or record["msg"] or record["text"] or "No message"
                print("  Message: " .. tostring(message):sub(1, 100) .. (tostring(message):len() > 100 and "..." or ""))
                print("")
            end
        end
    end
    
    print("Processing complete!")
    print("Records processed: " .. processed)
    print("Errors found: " .. error_count)
    if error_count > 0 then
        print("Beep count: " .. error_count)
    end
end

-- Alternative: Use for_each_record to process with beeping
function beep_on_error_processor(record)
    if is_error_record(record) then
        print("ERROR detected - playing beep...")
        beep()
        
        -- Add a marker attribute to show this record had an error
        return {
            error_beeped = "true",
            error_timestamp = os.date("%Y-%m-%d %H:%M:%S")
        }
    end
    
    return nil  -- No changes for non-error records
end

-- Main execution
print("=== Beep on Error Example ===")
print("This script will process all records and play a beep sound")
print("whenever an error message is detected.")
print("")
print("Available functions:")
print("  process_records_with_beep()  - Process all records and beep for errors")
print("  for_each_record(beep_on_error_processor)  - Use record processor approach")
print("  is_error_record(record)     - Check if a specific record contains an error")
print("  beep()                      - Play a beep sound")
print("")
print("Example usage:")
print("  process_records_with_beep()")
print("  -- or --")
print("  for_each_record(beep_on_error_processor)")
print("")

