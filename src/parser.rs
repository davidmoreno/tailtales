use chrono::prelude::*;
use chrono::NaiveDateTime;
use std::{collections::HashMap, sync::RwLock};

#[derive(Debug)]
pub struct CsvParser {
    pub headers: Vec<String>,
    separator: char,
}

#[derive(Debug)]
pub enum Parser {
    Regex(regex::Regex),
    LogFmt(regex::Regex),
    AutoDatetime,
    Csv(Box<RwLock<CsvParser>>),
    TransformTimestampIso8601,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ParserError {
    InvalidParser(String),
}

impl Parser {
    pub fn new(s: &str) -> Result<Parser, ParserError> {
        // split frist word of and the rest as a string
        let mut parts = s.splitn(2, ' ');
        let first = parts.next().ok_or(ParserError::InvalidParser(s.into()))?;
        match first {
            "logfmt" => return Ok(Parser::new_logfmt()),
            "regex" => {
                let rest = parts.next().ok_or(ParserError::InvalidParser(s.into()))?;
                return Ok(Parser::new_from_regex(rest));
            }
            "pattern" => {
                let rest = parts.next().ok_or(ParserError::InvalidParser(s.into()))?;
                return Ok(Parser::new_from_pattern(rest));
            }
            "autodatetime" => {
                return Ok(Parser::new_autodate());
            }
            "csv" => {
                return Ok(Parser::new_csv());
            }
            "transform" => {
                let rest = parts.next().ok_or(ParserError::InvalidParser(s.into()))?;
                return Parser::new_transform(rest);
            }
            _ => Err(ParserError::InvalidParser(s.into())),
        }
    }

    pub fn new_from_pattern(linepattern: &str) -> Parser {
        // The linepatter lang is <_> equals .*
        // <name> is a named group "name"
        // Any other thing is to be matched exactly, including spaces, text and symbols.

        let mut repattern = "^".to_string();
        let mut inpattern: bool = false;
        let mut patternname = String::new();
        for c in linepattern.chars() {
            if c == '<' {
                inpattern = true;
                patternname.clear();
                continue;
            }
            if c == '>' {
                inpattern = false;
                if patternname != "_" && patternname != "" {
                    repattern.push_str("(?P<");
                    repattern.push_str(&patternname);
                    repattern.push_str(">");
                    repattern.push_str(".*?");
                    repattern.push_str(")");
                } else {
                    repattern.push_str("(");
                    repattern.push_str(".*?");
                    repattern.push_str(")");
                }
                continue;
            }
            if inpattern {
                if is_special_for_re(c) {
                    repattern.push('\\');
                }
                patternname.push(c);
            } else {
                if is_special_for_re(c) {
                    repattern.push('\\');
                }
                repattern.push(c);
            }
        }
        repattern.push_str("$");
        let re = regex::Regex::new(&repattern).unwrap();
        Parser::Regex(re)
    }

    pub fn new_logfmt() -> Parser {
        let re = regex::Regex::new("(?P<key>[^ ]*?)=(?P<value>\".*?\"|[^ ]*)( |$)").unwrap();
        Parser::LogFmt(re)
    }

    pub fn new_from_regex(regex: &str) -> Parser {
        let re = regex::Regex::new(regex).unwrap();
        Parser::Regex(re)
    }

    pub fn new_autodate() -> Parser {
        Parser::AutoDatetime
    }

    pub fn new_csv() -> Parser {
        Parser::Csv(Box::new(RwLock::new(CsvParser {
            headers: Vec::new(),
            separator: '?',
        })))
    }

    pub fn new_transform(transform_type: &str) -> Result<Parser, ParserError> {
        match transform_type {
            "timestamp iso8601" => Ok(Parser::TransformTimestampIso8601),
            "timestamp rfc3339" => Ok(Parser::TransformTimestampIso8601), // RFC3339 is the same as ISO8601
            _ => Err(ParserError::InvalidParser(format!(
                "transform {}",
                transform_type
            ))),
        }
    }

