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

use crate::{ast::AST, events::TuiEvent, parser::Parser, record::Record};

#[derive(Debug, Default)]
pub struct RecordList {
    pub all_records: Vec<Record>,
    pub visible_records: Vec<Record>,
    pub parsers: Vec<Parser>,
    pub filter: Option<AST>,
    pub child_process: Option<std::process::Child>,
}

impl RecordList {
    pub fn new() -> RecordList {
        RecordList {
            all_records: Vec::new(),
            visible_records: Vec::new(),
            parsers: vec![],
            filter: None,
            child_process: None,
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
    }

    pub fn readfile_parallel(&mut self, filename: &str, tx: mpsc::Sender<TuiEvent>) {
        let file = match std::fs::File::open(filename) {
            Ok(file) => file,
            Err(_error) => panic!("Could not open file={:?}", filename),
        };
        let mut reader = std::io::BufReader::new(file);
        let file_size = reader.seek(std::io::SeekFrom::End(0)).unwrap();
        reader.seek(std::io::SeekFrom::Start(0)).unwrap();

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
        record.parse(&self.parsers);
        record.set_data("line_number", self.all_records.len().to_string());

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

        self.child_process = Some(child);
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

    pub fn len(&self) -> usize {
        return self.visible_records.len();
    }

    pub fn clear(&mut self) {
        self.all_records.clear();
        self.visible_records.clear();
    }

    pub fn mark(&mut self, position: usize) {
        if position >= self.visible_records.len() {
            return;
        }
        let record = self.visible_records.get_mut(position).unwrap();

        if record.get("mark").is_some() {
            record.unset_data("mark");
        } else {
            record.set_data("mark", "true".into());
        }
    }
}

impl Drop for RecordList {
    fn drop(&mut self) {
        if let Some(child) = self.child_process.as_mut() {
            let pid = child.id();
            let _result = kill(Pid::from_raw(-(pid as i32)), Signal::SIGTERM);
            sleep(Duration::from_secs(1));
            if child.try_wait().is_err() {
                let _result = kill(Pid::from_raw(-(pid as i32)), Signal::SIGKILL);
            }
        }
    }
}
