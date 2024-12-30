use crate::events::TuiEvent;
use core::panic;
use std::sync::mpsc;
use std::thread;

use ratatui::crossterm::event::{self};

pub fn start_event_thread(tx: mpsc::Sender<TuiEvent>) {
    thread::spawn(move || loop {
        let ev = event::read();
        match ev {
            Ok(ev) => {
                tx.send(TuiEvent::Key(ev)).unwrap();
            }
            _ => {
                panic!("could not read event");
            }
        }
    });
}
