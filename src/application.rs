use notify::{Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc;
use std::{cmp::max, io, time};

use crate::keyboard_management::handle_key_event;
use crate::{events::TuiEvent, lua_engine::LuaEngine, state::TuiState, tuichrome::TuiChrome};
use crossterm::event::{Event, KeyEventKind};

pub struct Application {
    pub state: TuiState,
    pub ui: TuiChrome,
    pub lua_engine: LuaEngine,
    watcher: RecommendedWatcher,
    settings_rx: mpsc::Receiver<NotifyEvent>,
}

impl Application {
    pub fn new() -> Result<Application, Box<dyn std::error::Error>> {
        let ui = TuiChrome::new()?;
        let state = TuiState::new()?;

        // Initialize the Lua engine
        let mut lua_engine =
            LuaEngine::new().map_err(|e| format!("Failed to initialize Lua engine: {}", e))?;

        // Compile keybinding scripts during initialization
        state
            .settings
            .compile_keybinding_scripts(&mut lua_engine)
            .map_err(|e| format!("Failed to compile keybinding scripts: {}", e))?;

        let (watcher, rx) = Application::create_watcher()?;

        Ok(Application {
            state,
            ui,
            lua_engine,
            watcher,
            settings_rx: rx,
        })
    }

    fn create_watcher() -> io::Result<(RecommendedWatcher, mpsc::Receiver<NotifyEvent>)> {
        // Create a channel for file events
        let (tx, rx) = mpsc::channel();

        // Create the watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<NotifyEvent, _>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Watch the settings file
        if let Some(settings_path) = crate::settings::Settings::local_settings_filename() {
            watcher
                .watch(settings_path.as_ref(), RecursiveMode::NonRecursive)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }
        Ok((watcher, rx))
    }

    pub fn rearm_watcher(&mut self) {
        match Application::create_watcher() {
            Ok((watcher, rx)) => {
                self.watcher = watcher;
                self.settings_rx = rx;
            }
            Err(e) => {
                self.state
                    .set_warning(format!("Error creating watcher: {}", e));
            }
        }
    }

    pub fn run(&mut self) {
        loop {
            // Check for settings file changes
            if let Ok(event) = self.settings_rx.try_recv() {
                match event.kind {
                    notify::EventKind::Access(_) => {
                        self.state.reload_settings();
                    }
                    _ => {}
                }
            }

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

            // Check for settings file changes
            if let Ok(event) = self.settings_rx.try_recv() {
                self.state.set_warning("Settings reloaded".into());
                match event.kind {
                    _ => {
                        // Add a small delay to ensure the file is fully written
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        self.state.reload_settings();
                        self.rearm_watcher();
                    }
                }
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
                        handle_key_event(key_event, &mut self.state, &mut self.lua_engine);
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
