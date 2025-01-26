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
            original: line,
            data: HashMap::new(),
            index: 0,
        }
    }

    pub fn set_data(&mut self, key: &str, value: String) {
        self.data.insert(key.to_string(), value);
    }
    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
    pub fn unset_data(&mut self, key: &str) {
        self.data.remove(key);
    }

    pub fn set_line_number(&mut self, line_number: usize) {
        self.index = line_number;
    }

    pub fn parse(&mut self, parsers: &Vec<Parser>) {
        let data = Record::parse_line(&self.original, parsers);
        self.data.extend(data);
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
