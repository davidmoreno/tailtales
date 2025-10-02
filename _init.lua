-- TailTales Lua Initialization Script
-- This file is loaded automatically when the Lua engine starts

-- Documentation table for all Lua functions exported by TailTales
-- This table contains metadata for all functions registered in the Lua engine
documentation = {
    -- Core Application Functions
    quit = {
        name = "quit",
        description = "Exit the TailTales application",
        parameters = {},
        return_value = "none",
        category = "core"
    },
    
    warning = {
        name = "warning",
        description = "Display a warning message to the user",
        parameters = {"msg (string) - The warning message to display"},
        return_value = "none",
        category = "core"
    },
    
    -- Navigation Functions
    vmove = {
        name = "vmove",
        description = "Move the cursor vertically by n lines",
        parameters = {"n (number) - Number of lines to move (positive = down, negative = up)"},
        return_value = "none",
        category = "navigation"
    },
    
    vgoto = {
        name = "vgoto",
        description = "Jump to a specific line number",
        parameters = {"n (number) - Line number to jump to (1-based)"},
        return_value = "none",
        category = "navigation"
    },
    
    hmove = {
        name = "hmove",
        description = "Move the view horizontally by n characters",
        parameters = {"n (number) - Number of characters to scroll (positive = right, negative = left)"},
        return_value = "none",
        category = "navigation"
    },
    
    move_top = {
        name = "move_top",
        description = "Move to the first record",
        parameters = {},
        return_value = "none",
        category = "navigation"
    },
    
    move_bottom = {
        name = "move_bottom",
        description = "Move to the last record",
        parameters = {},
        return_value = "none",
        category = "navigation"
    },
    
    -- Search Functions
    search_next = {
        name = "search_next",
        description = "Find the next occurrence of the current search term",
        parameters = {},
        return_value = "none",
        category = "search"
    },
    
    search_prev = {
        name = "search_prev",
        description = "Find the previous occurrence of the current search term",
        parameters = {},
        return_value = "none",
        category = "search"
    },
    
    -- Mark Functions
    toggle_mark = {
        name = "toggle_mark",
        description = "Toggle a mark on the current record",
        parameters = {"color (string, optional) - Mark color (default: 'yellow')"},
        return_value = "none",
        category = "marks"
    },
    
    move_to_next_mark = {
        name = "move_to_next_mark",
        description = "Move to the next marked record",
        parameters = {},
        return_value = "none",
        category = "marks"
    },
    
    move_to_prev_mark = {
        name = "move_to_prev_mark",
        description = "Move to the previous marked record",
        parameters = {},
        return_value = "none",
        category = "marks"
    },
    
    -- Mode and UI Functions
    mode = {
        name = "mode",
        description = "Change the application mode",
        parameters = {"mode_name (string) - Mode to switch to (normal, search, filter, command, warning, script_input, lua_repl)"},
        return_value = "none",
        category = "ui"
    },
    
    toggle_details = {
        name = "toggle_details",
        description = "Toggle the details view on/off",
        parameters = {},
        return_value = "none",
        category = "ui"
    },
    
    lua_repl = {
        name = "lua_repl",
        description = "Enter Lua REPL mode",
        parameters = {},
        return_value = "none",
        category = "ui"
    },
    
    -- System Functions
    refresh_screen = {
        name = "refresh_screen",
        description = "Force a screen refresh",
        parameters = {},
        return_value = "none",
        category = "system"
    },
    
    clear_records = {
        name = "clear_records",
        description = "Clear all records from memory",
        parameters = {},
        return_value = "none",
        category = "system"
    },
    
    clear = {
        name = "clear",
        description = "Clear the REPL console buffer",
        parameters = {},
        return_value = "none",
        category = "system"
    },
    
    exec = {
        name = "exec",
        description = "Execute an external command",
        parameters = {"command (string) - Command to execute"},
        return_value = "boolean - true if successful, false if failed",
        category = "system"
    },
    
    settings = {
        name = "settings",
        description = "Open the settings interface",
        parameters = {},
        return_value = "none",
        category = "system"
    },
    
    -- Filter Functions
    filter = {
        name = "filter",
        description = "Filter records using a filter expression",
        parameters = {"expression (string) - Filter expression (e.g., 'level == \"ERROR\"', 'status >= 400')"},
        return_value = "number - Number of records after filtering",
        category = "filtering"
    },
    
    -- Record Access Functions
    get_record = {
        name = "get_record",
        description = "Get record data at specified index or current position",
        parameters = {"index (number, optional) - Record index (1-based), defaults to current position"},
        return_value = "table or nil - Record data table or nil if not found",
        category = "records"
    },
    
    get_record_data = {
        name = "get_record_data",
        description = "Alias for get_record()",
        parameters = {"index (number, optional) - Record index (1-based), defaults to current position"},
        return_value = "table or nil - Record data table or nil if not found",
        category = "records"
    },
    
    get_position = {
        name = "get_position",
        description = "Get the current record position",
        parameters = {},
        return_value = "number - Current position (1-based)",
        category = "records"
    },
    
    get_record_count = {
        name = "get_record_count",
        description = "Get the total number of records",
        parameters = {},
        return_value = "number - Total number of records",
        category = "records"
    },
    
    -- Record Attribute Functions
    update_record_attribute = {
        name = "update_record_attribute",
        description = "Add, update, or remove an attribute from a record",
        parameters = {"index (number) - Record index (1-based)", "key (string) - Attribute name", "value (string or nil) - Attribute value (nil to remove)"},
        return_value = "boolean - true if successful, false if record not found",
        category = "attributes"
    },
    
    get_record_attribute = {
        name = "get_record_attribute",
        description = "Get the value of an attribute from a record",
        parameters = {"index (number) - Record index (1-based)", "key (string) - Attribute name"},
        return_value = "string or nil - Attribute value or nil if not found",
        category = "attributes"
    },
    
    -- State Getter Functions
    get_viewport = {
        name = "get_viewport",
        description = "Get viewport information",
        parameters = {},
        return_value = "table - Viewport data (height, width, scroll_top, scroll_left, view_details)",
        category = "state"
    },
    
    get_mode = {
        name = "get_mode",
        description = "Get the current application mode",
        parameters = {},
        return_value = "string - Current mode name",
        category = "state"
    },
    
    get_search = {
        name = "get_search",
        description = "Get the current search term",
        parameters = {},
        return_value = "string - Current search term",
        category = "state"
    },
    
    get_filter = {
        name = "get_filter",
        description = "Get the current filter expression",
        parameters = {},
        return_value = "string - Current filter expression",
        category = "state"
    },
    
    get_command = {
        name = "get_command",
        description = "Get the current command input",
        parameters = {},
        return_value = "string - Current command input",
        category = "state"
    },
    
    get_warning = {
        name = "get_warning",
        description = "Get the current warning message",
        parameters = {},
        return_value = "string - Current warning message",
        category = "state"
    },
    
    -- Utility Functions
    url_encode = {
        name = "url_encode",
        description = "URL encode a string",
        parameters = {"input (string) - String to encode"},
        return_value = "string - URL encoded string",
        category = "utility"
    },
    
    url_decode = {
        name = "url_decode",
        description = "URL decode a string",
        parameters = {"input (string) - String to decode"},
        return_value = "string - URL decoded string",
        category = "utility"
    },
    
    escape_shell = {
        name = "escape_shell",
        description = "Escape a string for safe shell execution",
        parameters = {"input (string) - String to escape"},
        return_value = "string - Shell-escaped string",
        category = "utility"
    },
    
    debug_log = {
        name = "debug_log",
        description = "Log a debug message",
        parameters = {"msg (string) - Debug message"},
        return_value = "none",
        category = "utility"
    },
    
    -- Analysis Functions (defined in _init.lua)
    histogram = {
        name = "histogram",
        description = "Calculate histogram data for a numeric attribute",
        parameters = {"attribute (string) - Attribute name", "buckets (number, optional) - Number of histogram buckets (default: 10)"},
        return_value = "table - Histogram data (buckets, min, max, count, skipped, bucket_size)",
        category = "analysis"
    },
    
    print_histogram = {
        name = "print_histogram",
        description = "Display histogram with ASCII art visualization",
        parameters = {"attribute (string) - Attribute name", "buckets (number, optional) - Number of histogram buckets (default: 10)"},
        return_value = "none",
        category = "analysis"
    },
    
    extract_number = {
        name = "extract_number",
        description = "Extract numeric value from string with units (ms, s, us, ns, m, h)",
        parameters = {"value (string or number) - Value to extract number from"},
        return_value = "number or nil - Extracted number in base unit (seconds) or nil if invalid",
        category = "analysis"
    },
    
    count = {
        name = "count",
        description = "Count occurrences of different values for an attribute",
        parameters = {"attribute (string) - Attribute name"},
        return_value = "table - Sorted array of {value, count} objects",
        category = "analysis"
    },
    
    print_count = {
        name = "print_count",
        description = "Display count results in formatted table",
        parameters = {"attribute (string) - Attribute name"},
        return_value = "none",
        category = "analysis"
    },
    
    -- Processing Functions (defined in _init.lua)
    for_each_record = {
        name = "for_each_record",
        description = "Process each record with a custom function",
        parameters = {"processor_func (function) - Function that receives a record and returns a table"},
        return_value = "none",
        category = "processing"
    },
    
    -- Helper Functions (defined in _init.lua)
    dir = {
        name = "dir",
        description = "List available functions and variables",
        parameters = {"obj (optional) - Object to inspect, defaults to global scope"},
        return_value = "none",
        category = "helper"
    },
    
    callable = {
        name = "callable",
        description = "Check if a value is callable",
        parameters = {"obj - Value to check"},
        return_value = "boolean - true if callable, false otherwise",
        category = "helper"
    },
    
    typeof = {
        name = "typeof",
        description = "Get detailed type information for a value",
        parameters = {"obj - Value to inspect"},
        return_value = "string - Detailed type information",
        category = "helper"
    },
    
    ask = {
        name = "ask",
        description = "Prompt user for input (used in coroutines)",
        parameters = {"prompt (string) - Prompt message"},
        return_value = "string - User input",
        category = "helper"
    }
}

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

