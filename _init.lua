-- TailTales Lua Initialization Script
-- This file is loaded automatically when the Lua engine starts

-- Python-like dir() function that lists available functions and variables
function dir(obj)
    local items = {}
    
    if obj == nil then
        -- List global variables and functions
        for k, v in pairs(_G) do
            if type(v) == "function" then
                table.insert(items, k .. "()")
            else
                table.insert(items, k)
            end
        end
    else
        -- List properties of the given object
        if type(obj) == "table" then
            for k, v in pairs(obj) do
                if type(v) == "function" then
                    table.insert(items, k .. "()")
                else
                    table.insert(items, k)
                end
            end
        else
            -- For non-table objects, try to get metatable
            local mt = getmetatable(obj)
            if mt and mt.__index then
                for k, v in pairs(mt.__index) do
                    if type(v) == "function" then
                        table.insert(items, k .. "()")
                    else
                        table.insert(items, k)
                    end
                end
            end
        end
    end
    
    -- Sort the items
    table.sort(items)
    
    -- Print them in a nice format
    if #items == 0 then
        print("No items found")
        return
    end
    
    -- Print in columns if there are many items
    if #items > 10 then
        local cols = 3
        local rows = math.ceil(#items / cols)
        
        for row = 1, rows do
            local line = ""
            for col = 1, cols do
                local idx = (col - 1) * rows + row
                if idx <= #items then
                    line = line .. string.format("%-25s", items[idx])
                end
            end
            print(line)
        end
    else
        -- Simple list for few items
        for _, item in ipairs(items) do
            print(item)
        end
    end
end

-- Helper function to check if a value is callable
function callable(obj)
    return type(obj) == "function" or 
           (type(obj) == "table" and getmetatable(obj) and getmetatable(obj).__call)
end

-- Helper function to get type information
function typeof(obj)
    local t = type(obj)
    if t == "table" then
        local mt = getmetatable(obj)
        if mt and mt.__name then
            return mt.__name
        end
        -- Check if it looks like an array
        local len = #obj
        if len > 0 then
            local all_numeric = true
            for k, v in pairs(obj) do
                if type(k) ~= "number" or k < 1 or k > len then
                    all_numeric = false
                    break
                end
            end
            if all_numeric then
                return "array[" .. len .. "]"
            end
        end
        return "table"
    elseif t == "userdata" then
        local mt = getmetatable(obj)
        if mt and mt.__name then
            return mt.__name
        end
    end
    return t
end

-- Help function that provides usage information
function help(topic)
    if topic == nil then
        print("TailTales Lua REPL Help:")
        print("  dir()           - List all available functions and variables")
        print("  dir(obj)        - List properties of an object")
        print("  help()          - Show this help message")
        print("  help('topic')   - Show help for specific topic")
        print("")
        print("Available topics: 'functions', 'navigation', 'records', 'analysis'")
        print("")
        print("Core functions:")
        print("  get_position()  - Get current line position")
        print("  get_record()    - Get current record data")
        print("  quit()          - Exit the application")
        print("  warning(msg)    - Show warning message")
        print("  print(...)      - Output text to REPL")
        print("  clear()         - Clear REPL console buffer")
    elseif topic == "functions" then
        print("TailTales Functions:")
        print("  Movement: vmove(n), vgoto(n), hmove(n)")
        print("  Marks: toggle_mark(color), move_to_next_mark(), move_to_prev_mark()")
        print("  Mode: mode(name), toggle_details()")
        print("  System: exec(cmd), refresh_screen(), clear_records()")
        print("  Utility: url_encode(text)")
    elseif topic == "navigation" then
        print("Navigation Help:")
        print("  vmove(n)        - Move cursor n lines up/down")
        print("  vgoto(n)        - Go to specific line number")
        print("  hmove(n)        - Scroll horizontally")
        print("  get_position()  - Get current line number")
    elseif topic == "records" then
        print("Record Access Help:")
        print("  get_record()    - Get current record as table")
        print("  record.original - Raw log line")
        print("  record.timestamp- Parsed timestamp (if available)")
        print("  record.*        - Other parsed fields")
        print("")
        print("Analysis Functions:")
        print("  histogram(attr, buckets) - Calculate histogram data for attribute")
        print("  print_histogram(attr, buckets) - Display histogram with ASCII art")
        print("  extract_number(value) - Extract numeric value from string with units")
        print("  count(attr) - Count occurrences of different values for attribute")
        print("  print_count(attr) - Display count results in formatted table")
    elseif topic == "analysis" then
        print("Analysis Functions Help:")
        print("  histogram(attribute, buckets)")
        print("    - Calculate histogram data for a numeric attribute")
        print("    - buckets: number of histogram buckets (default: 10)")
        print("    - Returns table with bucket counts, min/max, etc.")
        print("")
        print("  print_histogram(attribute, buckets)")
        print("    - Display histogram with ASCII art visualization")
        print("    - Uses histogram() internally")
        print("    - Shows range, bucket size, counts, and percentages")
        print("")
        print("  extract_number(value)")
        print("    - Extract numeric value from string with units")
        print("    - Supports: ms, us, ns, s, m, h")
        print("    - Converts all to base unit (seconds)")
        print("")
        print("Examples:")
        print("  print_histogram('duration', 15)")
        print("  print_histogram('response_time')")
        print("  local data = histogram('latency', 20)")
        print("")
        print("  count(attribute)")
        print("    - Count occurrences of different values for an attribute")
        print("    - Returns sorted table with value/count pairs")
        print("    - Useful for categorical data (log levels, status codes, etc.)")
        print("")
        print("  print_count(attribute)")
        print("    - Display count results in formatted table")
        print("    - Shows value, count, percentage, and ASCII bar")
        print("    - Sorted by count (descending) then value (ascending)")
        print("")
        print("Count Examples:")
        print("  print_count('level')     -- Count log levels")
        print("  print_count('status')    -- Count status codes")
        print("  local data = count('type') -- Get raw count data")
    else
        print("Unknown help topic: " .. tostring(topic))
        print("Available topics: 'functions', 'navigation', 'records', 'analysis'")
    end
end

-- Helper function to extract numeric value from string (handles units like "10ms", "5.2s", etc.)
function extract_number(value)
    if type(value) == "number" then
        return value
    end
    
    if type(value) ~= "string" then
        return nil
    end
    
    -- Remove whitespace
    value = value:gsub("%s+", "")
    
    -- Try to extract number from string (handles units like ms, s, us, etc.)
    local num_str = value:match("^([%d%.]+)")
    if num_str then
        local num = tonumber(num_str)
        if num then
            -- Check for common units and convert to base unit (seconds)
            if value:match("ms$") then
                return num / 1000
            elseif value:match("us$") or value:match("μs$") then
                return num / 1000000
            elseif value:match("ns$") then
                return num / 1000000000
            elseif value:match("s$") then
                return num
            elseif value:match("m$") then
                return num * 60
            elseif value:match("h$") then
                return num * 3600
            else
                -- No unit, assume it's already in base unit
                return num
            end
        end
    end
    
    return nil
end

-- Calculate histogram data from all records for a given attribute
function histogram(attribute, buckets)
    buckets = buckets or 10
    
    local total_records = get_record_count()
    if total_records == 0 then
        return {buckets = {}, min = 0, max = 0, count = 0, skipped = 0}
    end
    
    local values = {}
    local skipped = 0
    
    -- Collect all numeric values for the attribute
    for i = 1, total_records do
        local record = get_record(i)
        if record and record[attribute] then
            local num_value = extract_number(record[attribute])
            if num_value then
                table.insert(values, num_value)
            else
                skipped = skipped + 1
            end
        else
            skipped = skipped + 1
        end
    end
    
    if #values == 0 then
        return {buckets = {}, min = 0, max = 0, count = 0, skipped = skipped}
    end
    
    -- Sort values to find min/max
    table.sort(values)
    local min_val = values[1]
    local max_val = values[#values]
    
    -- Handle edge case where all values are the same
    if min_val == max_val then
        local buckets = {}
        for i = 1, buckets do
            buckets[i] = 0
        end
        buckets[1] = #values
        return {
            buckets = buckets,
            min = min_val,
            max = max_val,
            count = #values,
            skipped = skipped,
            bucket_size = 0
        }
    end
    
    -- Calculate bucket size
    local bucket_size = (max_val - min_val) / buckets
    
    -- Initialize buckets
    local bucket_counts = {}
    for i = 1, buckets do
        bucket_counts[i] = 0
    end
    
    -- Distribute values into buckets
    for _, value in ipairs(values) do
        local bucket_index = math.min(buckets, math.max(1, math.floor((value - min_val) / bucket_size) + 1))
        bucket_counts[bucket_index] = bucket_counts[bucket_index] + 1
    end
    
    return {
        buckets = bucket_counts,
        min = min_val,
        max = max_val,
        count = #values,
        skipped = skipped,
        bucket_size = bucket_size
    }
end

-- Print histogram with ASCII art visualization
function print_histogram(attribute, buckets)
    buckets = buckets or 10
    
    local hist_data = histogram(attribute, buckets)
    
    if hist_data.count == 0 then
        print("No numeric data found for attribute '" .. attribute .. "'")
        if hist_data.skipped > 0 then
            print("Skipped " .. hist_data.skipped .. " records (non-numeric or missing values)")
        end
        return
    end
    
    print("Histogram for attribute '" .. attribute .. "'")
    print("Total records: " .. hist_data.count .. " (skipped: " .. hist_data.skipped .. ")")
    print("Range: " .. string.format("%.3f", hist_data.min) .. " to " .. string.format("%.3f", hist_data.max))
    print("Bucket size: " .. string.format("%.3f", hist_data.bucket_size))
    print("")
    
    -- Find max count for scaling
    local max_count = 0
    for _, count in ipairs(hist_data.buckets) do
        max_count = math.max(max_count, count)
    end
    
    -- Print histogram with ASCII bars
    local bar_width = 50  -- Maximum bar width in characters
    
    for i, count in ipairs(hist_data.buckets) do
        local bucket_start = hist_data.min + (i - 1) * hist_data.bucket_size
        local bucket_end = hist_data.min + i * hist_data.bucket_size
        
        -- Create ASCII bar
        local bar_length = 0
        local bar = ""
        if max_count > 0 then
            bar_length = math.floor((count / max_count) * bar_width)
            bar = string.rep("█", bar_length)
        end
        
        -- Format bucket range
        local range_str = string.format("[%.3f - %.3f]", bucket_start, bucket_end)
        
        -- Print line with count, bar, and percentage
        local percentage = (hist_data.count > 0) and (count / hist_data.count * 100) or 0
        print(string.format("%-20s %6d %s %5.1f%%", range_str, count, bar, percentage))
    end
    
    print("")
end

-- Count occurrences of different values for a given attribute across all records
function count(attribute)
    local total_records = get_record_count()
    if total_records == 0 then
        return {}
    end
    
    local counts = {}
    local skipped = 0
    
    -- Count occurrences of each value for the attribute
    for i = 1, total_records do
        local record = get_record(i)
        if record and record[attribute] then
            local value = tostring(record[attribute])
            if counts[value] then
                counts[value] = counts[value] + 1
            else
                counts[value] = 1
            end
        else
            skipped = skipped + 1
        end
    end
    
    -- Convert to sorted table format
    local result = {}
    for value, count in pairs(counts) do
        table.insert(result, {value = value, count = count})
    end
    
    -- Sort by count (descending) then by value (ascending)
    table.sort(result, function(a, b)
        if a.count == b.count then
            return a.value < b.value
        end
        return a.count > b.count
    end)
    
    -- Add metadata
    result._metadata = {
        total_records = total_records,
        skipped = skipped,
        unique_values = #result
    }
    
    return result
end

-- Print count results in a formatted table
function print_count(attribute)
    local count_data = count(attribute)
    
    if #count_data == 0 then
        print("No data found for attribute '" .. attribute .. "'")
        if count_data._metadata and count_data._metadata.skipped > 0 then
            print("Skipped " .. count_data._metadata.skipped .. " records (missing values)")
        end
        return
    end
    
    local metadata = count_data._metadata
    print("Count for attribute '" .. attribute .. "'")
    print("Total records: " .. metadata.total_records .. " (skipped: " .. metadata.skipped .. ")")
    print("Unique values: " .. metadata.unique_values)
    print("")
    
    -- Find max count for percentage calculation
    local max_count = 0
    for _, item in ipairs(count_data) do
        max_count = math.max(max_count, item.count)
    end
    
    -- Print header
    print(string.format("%-20s %8s %6s %s", "Value", "Count", "Pct", "Bar"))
    print(string.rep("-", 50))
    
    -- Print each value with count, percentage, and ASCII bar
    for _, item in ipairs(count_data) do
        local percentage = (metadata.total_records > 0) and (item.count / metadata.total_records * 100) or 0
        local bar_length = math.floor((item.count / max_count) * 20)
        local bar = string.rep("█", bar_length)
        
        print(string.format("%-20s %8d %5.1f%% %s", 
            item.value, item.count, percentage, bar))
    end
    
    print("")
end

print("TailTales Lua environment initialized. Use dir() to explore or help() for assistance.")
