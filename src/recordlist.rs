use notify::{Event, Watcher};
use rayon::{prelude::*, spawn};
use std::{
    io::{BufRead, Seek},
    path::Path,
    sync::mpsc,
};

use crate::{events::TuiEvent, parser::Parser, record::Record};

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

    pub fn readfile_parallel(&mut self, filename: &str, tx: mpsc::Sender<TuiEvent>) {
        let file = std::fs::File::open(filename).expect("could not open file");
        let mut reader = std::io::BufReader::new(file);
        let file_size = reader.seek(std::io::SeekFrom::End(0)).unwrap();
        reader.seek(std::io::SeekFrom::Start(0)).unwrap();

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

        Self::wait_for_changes(filename.to_string(), tx, file_size.try_into().unwrap());
    }

    pub fn wait_for_changes(filename: String, tx: mpsc::Sender<TuiEvent>, position: usize) {
        let tx_clone = tx.clone();
        spawn(move || {
            let mut position = position;
            let (tx, rx) = mpsc::channel();
            let mut watcher = notify::recommended_watcher(tx).unwrap();
            watcher
                .watch(Path::new(&filename), notify::RecursiveMode::NonRecursive)
                .unwrap();
            loop {
                match rx.recv() {
                    Ok(event) => match event.unwrap().kind {
                        notify::EventKind::Modify(_) => {
                            position =
                                Self::read_and_send_new_lines(&filename, &tx_clone, position);
                        }
                        _ => {}
                    },
                    Err(e) => println!("watch error: {:?}", e),
                }
            }
        });
    }

    pub fn read_and_send_new_lines(
        filename: &str,
        tx: &mpsc::Sender<TuiEvent>,
        position: usize,
    ) -> usize {
        let file = std::fs::File::open(filename).expect("could not open file");
        let mut reader = std::io::BufReader::new(file);
        let end_position = reader.seek(std::io::SeekFrom::End(0)).unwrap();
        reader
            .seek(std::io::SeekFrom::Start(position as u64))
            .unwrap();

        let start_line_number = 0;
        let mut line_number = 0;
        for line in reader.lines() {
            let line = line.expect("could not read line");
            let record = Record::new(line.clone())
                .set_data("filename", filename.to_string())
                .set_data("line_number", (line_number).to_string())
                .set_line_number(start_line_number + line_number)
                // .parse(&self.parsers)
                ;
            tx.send(TuiEvent::NewRecord(record)).unwrap();
            line_number += 1;
        }

        end_position as usize
    }

    pub fn readfile_stdin(&mut self, tx: mpsc::Sender<TuiEvent>) {
        let line_number_index = self.records.len();
        let parsers = self.parsers.clone();
        spawn(move || {
            let reader = std::io::stdin();
            let reader = reader.lock();
            let mut line_number = 0;
            for line in reader.lines() {
                let line = line.expect("could not read line");

                let record = Record::new(line)
                    .set_data("line_number", line_number.to_string())
                    .set_line_number(line_number + line_number_index)
                    .parse(&parsers);
                tx.send(TuiEvent::NewRecord(record));
                line_number += 1;
            }
        });
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
