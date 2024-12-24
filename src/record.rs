use regex::Regex;
use std::{collections::HashMap, io::BufRead};

#[derive(Debug, Default)]
pub struct Record {
    pub original: String,
    pub data: HashMap<String, String>,
}

impl Record {
    pub fn new(line: String, filename: &str, line_number: usize) -> Record {
        let mut data = Record::parse_line(&line);

        data.insert("filename".to_string(), filename.to_string());
        data.insert("line_number".to_string(), line_number.to_string());
        let timestamp = find_timestamp(&line);
        if let Some(timestamp) = timestamp {
            data.insert("timestamp".to_string(), timestamp);
        }

        Record {
            original: line,
            data,
        }
    }

    pub fn parse_line(line: &str) -> HashMap<String, String> {
        let mut data = HashMap::new();

        // Basic cound words
        let words: Vec<&str> = line.split_whitespace().collect();
        let word_count = words.len();
        data.insert("word_count".to_string(), word_count.to_string());

        data
    }
}

fn find_timestamp(line: &str) -> Option<String> {
    let re = regex::Regex::new(r"\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}").unwrap();
    let caps = re.captures(line);
    match caps {
        Some(caps) => Some(caps.get(0).unwrap().as_str().to_string()),
        None => None,
    }
}

#[derive(Debug, Default)]
pub struct RecordList {
    pub records: Vec<Record>,
}

impl RecordList {
    pub fn new() -> RecordList {
        RecordList {
            records: Vec::new(),
        }
    }

    pub fn readfile(&mut self, filename: &str) {
        let file = std::fs::File::open(filename).expect("could not open file");
        let reader = std::io::BufReader::new(file);
        let mut line_number = 0;
        for line in reader.lines() {
            line_number += 1;
            let line = line.expect("could not read line");
            self.add(Record::new(line, filename, line_number));
        }
    }

    pub fn add(&mut self, record: Record) {
        self.records.push(record);
    }
}
