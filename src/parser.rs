use chrono::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Parser {
    regex: regex::Regex,
    is_logfmt: bool,
    is_autodate: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ParserError {
    InvalidParser(String),
}

impl Parser {
    pub fn parse(s: &str) -> Result<Parser, ParserError> {
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
        Parser {
            regex: re,
            is_logfmt: false,
            is_autodate: false,
        }
    }

    pub fn new_logfmt() -> Parser {
        let re = regex::Regex::new("(?P<key>[^ ]*?)=(?P<value>\".*?\"|[^ ]*)( |$)").unwrap();
        Parser {
            regex: re,
            is_logfmt: true,
            is_autodate: false,
        }
    }

    pub fn new_from_regex(regex: &str) -> Parser {
        let re = regex::Regex::new(regex).unwrap();
        Parser {
            regex: re,
            is_logfmt: false,
            is_autodate: false,
        }
    }

    pub fn new_autodate() -> Parser {
        let re = regex::Regex::new(".*").unwrap();
        Parser {
            regex: re,
            is_logfmt: false,
            is_autodate: true,
        }
    }

    pub fn parse_line(&self, data: HashMap<String, String>, line: &str) -> HashMap<String, String> {
        if self.is_logfmt {
            return self.parse_logfmt(data, line);
        } else if self.is_autodate {
            return self.parse_autodate(data, line);
        } else {
            return self.parse_regex(data, line);
        }
    }

    fn parse_logfmt(
        &self,
        mut _data: HashMap<String, String>,
        line: &str,
    ) -> HashMap<String, String> {
        let mut data = HashMap::new();
        for caps in self.regex.captures_iter(line) {
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
        let mut data = HashMap::new();
        let caps = self.regex.captures(line);
        match caps {
            Some(caps) => {
                for name in self.regex.capture_names().flatten() {
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
}

fn is_special_for_re(c: char) -> bool {
    match c {
        '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => true,
        _ => false,
    }
}
