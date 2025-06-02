use std::{cmp::max, io, time};

use crate::keyboard_management::handle_key_event;
use crate::{events::TuiEvent, state::TuiState, tuichrome::TuiChrome};
use crossterm::event::{Event, KeyEventKind};

pub struct Application {
    pub state: TuiState,
    pub ui: TuiChrome,
}

impl Application {
    pub fn new() -> io::Result<Application> {
        let ui = TuiChrome::new()?;
        let state = TuiState::new();

        Ok(Application { state, ui })
    }

    pub fn run(&mut self) {
        loop {
            // Update the state
            if let Err(e) = self.ui.update_state(&mut self.state) {
                eprintln!("Error updating state: {}", e);
                break;
            }

            // Render the current state
            if let Err(e) = self.ui.render(&self.state) {
                eprintln!("Error rendering UI: {}", e);
                break;
            }

            // Handle events
            if let Err(e) = self.wait_for_events() {
                if e.kind() == io::ErrorKind::Other && e.to_string() == "Application exit" {
                    break;
                }
                eprintln!("Error handling events: {}", e);
                break;
            }

            // Check if we should exit
            if !self.state.running {
                break;
            }
        }
    }
    /**
     * It waits a lot first time, for any event.
     *
     * If its a key event returns inmediatly, to render changes.
     * If its a new record, keeps 100ms waiting for more records.
     *
     * So if there are a lot of new records, will get them all, and at max 100ms will render.
     */
    pub fn wait_for_events(&mut self) -> io::Result<()> {
        let mut timeout = time::Duration::from_millis(60000);
        let mut events_received = 0;
        loop {
            let event = self.ui.rx.recv_timeout(timeout);

            if event.is_err() {
                return Ok(());
            }
            let event = event.unwrap();

            match event {
                TuiEvent::Key(event) => match event {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        handle_key_event(key_event, &mut self.state);
                        timeout = time::Duration::from_millis(10);
                    }
                    _ => {
                        // Do nothing
                    }
                },
                TuiEvent::NewRecord(record) => {
                    self.state.records.add(record);
                    if self.state.position == max(0, self.state.records.len() as i32 - 2) as usize {
                        self.state.move_selection(1);
                    }
                    // self.wait_for_event_timeout(time::Duration::from_millis(100))?;
                    timeout = time::Duration::from_millis(100);
                }
            }
            events_received += 1;
            if events_received > 100 {
                return Ok(());
            }
        }
    }
}
