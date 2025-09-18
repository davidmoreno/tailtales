use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use notify::Watcher;
use rayon::{prelude::*, spawn};
use std::thread::sleep;
use std::time::Duration;
use std::{
    io::{BufRead, Read, Seek},
    path::Path,
    process::Stdio,
    sync::mpsc,
};

use crate::parser;
use crate::settings::RulesSettings;
use crate::{ast::AST, events::TuiEvent, parser::Parser, record::Record};

#[derive(Debug, Default)]
pub struct RecordList {
    pub all_records: Vec<Record>,
    pub visible_records: Vec<Record>,
    pub parsers: Vec<Parser>,
    pub filter: Option<AST>,
    pub child_process: Option<u32>,
    pub max_record_size: usize,
}

impl RecordList {
    pub fn new() -> RecordList {
        RecordList {
            all_records: Vec::new(),
            visible_records: Vec::new(),
            parsers: vec![],
            filter: None,
            child_process: None,
            max_record_size: 0,
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

    pub fn readfile_gz(&mut self, filename: &str) {
        let file = match std::fs::File::open(filename) {
            Ok(file) => file,
            Err(_error) => panic!("Could not open file={:?}", filename),
        };

        let reader = std::io::BufReader::new(file);
        let mut decoder = flate2::read::GzDecoder::new(reader);
        let mut buffer = String::new();
        decoder.read_to_string(&mut buffer).unwrap();

        let lines: Vec<String> = buffer.lines().map(|line| line.to_string()).collect();
        let records: Vec<Record> = lines
            .par_iter()
            .enumerate()
            .map(|(line_number, line)| {
                let mut record = Record::new(line.clone());
                record.set_data("filename", filename.to_string());
                record.set_data("line_number", line_number.to_string());
                record.parse(&self.parsers);
                record
            })
            .collect();

        self.visible_records = records.clone();
        self.all_records.extend(records);
        self.renumber();
        self.max_record_size = self
            .visible_records
            .iter()
            .map(|r| r.original.len())
            .max()
            .unwrap_or(0);
    }

    pub fn readfile_parallel(&mut self, filename: &str, tx: mpsc::Sender<TuiEvent>) {
        let file = match std::fs::File::open(filename) {
            Ok(file) => file,
            Err(_error) => panic!("Could not open file={:?}", filename),
        };
        let mut reader = std::io::BufReader::new(file);
        let file_size = reader.seek(std::io::SeekFrom::End(0)).unwrap();
        reader.seek(std::io::SeekFrom::Start(0)).unwrap();

        let mut first_line = String::new();

        if let Ok(size) = reader.read_line(&mut first_line) {
            if size > 0 {
                let mut record = Record::new(first_line);
                record.set_data("filename", filename.to_string());
                record.set_data("line_number", "0".to_string());
                record.parse(&self.parsers);
                self.all_records.push(record);
            }
        }

        let lines: Vec<String> = reader.lines().map(|line| line.unwrap()).collect();

        let records: Vec<Record> = lines
            .par_iter()
            .enumerate()
            .map(|(line_number, line)| {
                let mut record = Record::new(line.clone());
                record.set_data("filename", filename.to_string());
                record.set_data("line_number", line_number.to_string());
                record.parse(&self.parsers);
                record
            })
            .collect();

        self.visible_records = records.clone();
        self.all_records.extend(records);
        self.renumber();

        Self::wait_for_changes(filename.to_string(), tx, file_size.try_into().unwrap());
        self.max_record_size = self
            .visible_records
            .iter()
            .map(|r| r.original.len())
            .max()
            .unwrap_or(0);
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
            let mut record = Record::new(line.clone());
            record.set_data("filename", filename.to_string());
            record.set_line_number(line_number);
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

    pub fn add(&mut self, mut record: Record) {
        record.parse(&mut self.parsers);
        record.set_data("line_number", self.all_records.len().to_string());
        self.max_record_size = self.max_record_size.max(record.original.len());
        self.all_records.push(record.clone());

        if self.filter.is_none() || record.matches(&self.filter.as_ref().unwrap()) {
            record.set_line_number(self.visible_records.len());
            self.visible_records.push(record);
        }
    }

    // Executes a command line program and read the output. Waits as in readfile_stdint to send new lines.
    pub fn readfile_exec(&mut self, args: &[&str], tx: mpsc::Sender<TuiEvent>) {
        let mut child = std::process::Command::new("setsid")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("could not execute command");

        let stdout = std::io::BufReader::new(child.stdout.take().expect("could not read stdout"));
        let stderr = std::io::BufReader::new(child.stderr.take().expect("could not read stderr"));
        let tx_stdout = tx.clone();
        let tx_exit = tx.clone();
        let tx_stderr = tx;
        spawn(move || {
            for line in stdout.lines() {
                if let Ok(line) = line {
                    let mut record = Record::new(line);
                    record.set_data("filename", "stdout".into());
                    tx_stdout.send(TuiEvent::NewRecord(record)).unwrap();
                } else {
                    return;
                }
            }
        });
        spawn(move || {
            for line in stderr.lines() {
                if let Ok(line) = line {
                    let mut record = Record::new(line);
                    record.set_data("filename", "stderr".into());
                    tx_stderr.send(TuiEvent::NewRecord(record)).unwrap();
                } else {
                    return;
                }
            }
        });

        let child_pid = child.id();
        // wait for the process to finish
        spawn(move || {
            // wait, but using UNIX pid
            let result = child.wait();
            // wait a bit to send the exit message, to allow read stdin and stdout
            sleep(Duration::from_millis(100));

            let mut record = Record::new(format!("EXIT: {}", result.unwrap()));
            record.set_data("filename", "stderr".into());
            record.set_data("mark", "white red".into());
            tx_exit.send(TuiEvent::NewRecord(record)).unwrap();
        });

        self.child_process = Some(child_pid);
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

    pub fn reparse(&mut self) {
        self.all_records.par_iter_mut().for_each(|record| {
            record.parse(&self.parsers);
        });
        self.visible_records.par_iter_mut().for_each(|record| {
            record.parse(&self.parsers);
        });
        self.renumber();
    }

    pub fn len(&self) -> usize {
        return self.visible_records.len();
    }

    pub fn clear(&mut self) {
        self.all_records.clear();
        self.visible_records.clear();
    }

    pub fn get(&self, index: usize) -> Option<&Record> {
        if index < self.visible_records.len() {
            Some(&self.visible_records[index])
        } else {
            None
        }
    }

    pub fn max_record_size(&self, key: &str) -> usize {
        if self.max_record_size > 0 {
            return self.max_record_size;
        }

        let mut max_size = 0;
        let empty = "".to_string();
        for record in &self.visible_records {
            max_size = max_size.max(record.get(key).unwrap_or(&empty).len());
        }
        max_size
    }
}

impl Drop for RecordList {
    fn drop(&mut self) {
        if let Some(pid) = self.child_process {
            let _result = kill(Pid::from_raw(-(pid as i32)), Signal::SIGTERM);
        }
    }
}

pub fn load_parsers(
    rule: &RulesSettings,
    parsers: &mut Vec<parser::Parser>,
) -> Result<(), parser::ParserError> {
    for extractor in rule.extractors.iter() {
        parsers.push(parser::Parser::new(extractor)?);
    }

    Ok(())
}
