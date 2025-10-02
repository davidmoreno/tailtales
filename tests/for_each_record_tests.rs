use tailtales::lua_engine::LuaEngine;
use tailtales::state::TuiState;

#[test]
fn test_for_each_record_function() {
    let mut lua_engine = LuaEngine::new().unwrap();
    let mut state = TuiState::new().unwrap();

    // Add some test records
    state.records.add_record(
        tailtales::record::Record::new("This is an error message".to_string()),
        None,
    );
    state.records.add_record(
        tailtales::record::Record::new("This is a normal message".to_string()),
        None,
    );
    state.records.add_record(
        tailtales::record::Record::new("Another error occurred".to_string()),
        None,
    );

    // Test the for_each_record function
    let script = r#"
        for_each_record(function(record)
            if record.original and string.find(string.lower(record.original), "error") then
                return {mark = "red white"}
            end
            return nil
        end)
    "#;

    let result = lua_engine.execute_script_string_with_state(script, &mut state);
    assert!(result.is_ok());

    // Check that error records were marked
    let record1 = state.records.get(0).unwrap();
    assert_eq!(record1.get("mark"), Some(&"red white".to_string()));

    let record2 = state.records.get(1).unwrap();
    assert_eq!(record2.get("mark"), None); // No error in this record

    let record3 = state.records.get(2).unwrap();
    assert_eq!(record3.get("mark"), Some(&"red white".to_string()));
}

#[test]
fn test_for_each_record_with_nil_values() {
    let mut lua_engine = LuaEngine::new().unwrap();
    let mut state = TuiState::new().unwrap();

    // Add a test record with an attribute
    let mut record = tailtales::record::Record::new("Test message".to_string());
    record.set_data("debug_info", "some debug data".to_string());
    state.records.add_record(record, None);

    // Test removing an attribute by setting it to "__REMOVE__"
    let script = r#"
        for_each_record(function(record)
            return {debug_info = "__REMOVE__"}
        end)
    "#;

    let result = lua_engine.execute_script_string_with_state(script, &mut state);
    assert!(result.is_ok());

    // Check that the attribute was removed
    let record = state.records.get(0).unwrap();
    assert_eq!(record.get("debug_info"), None);
}

#[test]
fn test_for_each_record_error_handling() {
    let mut lua_engine = LuaEngine::new().unwrap();
    let mut state = TuiState::new().unwrap();

    // Test with invalid parameter (not a function)
    let script = r#"
        for_each_record("not a function")
    "#;

    let result = lua_engine.execute_script_string_with_state(script, &mut state);
    assert!(result.is_ok()); // Should not crash, just print error

    // Test with no records
    let script = r#"
        for_each_record(function(record)
            return {test = "value"}
        end)
    "#;

    let result = lua_engine.execute_script_string_with_state(script, &mut state);
    assert!(result.is_ok()); // Should handle empty records gracefully
}