    pub fn parse_line(&self, data: HashMap<String, String>, line: &str) -> HashMap<String, String> {
        match self {
            Parser::Regex(_) => self.parse_regex(data, line),
            Parser::AutoDatetime => self.parse_autodate(data, line),
            Parser::Csv(_) => self.parse_csv(data, line),
            Parser::LogFmt(_) => self.parse_logfmt(data, line),
            Parser::TransformTimestampIso8601 => self.parse_transform_timestamp_iso8601(data, line),
        }
    }

    fn parse_logfmt(
        &self,
        mut _data: HashMap<String, String>,
        line: &str,
    ) -> HashMap<String, String> {
        let re: &regex::Regex = match self {
            Parser::LogFmt(re) => re,
            _ => panic!("Invalid parser type"),
        };
        let mut data = HashMap::new();
        for caps in re.captures_iter(line) {
            let key = caps["key"].to_string();
            let value = caps["value"].to_string();

            if value.starts_with('"') && value.ends_with('"') {
                let end = value.len() - 1;
                let value = if end > 1 {
                    value[1..end].to_string()
                } else {
                    "".to_string()
                };
                data.insert(key, value);
                continue;
            }

            data.insert(key, value);
        }
        data
    }

    fn parse_regex(
        &self,
        mut _data: HashMap<String, String>,
        line: &str,
    ) -> HashMap<String, String> {
        let re: &regex::Regex = match self {
            Parser::Regex(re) => re,
            _ => panic!("Invalid parser type"),
        };
        let mut data = HashMap::new();
        let caps = re.captures(line);
        match caps {
            Some(caps) => {
                for name in re.capture_names().flatten() {
                    if name.starts_with("_") {
                        continue; // ignore
                    }
                    let value = caps[name].to_string();
                    data.insert(name.to_string(), value);
                }
            }
            None => {}
        }
        data
    }
    fn parse_autodate(
        &self,
        mut data: HashMap<String, String>,
        _line: &str,
    ) -> HashMap<String, String> {
        if data.contains_key("timestamp") {
            return data;
        }
        let now = Utc::now().to_rfc3339();
        data.insert("timestamp".to_string(), now);
        data
    }

    fn parse_csv(&self, mut data: HashMap<String, String>, line: &str) -> HashMap<String, String> {
        let parser = match self {
            Parser::Csv(parser) => parser,
            _ => panic!("Invalid parser type"),
        };
        let parts = self.csv_read_data(parser, line);

        if parser.read().unwrap().headers.len() == 0 {
            // first parse is header
            let headers = &mut parser.write().unwrap().headers;
            headers.extend(parts.into_iter().map(|part| part.to_string()));
            return data;
        }

        // next use the header name to map the value
        let headers = &parser.read().unwrap().headers;
        for (i, part) in parts.into_iter().enumerate() {
            let key = match headers.get(i) {
                Some(header) => header.clone(),
                None => format!("header_{}", i),
            };
            data.insert(key, part.to_string());
        }
        data
    }
    fn csv_read_data(&self, parser: &RwLock<CsvParser>, line: &str) -> Vec<String> {
        let mut delimiter = parser.read().unwrap().separator;
        if delimiter == '?' {
            // guess from line
            delimiter = line.chars().find(|c| *c == ',' || *c == ';').unwrap_or(',');
            parser.write().unwrap().separator = delimiter;
        }

        let mut parts = Vec::new();
        let mut in_quotes = false;
        let mut next_is_escaped = false;
        let mut current = String::new();
        for char in line.chars() {
            if char == '"' && !next_is_escaped {
                in_quotes = !in_quotes;
            } else if char == delimiter && !in_quotes && !next_is_escaped {
                parts.push(current);
                current = String::new();
            } else if char == '\\' && !in_quotes && !next_is_escaped {
                next_is_escaped = true;
            } else {
                if next_is_escaped {
                    next_is_escaped = false;
                }
                current.push(char);
            }
        }
        parts.push(current);
        parts
    }

