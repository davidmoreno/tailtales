use rayon::prelude::*;
use regex::Regex;
use std::{collections::HashMap, io::BufRead};

#[derive(Debug, Default, Clone)]
pub struct Record {
    pub original: String,
    pub data: HashMap<String, String>,
    pub index: usize,
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
            index: line_number,
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

    pub fn readfile_parallel(&mut self, filename: &str) {
        let file = std::fs::File::open(filename).expect("could not open file");
        let reader = std::io::BufReader::new(file);

        let lines: Vec<String> = reader.lines().map(|line| line.unwrap()).collect();
        let records: Vec<Record> = lines
            .par_iter()
            .enumerate()
            .map(|(line_number, line)| Record::new(line.clone(), filename, line_number))
            .collect();

        self.records.extend(records);
    }

    pub fn add(&mut self, record: Record) {
        self.records.push(record);
    }

    pub fn filter(&self, search: &str) -> RecordList {
        let mut result = vec![];
        for record in &self.records {
            if RecordList::record_matches(record, search) {
                result.push((*record).clone());
            }
        }
        RecordList { records: result }
    }

    /// Search for a string in the records, returns the position of the next match.
    pub fn search_forward(&mut self, search: &str, start_at: usize) -> Option<usize> {
        for (i, record) in self.records.iter().enumerate().skip(start_at) {
            if Self::record_matches(record, search) {
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
            if Self::record_matches(record, search) {
                return Some(pos);
            }
        }
        None
    }

    pub fn record_matches(record: &Record, search: &str) -> bool {
        record
            .original
            .to_lowercase()
            .contains(search.to_lowercase().as_str())
    }

    pub fn renumber(&mut self) {
        for (i, record) in self.records.iter_mut().enumerate() {
            record.index = i;
        }
    }
}
