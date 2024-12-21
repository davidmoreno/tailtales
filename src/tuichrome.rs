use crate::record;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, thread, time::Duration};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Widget},
    Terminal,
};

pub struct TuiState {
    pub records: record::RecordList,
}

pub struct TuiChrome {
    pub state: TuiState,
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TuiChrome {
    pub fn new() -> Result<TuiChrome, io::Error> {
        enable_raw_mode()?;
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal: Terminal<CrosstermBackend<io::Stdout>> = Terminal::new(backend)?;

        Ok(TuiChrome {
            state: TuiState {
                records: record::RecordList::new(),
            },
            terminal: terminal,
        })
    }

    pub fn draw(&mut self) -> Result<(), io::Error> {
        let size = self.terminal.size()?;
        let header = Self::render_header(&self.state);
        let mainarea = Self::render_mainarea(&self.state);
        let footer = Self::render_footer(&self.state);

        let result = self
            .terminal
            .draw(|rect| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Length(1),
                            Constraint::Min(0),
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(size);
                rect.render_widget(header, chunks[0]);
                rect.render_widget(mainarea, chunks[1]);
                rect.render_widget(footer, chunks[2]);
            })
            .unwrap();

        Ok(())
    }

    pub fn render_header<'a>(state: &TuiState) -> Block<'a> {
        Block::default().title("Header").borders(Borders::TOP)
    }

    pub fn render_mainarea<'a>(state: &TuiState) -> Block<'a> {
        let ret = Block::default()
            .title("Main Area")
            .borders(Borders::RIGHT | Borders::LEFT)
            // Set white the background color
            .style(tui::style::Style::default().bg(tui::style::Color::Black));

        ret
    }

    pub fn render_footer<'a>(state: &TuiState) -> Block<'a> {
        Block::default().title("Footer").borders(Borders::BOTTOM)
    }
}

// drop impl
impl Drop for TuiChrome {
    fn drop(&mut self) {
        // restore terminal
        disable_raw_mode().unwrap();
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .unwrap();
        self.terminal.show_cursor().unwrap();
    }
}