    fn parse_transform_timestamp_iso8601(
        &self,
        mut data: HashMap<String, String>,
        _line: &str,
    ) -> HashMap<String, String> {
        // Look for timestamp field in the data
        if let Some(timestamp) = data.get("timestamp") {
            // Try to parse various timestamp formats and convert to ISO8601
            if let Some(iso8601_timestamp) = self.convert_to_iso8601(timestamp) {
                data.insert("timestamp".to_string(), iso8601_timestamp);
            }
        }
        data
    }

    fn convert_to_iso8601(&self, timestamp: &str) -> Option<String> {
        // Try various timestamp formats and convert to ISO8601

        // Format: "2024-01-01T12:30:45Z" (with Z) - already RFC3339
        if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
            return Some(dt.to_rfc3339());
        }

        // Format: "2024-01-01T12:30:45.123Z" (with milliseconds) - already RFC3339
        if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
            return Some(dt.to_rfc3339());
        }

        // Format: "02/Jan/2024:12:30:45 +0100" (nginx/apache format)
        if let Ok(dt) = DateTime::parse_from_str(timestamp, "%d/%b/%Y:%H:%M:%S %z") {
            return Some(dt.to_rfc3339());
        }

        // Format: "2024-01-01 12:30:45" (space separated) - assume UTC
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S") {
            let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc);
            return Some(utc_dt.to_rfc3339());
        }

        // Format: "2024-01-01T12:30:45" (T separated) - assume UTC
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S") {
            let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc);
            return Some(utc_dt.to_rfc3339());
        }

        // Format: "Jan 02 12:30:45" (syslog format) - assume UTC and current year
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(timestamp, "%b %d %H:%M:%S") {
            let now = Utc::now();
            let dt_with_year = naive_dt.with_year(now.year()).unwrap_or(naive_dt);
            let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(dt_with_year, Utc);
            return Some(utc_dt.to_rfc3339());
        }

        // If we can't parse it, return None (keep original)
        None
    }
}

