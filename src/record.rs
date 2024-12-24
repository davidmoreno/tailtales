use regex::Regex;
use std::{
    collections::HashMap,
    io::BufRead,
    thread::{self},
};

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

    pub fn readfile_parallel(&mut self, filename: &str) {
        let file = std::fs::File::open(filename).expect("could not open file");
        let reader = std::io::BufReader::new(file);
        let mut line_number = 0;
        let mut threads = Vec::new();

        // We prepare a vector of 100 lines per thread, and send the work, as result we get the records
        // and we add them to the records list. Continue until no more lines are available.

        let mut lines: Vec<String> = Vec::new();
        let mut thread_block = 0;
        for line in reader.lines() {
            line_number += 1;
            let line = line.expect("could not read line");
            lines.push(line);
            if lines.len() >= 300 {
                thread_block += 1;
                let mylines = lines.clone();
                let filename = filename.to_string();
                let start_line_number = line_number;
                let thread = thread::spawn(move || {
                    let mut records = Vec::new();
                    for (line_number, line) in mylines.iter().enumerate() {
                        let mut rec =
                            Record::new(line.clone(), &filename, line_number + start_line_number);
                        rec.data
                            .insert("thread_block".to_string(), thread_block.to_string());
                        records.push(rec);
                    }
                    records
                });
                threads.push(thread);
                lines.clear();
            }
        }
        if lines.len() > 0 {
            thread_block += 1;
            let mylines = lines.clone();
            let filename = filename.to_string();
            let start_line_number = line_number;
            let thread = thread::spawn(move || {
                let mut records = Vec::new();
                for (line_number, line) in mylines.iter().enumerate() {
                    let mut rec =
                        Record::new(line.clone(), &filename, line_number + start_line_number);
                    rec.data
                        .insert("thread_block".to_string(), thread_block.to_string());
                    records.push(rec);
                }
                records
            });
            threads.push(thread);
            lines.clear();
        }

        for thread in threads {
            let records = thread.join().unwrap();
            self.records.extend(records);
        }
    }

    pub fn add(&mut self, record: Record) {
        self.records.push(record);
    }
}
