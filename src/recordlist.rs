use notify::Watcher;
use rayon::{prelude::*, spawn};
use std::{
    io::{BufRead, Seek},
    path::Path,
    sync::mpsc,
};

use crate::{ast::AST, events::TuiEvent, parser::Parser, record::Record};

#[derive(Debug, Default, Clone)]
pub struct RecordList {
    pub all_records: Vec<Record>,
    pub visible_records: Vec<Record>,
    pub parsers: Vec<Parser>,
    pub filter: Option<AST>,
}

impl RecordList {
    pub fn new() -> RecordList {
        RecordList {
            all_records: Vec::new(),
            visible_records: Vec::new(),
            parsers: vec![],
            filter: None,
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
        let records: Vec<Record> = lines
            .par_iter()
            .enumerate()
            .map(|(line_number, line)| {
                Record::new(line.clone())
                    .set_data("filename", filename.to_string())
                    .set_data("line_number", line_number.to_string())
                    .parse(&self.parsers)
            })
            .collect();

        self.visible_records = records.clone();
        self.all_records.extend(records);
        self.renumber();

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

        let mut line_number = 0;
        for line in reader.lines() {
            let line = line.expect("could not read line");
            let record = Record::new(line.clone())
                .set_data("filename", filename.to_string())
                .set_line_number(line_number);
            tx.send(TuiEvent::NewRecord(record)).unwrap();
            line_number += 1;
        }

        end_position as usize
    }

    pub fn readfile_stdin(&mut self, tx: mpsc::Sender<TuiEvent>) {
        spawn(move || {
            let reader = std::io::stdin();
            let reader = reader.lock();
            for line in reader.lines() {
                let line = line.expect("could not read line");
                let record = Record::new(line);
                tx.send(TuiEvent::NewRecord(record)).unwrap();
            }
        });
    }

    pub fn add(&mut self, record: Record) {
        let record = record
            .parse(&self.parsers)
            .set_data("line_number", self.all_records.len().to_string());

        self.all_records.push(record.clone());

        if self.filter.is_none() || record.matches(&self.filter.as_ref().unwrap()) {
            let record = record.set_line_number(self.visible_records.len());
            self.visible_records.push(record);
        }
    }

    // pub fn filter(&mut self, search: AST) {
    //     let mut result = vec![];
    //     for record in &self.all_records {
    //         if record.matches(&search) {
    //             result.push((*record).clone());
    //         }
    //     }
    //     self.filter = Some(search);
    //     self.visible_records = result;
    //     self.renumber();
    // }

    pub fn filter_parallel(&mut self, search: AST) {
        let result: Vec<Record> = self
            .all_records
            .par_iter()
            .filter(|record| record.matches(&search))
            .map(|record| (*record).clone())
            .collect();
        self.filter = Some(search);
        self.visible_records = result;
        self.renumber();
    }

    /// Search for a string in the records, returns the position of the next match.
    pub fn search_forward(&mut self, search: &AST, start_at: usize) -> Option<usize> {
        for (i, record) in self.all_records.iter().enumerate().skip(start_at) {
            if record.matches(search) {
                return Some(i);
            }
        }
        None
    }

    pub fn search_backwards(&mut self, search: &AST, start_at: usize) -> Option<usize> {
        let rstart_at = if start_at == 0 {
            self.all_records.len()
        } else {
            start_at + 1
        };

        for pos in (0..rstart_at).rev() {
            let record = &self.all_records[pos];
            if record.matches(search) {
                return Some(pos);
            }
        }
        None
    }

    pub fn renumber(&mut self) {
        for (i, record) in self.visible_records.iter_mut().enumerate() {
            record.index = i;
        }
    }
}