fn is_special_for_re(c: char) -> bool {
    match c {
        '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_is_special_for_re() {
        assert!(is_special_for_re('.'));
        assert!(is_special_for_re('*'));
        assert!(is_special_for_re('+'));
        assert!(is_special_for_re('?'));
        assert!(is_special_for_re('('));
        assert!(is_special_for_re(')'));
        assert!(is_special_for_re('['));
        assert!(is_special_for_re(']'));
        assert!(is_special_for_re('{'));
        assert!(is_special_for_re('}'));
        assert!(is_special_for_re('^'));
        assert!(is_special_for_re('$'));
        assert!(is_special_for_re('|'));
        assert!(is_special_for_re('\\'));
        assert!(!is_special_for_re('a'));
        assert!(!is_special_for_re('1'));
        assert!(!is_special_for_re(' '));
    }

    #[test]
    fn test_parser_new_valid_cases() {
        // Test logfmt
        let parser = Parser::new("logfmt").unwrap();
        matches!(parser, Parser::LogFmt(_));

        // Test regex
        let parser = Parser::new("regex .*").unwrap();
        matches!(parser, Parser::Regex(_));

        // Test pattern
        let parser = Parser::new("pattern <name> test").unwrap();
        matches!(parser, Parser::Regex(_));

        // Test autodatetime
        let parser = Parser::new("autodatetime").unwrap();
        matches!(parser, Parser::AutoDatetime);

        // Test csv
        let parser = Parser::new("csv").unwrap();
        matches!(parser, Parser::Csv(_));
    }

    #[test]
    fn test_parser_new_invalid_cases() {
        // Test invalid parser type
        let result = Parser::new("invalid");
        assert!(result.is_err());
        matches!(result.unwrap_err(), ParserError::InvalidParser(_));

        // Test regex without pattern
        let result = Parser::new("regex");
        assert!(result.is_err());

        // Test pattern without pattern
        let result = Parser::new("pattern");
        assert!(result.is_err());
    }

    #[test]
    fn test_new_from_pattern() {
        // Test simple pattern with named group
        let parser = Parser::new_from_pattern("Hello <name> world");
        if let Parser::Regex(re) = parser {
            let test_line = "Hello John world";
            let caps = re.captures(test_line).unwrap();
            assert_eq!(&caps["name"], "John");
        } else {
            panic!("Expected Regex parser");
        }

        // Test pattern with unnamed group
        let parser = Parser::new_from_pattern("Hello <_> world");
        if let Parser::Regex(re) = parser {
            let test_line = "Hello anyone world";
            assert!(re.is_match(test_line));
        }

        // Test pattern with special regex characters
        let parser = Parser::new_from_pattern("Test [brackets] and (parens)");
        if let Parser::Regex(re) = parser {
            let test_line = "Test [brackets] and (parens)";
            assert!(re.is_match(test_line));
        }
    }

    #[test]
    fn test_parse_logfmt() {
        let parser = Parser::new_logfmt();
        let data = HashMap::new();

        // Test simple key-value pairs
        let result = parser.parse_line(data.clone(), "key1=value1 key2=value2");
        assert_eq!(result.get("key1"), Some(&"value1".to_string()));
        assert_eq!(result.get("key2"), Some(&"value2".to_string()));

        // Test quoted values
        let result = parser.parse_line(data.clone(), "key1=\"quoted value\" key2=unquoted");
        assert_eq!(result.get("key1"), Some(&"quoted value".to_string()));
        assert_eq!(result.get("key2"), Some(&"unquoted".to_string()));

        // Test empty quoted value
        let result = parser.parse_line(data, "key1=\"\"");
        assert_eq!(result.get("key1"), Some(&"".to_string()));
    }

    #[test]
    fn test_parse_logfmt_complex_line() {
        let parser = Parser::new_logfmt();
        let data = HashMap::new();

        // Test complex logfmt line with multiple key-value pairs and special characters
        let complex_line = "level=INFO module=test function=login filename=views.py | level=4 allowed=True user_id=568 mod=users description=\"login /bombards.parapsychology@serial.com/\"";
        let result = parser.parse_line(data, complex_line);

        // The logfmt parser correctly extracts key-value pairs separated by spaces
        // It treats the pipe character as a separator between key-value pairs
        // The parser finds: level=INFO, module=test, function=login, filename=views.py, |, level=4, allowed=True, etc.
        // Note: The pipe character "|" is treated as a standalone key with no value

        // Verify the key-value pairs that are actually parsed
        assert_eq!(result.get("level"), Some(&"4".to_string())); // Second level=4 overwrites first level=INFO
        assert_eq!(result.get("module"), Some(&"test".to_string()));
        assert_eq!(result.get("function"), Some(&"login".to_string()));
        assert_eq!(result.get("filename"), Some(&"views.py".to_string()));
        assert_eq!(result.get("allowed"), Some(&"True".to_string()));
        assert_eq!(result.get("user_id"), Some(&"568".to_string()));
        assert_eq!(result.get("mod"), Some(&"users".to_string()));
        assert_eq!(
            result.get("description"),
            Some(&"login /bombards.parapsychology@serial.com/".to_string())
        );

        // Test that the parser correctly handles the complex line structure
        // The parser successfully extracts all the key-value pairs from the complex logfmt line
        // This demonstrates that the logfmt parser can handle lines with special characters
        // and multiple key-value pairs, including cases where keys are duplicated (last value wins)
    }

    #[test]
    fn test_parse_regex() {
        let parser = Parser::new_from_regex(r"(?P<method>\w+) (?P<path>/\S+)");
        let data = HashMap::new();

        let result = parser.parse_line(data, "GET /api/users");
        assert_eq!(result.get("method"), Some(&"GET".to_string()));
        assert_eq!(result.get("path"), Some(&"/api/users".to_string()));

        // Test line that doesn't match
        let result = parser.parse_line(HashMap::new(), "invalid line");
        assert!(result.is_empty());

        // Test with underscore prefixed group (should be ignored)
        let parser = Parser::new_from_regex(r"(?P<_ignore>\w+) (?P<keep>\w+)");
        let result = parser.parse_line(HashMap::new(), "ignore keep");
        assert!(!result.contains_key("_ignore"));
        assert_eq!(result.get("keep"), Some(&"keep".to_string()));
    }

    #[test]
    fn test_parse_autodate() {
        let parser = Parser::new_autodate();
        let mut data = HashMap::new();

        // Test adding timestamp when not present
        let result = parser.parse_line(data.clone(), "test line");
        assert!(result.contains_key("timestamp"));

        // Test preserving existing timestamp
        data.insert("timestamp".to_string(), "existing".to_string());
        let result = parser.parse_line(data, "test line");
        assert_eq!(result.get("timestamp"), Some(&"existing".to_string()));
    }

    #[test]
    fn test_parse_csv() {
        let parser = Parser::new_csv();
        let data = HashMap::new();

        // First line should be treated as headers
        let result = parser.parse_line(data.clone(), "name,age,city");
        assert!(result.is_empty()); // Headers don't produce data

        // Second line should use headers as keys
        let result = parser.parse_line(data.clone(), "John,25,NYC");
        assert_eq!(result.get("name"), Some(&"John".to_string()));
        assert_eq!(result.get("age"), Some(&"25".to_string()));
        assert_eq!(result.get("city"), Some(&"NYC".to_string()));

        // Test with semicolon separator
        let parser2 = Parser::new_csv();
        let _result = parser2.parse_line(HashMap::new(), "name;age;city");
        let result = parser2.parse_line(HashMap::new(), "Jane;30;LA");
        assert_eq!(result.get("name"), Some(&"Jane".to_string()));
        assert_eq!(result.get("age"), Some(&"30".to_string()));
        assert_eq!(result.get("city"), Some(&"LA".to_string()));
    }

    #[test]
    fn test_csv_read_data() {
        let parser = Parser::new_csv();
        if let Parser::Csv(csv_parser) = &parser {
            // Test comma separated values
            let result = parser.csv_read_data(csv_parser, "a,b,c");
            assert_eq!(result, vec!["a", "b", "c"]);

            // Test quoted values
            let result = parser.csv_read_data(csv_parser, "\"quoted,value\",normal");
            assert_eq!(result, vec!["quoted,value", "normal"]);

            // Test escaped characters
            let result = parser.csv_read_data(csv_parser, "value1,value\\,with\\,commas,value3");
            assert_eq!(result, vec!["value1", "value,with,commas", "value3"]);

            // Test empty fields
            let result = parser.csv_read_data(csv_parser, "a,,c");
            assert_eq!(result, vec!["a", "", "c"]);
        }
    }

    #[test]
    fn test_csv_with_quoted_fields() {
        let parser = Parser::new_csv();
        let data = HashMap::new();

        // Set up headers
        let _result = parser.parse_line(data.clone(), "name,description");

        // Test quoted field with comma
        let result = parser.parse_line(data, "John,\"Software Engineer, Senior\"");
        assert_eq!(result.get("name"), Some(&"John".to_string()));
        assert_eq!(
            result.get("description"),
            Some(&"Software Engineer, Senior".to_string())
        );
    }

    #[test]
    fn test_parse_line_routing() {
        // Test that parse_line correctly routes to the right parser
        let regex_parser = Parser::new_from_regex(r"(?P<word>\w+)");
        let result = regex_parser.parse_line(HashMap::new(), "hello");
        assert!(result.contains_key("word"));

        let autodate_parser = Parser::new_autodate();
        let result = autodate_parser.parse_line(HashMap::new(), "test");
        assert!(result.contains_key("timestamp"));

        let logfmt_parser = Parser::new_logfmt();
        let result = logfmt_parser.parse_line(HashMap::new(), "key=value");
        assert!(result.contains_key("key"));

        let csv_parser = Parser::new_csv();
        let _result = csv_parser.parse_line(HashMap::new(), "header1,header2");
        let result = csv_parser.parse_line(HashMap::new(), "value1,value2");
        assert!(result.contains_key("header1"));
    }

    #[test]
    fn test_parser_new_with_transform() {
        // Test that "transform timestamp iso8601" now works correctly
        let result = Parser::new("transform timestamp iso8601");
        assert!(result.is_ok());
        match result.unwrap() {
            Parser::TransformTimestampIso8601 => {}
            _ => panic!("Expected TransformTimestampIso8601 parser"),
        }
    }

    #[test]
    fn test_parser_new_with_empty_string() {
        // Test that empty string produces InvalidParser error
        let result = Parser::new("");
        assert!(result.is_err());
        match result.unwrap_err() {
            ParserError::InvalidParser(msg) => {
                assert_eq!(msg, "");
            }
        }
    }

    #[test]
    fn test_parser_new_with_unknown_types() {
        // Test various unknown parser types that should fail
        let unknown_types = vec![
            "transform",
            "transform unknown_format",
            "json",
            "xml",
            "yaml",
            "unknown_parser_type",
        ];

        for parser_type in unknown_types {
            let result = Parser::new(parser_type);
            assert!(result.is_err(), "Parser type '{}' should fail", parser_type);
            match result.unwrap_err() {
                ParserError::InvalidParser(msg) => {
                    assert_eq!(msg, parser_type);
                }
            }
        }
    }

    #[test]
    fn test_transform_timestamp_iso8601_conversion() {
        let parser = Parser::new("transform timestamp iso8601").unwrap();

        // Test space-separated format
        let mut data = HashMap::new();
        data.insert("timestamp".to_string(), "2024-01-01 12:30:45".to_string());
        let result = parser.parse_line(data.clone(), "test line");
        let timestamp = result.get("timestamp").unwrap();
        assert!(timestamp.contains("2024-01-01T12:30:45"));
        assert!(timestamp.contains("+00:00") || timestamp.contains("Z"));

        // Test T-separated format
        data.insert("timestamp".to_string(), "2024-01-01T12:30:45".to_string());
        let result = parser.parse_line(data.clone(), "test line");
        let timestamp = result.get("timestamp").unwrap();
        assert!(timestamp.contains("2024-01-01T12:30:45"));
        assert!(timestamp.contains("+00:00") || timestamp.contains("Z"));

        // Test RFC3339 format (should remain unchanged)
        data.insert("timestamp".to_string(), "2024-01-01T12:30:45Z".to_string());
        let result = parser.parse_line(data.clone(), "test line");
        let timestamp = result.get("timestamp").unwrap();
        assert!(timestamp.contains("2024-01-01T12:30:45"));
        assert!(timestamp.contains("Z") || timestamp.contains("+00:00"));

        // Test nginx/apache format
        data.insert(
            "timestamp".to_string(),
            "02/Jan/2024:12:30:45 +0100".to_string(),
        );
        let result = parser.parse_line(data.clone(), "test line");
        let timestamp = result.get("timestamp").unwrap();
        assert!(timestamp.contains("2024-01-02T12:30:45"));
        assert!(timestamp.contains("+01:00"));

        // Test syslog format
        data.insert("timestamp".to_string(), "Jan 02 12:30:45".to_string());
        let result = parser.parse_line(data.clone(), "test line");
        let timestamp = result.get("timestamp").unwrap();
        assert!(timestamp.contains("12:30:45"));

        // Test with no timestamp field (should not change data)
        let mut data = HashMap::new();
        data.insert("other_field".to_string(), "value".to_string());
        let result = parser.parse_line(data.clone(), "test line");
        assert_eq!(result.get("other_field"), Some(&"value".to_string()));
        assert!(result.get("timestamp").is_none());

        // Test with unparseable timestamp (should keep original)
        data.insert("timestamp".to_string(), "invalid timestamp".to_string());
        let result = parser.parse_line(data.clone(), "test line");
        assert_eq!(
            result.get("timestamp"),
            Some(&"invalid timestamp".to_string())
        );
    }

    #[test]
    fn test_transform_parser_routing() {
        // Test that transform parser correctly routes to the right method
        let transform_parser = Parser::new("transform timestamp iso8601").unwrap();
        let mut data = HashMap::new();
        data.insert("timestamp".to_string(), "2024-01-01 12:30:45".to_string());

        let result = transform_parser.parse_line(data, "test line");
        let timestamp = result.get("timestamp").unwrap();
        assert!(timestamp.contains("2024-01-01T12:30:45"));
        assert!(timestamp.contains("+00:00") || timestamp.contains("Z"));
    }
}
