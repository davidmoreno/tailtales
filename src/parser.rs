use chrono::prelude::*;
use std::{collections::HashMap, sync::RwLock};

#[derive(Debug)]
struct CsvParser {
    headers: Vec<String>,
    separator: char,
}

#[derive(Debug)]
pub enum Parser {
    Regex(regex::Regex),
    LogFmt(regex::Regex),
    AutoDatetime,
    Csv(Box<RwLock<CsvParser>>),
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

    pub fn parse_line(&self, data: HashMap<String, String>, line: &str) -> HashMap<String, String> {
        match self {
            Parser::Regex(_) => self.parse_regex(data, line),
            Parser::AutoDatetime => self.parse_autodate(data, line),
            Parser::Csv(_) => self.parse_csv(data, line),
            Parser::LogFmt(_) => self.parse_logfmt(data, line),
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
                let value = value[1..value.len() - 1].to_string();
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
}

fn is_special_for_re(c: char) -> bool {
    match c {
        '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => true,
        _ => false,
    }
}
