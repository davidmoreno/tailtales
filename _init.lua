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
        print("Available topics: 'functions', 'navigation', 'records'")
        print("")
        print("Core functions:")
        print("  get_position()  - Get current line position")
        print("  get_record()    - Get current record data")
        print("  quit()          - Exit the application")
        print("  warning(msg)    - Show warning message")
        print("  print(...)      - Output text to REPL")
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
        print("  record.line     - Raw log line")
        print("  record.timestamp- Parsed timestamp (if available)")
        print("  record.*        - Other parsed fields")
    else
        print("Unknown help topic: " .. tostring(topic))
        print("Available topics: 'functions', 'navigation', 'records'")
    end
end

print("TailTales Lua environment initialized. Use dir() to explore or help() for assistance.")