-- Enhanced help function that uses the documentation table
function help(topic)
    -- if topic is a function, use the function name as the topic
    if type(topic) == "function" then
        topic = topic.name
    end

    if topic == nil then
        -- Show general help with categories
        print("TailTales Lua REPL Help:")
        print("  help()                    - Show this help message")
        print("  help('category')          - Show functions in a category")
        print("  help('function_name')     - Show help for a specific function")
        print("  dir()                     - List all available functions and variables")
        print("")
        
        -- Get all unique categories
        local categories = {}
        for _, func_info in pairs(documentation) do
            if not categories[func_info.category] then
                categories[func_info.category] = true
            end
        end
        
        -- Sort categories
        local sorted_categories = {}
        for category, _ in pairs(categories) do
            table.insert(sorted_categories, category)
        end
        table.sort(sorted_categories)
        
        print("Available categories:")
        for _, category in ipairs(sorted_categories) do
            local count = 0
            for _, func_info in pairs(documentation) do
                if func_info.category == category then
                    count = count + 1
                end
            end
            print(string.format("  %-15s (%d functions)", category, count))
        end
        
        print("")
        print("Quick examples:")
        print("  help('navigation')        - Show navigation functions")
        print("  help('get_record')        - Show help for get_record function")
        print("  help('analysis')          - Show analysis functions")
        
    elseif documentation[topic] then
        -- Show help for a specific function
        local func_info = documentation[topic]
        print(string.format("Function: %s", func_info.name))
        print(string.format("Category: %s", func_info.category))
        print(string.format("Description: %s", func_info.description))
        
        if #func_info.parameters > 0 then
            print("Parameters:")
            for _, param in ipairs(func_info.parameters) do
                print(string.format("  %s", param))
            end
        else
            print("Parameters: none")
        end
        
        print(string.format("Returns: %s", func_info.return_value))
        
    else
        -- Check if it's a category
        local found_functions = {}
        for _, func_info in pairs(documentation) do
            if func_info.category == topic then
                table.insert(found_functions, func_info)
            end
        end
        
        if #found_functions > 0 then
            -- Show functions in the category
            print(string.format("Functions in category '%s':", topic))
            print("")
            
            -- Sort functions by name
            table.sort(found_functions, function(a, b) return a.name < b.name end)
            
            for _, func_info in ipairs(found_functions) do
                print(string.format("  %s", func_info.name))
                print(string.format("    %s", func_info.description))
                
                if #func_info.parameters > 0 then
                    print("    Parameters:")
                    for _, param in ipairs(func_info.parameters) do
                        print(string.format("      %s", param))
                    end
                end
                
                print(string.format("    Returns: %s", func_info.return_value))
                print("")
            end
            
            print(string.format("Use help('%s') for detailed help on any function.", found_functions[1].name))
            
        else
            -- Unknown topic
            print(string.format("Unknown help topic: %s", tostring(topic)))
            print("")
            print("Available options:")
            print("  help()                    - Show general help")
            print("  help('category')          - Show functions in a category")
            print("  help('function_name')     - Show help for a specific function")
            print("")
            
            -- Show available categories
            local categories = {}
            for _, func_info in pairs(documentation) do
                if not categories[func_info.category] then
                    categories[func_info.category] = true
                end
            end
            
            local sorted_categories = {}
            for category, _ in pairs(categories) do
                table.insert(sorted_categories, category)
            end
            table.sort(sorted_categories)
            
            print("Available categories:")
            for _, category in ipairs(sorted_categories) do
                print(string.format("  %s", category))
            end
            
            print("")
            print("Available functions:")
            local function_names = {}
            for name, _ in pairs(documentation) do
                table.insert(function_names, name)
            end
            table.sort(function_names)
            
            -- Show functions in columns
            local cols = 3
            local rows = math.ceil(#function_names / cols)
            
            for row = 1, rows do
                local line = ""
                for col = 1, cols do
                    local idx = (col - 1) * rows + row
                    if idx <= #function_names then
                        line = line .. string.format("%-20s", function_names[idx])
                    end
                end
                print(line)
            end
        end
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

-- Record processors array - callbacks that process each new record
-- Each callback receives the current record (with builtin attributes already set)
-- and returns a table with new attributes to add/update/remove
-- Use nil values to remove attributes
record_processors = {}

-- Process each record with a custom function
-- The function receives the record and can return a table to update the record
-- To remove an attribute, use the special value "__REMOVE__" instead of nil
function for_each_record(processor_func)
    if type(processor_func) ~= "function" then
        print("Error: for_each_record requires a function parameter")
        return
    end
    
    local total_records = get_record_count()
    if total_records == 0 then
        print("No records to process")
        return
    end
    
    local processed = 0
    local updated = 0
    
    for i = 0, total_records - 1 do
        local record = get_record(i)
        if record then
            processed = processed + 1
            
            -- Call the processor function with the record
            local result = processor_func(record)
            
            -- If the function returns a table, update the record
            if type(result) == "table" then
                for key, value in pairs(result) do
                    if value == "__REMOVE__" then
                        -- Remove attribute if value is "__REMOVE__"
                        update_record_attribute(i + 1, key, nil)  -- Convert to 1-based for update_record_attribute
                    else
                        -- Add or update attribute
                        update_record_attribute(i + 1, key, tostring(value))  -- Convert to 1-based for update_record_attribute
                    end
                end
                updated = updated + 1
            end
        else
            print("Warning: Could not get record " .. i .. " of " .. total_records)
        end
    end
    
    print(string.format("Processed %d records, updated %d records", processed, updated))
end

print("TailTales Lua environment initialized. Use dir() to explore or help() for assistance.")
