use rayon::prelude::*;
use regex::Regex;
use std::{collections::HashMap, io::BufRead};

use crate::parser::Parser;

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

    pub fn set_data(mut self, key: &str, value: String) -> Self {
        self.data.insert(key.to_string(), value);
        self
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

    pub fn matches(&self, search: &str) -> bool {
        self.original
            .to_lowercase()
            .contains(search.to_lowercase().as_str())
    }
}

lazy_static::lazy_static! {
    static ref TIMESTAMP_RE: Regex = Regex::new(r"\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}").unwrap();
}

fn find_timestamp(line: &str) -> Option<String> {
    let caps = TIMESTAMP_RE.captures(line);
    match caps {
        Some(caps) => Some(caps.get(0).unwrap().as_str().to_string()),
        None => None,
    }
}

#[derive(Debug, Default, Clone)]
pub struct RecordList {
    pub records: Vec<Record>,
    pub parsers: Vec<Parser>,
}

impl RecordList {
    pub fn new() -> RecordList {
        RecordList {
            records: Vec::new(),
            parsers: vec![],
        }
    }

    // pub fn readfile(&mut self, filename: &str) {
    //     let file = std::fs::File::open(filename).expect("could not open file");
    //     let reader = std::io::BufReader::new(file);
    //     let mut line_number = 0;
    //     for line in reader.lines() {
    //         line_number += 1;
    //         let line = line.expect("could not read line");
    //         self.add(Record::new(line, filename, line_number, &self.parsers));
    //     }
    // }

    pub fn readfile_parallel(&mut self, filename: &str) {
        let file = std::fs::File::open(filename).expect("could not open file");
        let reader = std::io::BufReader::new(file);

        let lines: Vec<String> = reader.lines().map(|line| line.unwrap()).collect();
        let start_line_number = self.records.len();
        let records: Vec<Record> = lines
            .par_iter()
            .enumerate()
            .map(|(line_number, line)| {
                Record::new(line.clone())
                    .set_data("filename", filename.to_string())
                    .set_data("line_number", (line_number).to_string())
                    .set_line_number(start_line_number + line_number)
                    .parse(&self.parsers)
            })
            .collect();

        self.records.extend(records);
    }

    pub fn readfile_stdin(&mut self) {
        let reader = std::io::stdin();
        let reader = reader.lock();
        let mut line_number = 0;
        let line_number_index = self.records.len();
        for line in reader.lines() {
            let line = line.expect("could not read line");
            self.add(
                Record::new(line)
                    .set_data("line_number", line_number.to_string())
                    .set_line_number(line_number + line_number_index)
                    .parse(&self.parsers),
            );
            line_number += 1;
        }
    }

    pub fn add(&mut self, record: Record) {
        self.records.push(record);
    }

    pub fn filter(&self, search: &str) -> RecordList {
        let mut result = vec![];
        for record in &self.records {
            if record.matches(search) {
                result.push((*record).clone());
            }
        }
        RecordList {
            records: result,
            parsers: self.parsers.clone(),
        }
    }

    pub fn filter_parallel(&self, search: &str) -> RecordList {
        let result: Vec<Record> = self
            .records
            .par_iter()
            .filter(|record| record.matches(search))
            .map(|record| (*record).clone())
            .collect();
        RecordList {
            records: result,
            parsers: self.parsers.clone(),
        }
    }

    /// Search for a string in the records, returns the position of the next match.
    pub fn search_forward(&mut self, search: &str, start_at: usize) -> Option<usize> {
        for (i, record) in self.records.iter().enumerate().skip(start_at) {
            if record.matches(search) {
                return Some(i);
            }
        }
        None
    }

    pub fn search_backwards(&mut self, search: &str, start_at: usize) -> Option<usize> {
        let rstart_at = if start_at == 0 {
            self.records.len()
        } else {
            start_at + 1
        };

        for pos in (0..rstart_at).rev() {
            let record = &self.records[pos];
            if record.matches(search) {
                return Some(pos);
            }
        }
        None
    }

    pub fn renumber(&mut self) {
        for (i, record) in self.records.iter_mut().enumerate() {
            record.index = i;
        }
    }
}
