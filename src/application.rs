use std::io;

use crate::{
    state::TuiState,
    tuichrome::TuiChrome,
};

pub struct Application {
    pub state: TuiState,
    pub ui: TuiChrome,
}

impl Application {
    pub fn new() -> io::Result<Application> {
        let ui = TuiChrome::new()?;
        let state = TuiState::new();

        Ok(Application {
            state,
            ui,
        })
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
            if let Err(e) = self.ui.wait_for_events(&mut self.state) {
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
} 