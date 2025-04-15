use crate::events::TuiEvent;
use core::panic;
use std::io::Write;
use std::thread;
use std::{process::ChildStdin, sync::mpsc};

use crossterm::event::Event;
use ratatui::crossterm::event::{self};

#[derive(Debug)]
enum KeyboartInputControl {
    Pause,
    Resume,
}

pub struct KeyboardInput {
    tx: mpsc::Sender<KeyboartInputControl>,
}

impl KeyboardInput {
    pub fn new(tx: mpsc::Sender<TuiEvent>) -> KeyboardInput {
        start_event_thread(tx)
    }

    pub fn pause(&self) {
        self.tx.send(KeyboartInputControl::Pause).unwrap();
    }

    pub fn resume(&self) {
        self.tx.send(KeyboartInputControl::Resume).unwrap();
    }
}

fn start_event_thread(tx: mpsc::Sender<TuiEvent>) -> KeyboardInput {
    let (ktx, krx) = mpsc::channel();
    let ret = KeyboardInput { tx: ktx };

    thread::spawn(move || loop {
        let event_ready: Result<bool, std::io::Error> =
            event::poll(std::time::Duration::from_millis(100));
        match event_ready {
            Ok(true) => {
                read_and_send_event(&tx);
            }

            Ok(false) => {
                maybe_pause(&krx);
            }

            _ => {
                panic!("could not poll event");
            }
        }
    });

    ret
}

fn read_and_send_event(tx: &mpsc::Sender<TuiEvent>) {
    let ev = event::read();

    match ev {
        Ok(ev) => {
            tx.send(TuiEvent::Key(ev)).unwrap();
        }

        _ => {
            panic!("could not read event");
        }
    }
}

fn maybe_pause(rx: &mpsc::Receiver<KeyboartInputControl>) {
    match rx.try_recv() {
        Ok(KeyboartInputControl::Pause) => {
            wait_for_resume(rx);
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            // Do nothing
        }

        ev => {
            panic!(
                "can only receive pause. Receiced {:?}, and in wrong state",
                ev,
            );
            // Do nothing
        }
    }
}

fn wait_for_resume(rx: &mpsc::Receiver<KeyboartInputControl>) {
    loop {
        if maybe_resume(rx) {
            return;
        }
    }
}
fn maybe_resume(rx: &mpsc::Receiver<KeyboartInputControl>) -> bool {
    match rx.recv() {
        Ok(KeyboartInputControl::Resume) => {
            return true;
        }
        Err(_) => {
            return true;
        }

        ev => {
            panic!(
                "can only receive resume. Receiced {:?}, and in wrong state",
                ev,
            );
        }
    }
}
