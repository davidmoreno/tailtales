use regex::Regex;
use std::collections::HashMap;

use crate::{ast::AST, parser::Parser};

#[derive(Debug, Default, Clone)]
pub struct Record {
    pub original: String,
    pub data: HashMap<String, String>,
    pub index: usize,
}

impl Record {
    pub fn new(line: String) -> Record {
        Record {
            original: clean_ansi_text(&line),
            data: HashMap::new(),
            index: 0,
        }
    }

    pub fn set_data(mut self, key: &str, value: String) -> Self {
        self.data.insert(key.to_string(), value);
        self
    }
    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }

    pub fn set_line_number(mut self, line_number: usize) -> Self {
        self.index = line_number;
        self
    }

    pub fn parse(mut self, parsers: &Vec<Parser>) -> Self {
        let data = Record::parse_line(&self.original, parsers);
        self.data.extend(data);
        self
    }

    pub fn parse_line(line: &str, parsers: &Vec<Parser>) -> HashMap<String, String> {
        let mut data = HashMap::new();

        // Basic cound words
        let words: Vec<&str> = line.split_whitespace().collect();
        let word_count = words.len();
        data.insert("word_count".to_string(), word_count.to_string());

        for parser in parsers {
            let more_data = parser.parse_line(line);
            data.extend(more_data);
        }

        data
    }

    pub fn matches(&self, search: &AST) -> bool {
        search.matches(self)
    }
}

lazy_static::lazy_static! {
    static ref TIMESTAMP_RE: Regex = Regex::new(r"\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}").unwrap();
}

pub fn clean_ansi_text(orig: &str) -> String {
    // Lenght in real text, skips ANIS codes
    let mut text = String::new();
    let mut in_ansi_escape = false;
    for c in orig.chars() {
        if in_ansi_escape {
            if c == 'm' {
                in_ansi_escape = false;
            }
        }
        if c == '\t' {
            // I need to put as many spaces to make for a multiple of 8 spaces
            let spaces = 8 - text.len() % 8;
            for _ in 0..spaces {
                text.push(' ');
            }
        } else if c == '\r' {
            // Ignore
        } else {
            if c == 0o33 as char {
                in_ansi_escape = true;
            } else {
                text.push(c);
            }
        }
    }
    text
}
