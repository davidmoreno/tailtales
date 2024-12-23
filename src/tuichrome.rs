use crate::record;

use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect, Size},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table, Tabs},
    Terminal,
};
use std::{fmt::Result, io};

pub struct TuiState {
    pub records: record::RecordList,
    pub tab_index: usize,
    pub selected_record: Option<usize>,
    pub visible_lines: usize,
    pub current_record: usize,
    pub scroll_offset: usize,
    pub running: bool,
}

pub struct TuiChrome {
    pub state: TuiState,
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TuiChrome {
    pub fn new() -> io::Result<TuiChrome> {
        let mut terminal = ratatui::init();
        Ok(TuiChrome {
            state: TuiState {
                records: record::RecordList::new(),
                tab_index: 0,
                selected_record: Option::Some(1),
                visible_lines: 78,
                current_record: 1,
                scroll_offset: 0,
                running: true,
            },
            terminal: terminal,
        })
    }

    pub fn render(&mut self) -> io::Result<()> {
        let size = self.terminal.size()?;

        let mut visible_lines = size.height as usize - 6;
        if self.state.visible_lines != visible_lines {
            self.state.visible_lines = visible_lines;
        }

        let header = Self::render_header(&self.state);
        let mainarea = Self::render_mainarea(&self.state, size);
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
                    .split(rect.area());
                rect.render_widget(header, chunks[0]);
                rect.render_widget(mainarea, chunks[1]);
                rect.render_widget(footer, chunks[2]);
            })
            .unwrap();

        Ok(())
    }

    pub fn render_header(state: &TuiState) -> Tabs {
        let titles = vec!["File", "Edit"];

        Tabs::new(titles)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow))
            .select(state.tab_index)
    }

    pub fn render_mainarea<'a>(state: &TuiState, size: Size) -> Table<'a> {
        let height = size.height as usize - 2;
        let width = size.width as u16 - 2;
        let start = state.scroll_offset;

        // do constriants with static storage, as a statinc in C++

        let ret = Table::new(
            state.records.records
                [start..std::cmp::min(start + height, state.records.records.len() - 1)]
                .iter()
                .map(|record| {
                    let ret = Row::new(vec![
                        Cell::from(record.original.clone()),
                        Cell::from(record.data.get("word_count").unwrap().clone()),
                    ]);

                    let current_index = record
                        .data
                        .get("line_number")
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();

                    if state.current_record == current_index {
                        ret.style(Style::default().bg(Color::Yellow).fg(Color::Black))
                    } else {
                        ret
                    }
                }),
            vec![Constraint::Min(80), Constraint::Length(6)],
        )
        .header(
            Row::new(vec!["Original", "Word Count"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .title("Main Area")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White).bg(Color::Black)),
        );
        ret
    }

    pub fn render_footer<'a>(state: &TuiState) -> Block<'a> {
        let position_hints = format!(
            "Position {}/{}",
            state.current_record,
            state.records.records.len()
        );

        Block::default()
            .title(position_hints)
            .borders(Borders::BOTTOM)
    }

    pub fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => {
                self.state.running = false;
            }
            KeyCode::Char('j') => {
                self.move_selection(1);
            }
            KeyCode::Char('k') => {
                self.move_selection(-1);
            }
            KeyCode::Char('d') => {
                self.move_selection(10);
            }
            KeyCode::Char('u') => {
                self.move_selection(-10);
            }
            // Keycode up
            KeyCode::Up => {
                self.move_selection(-1);
            }
            KeyCode::Down => {
                self.move_selection(1);
            }
            KeyCode::PageUp => {
                self.move_selection(-10);
            }
            KeyCode::PageDown => {
                self.move_selection(10);
            }
            // Start
            KeyCode::Home => {
                self.state.current_record = 1;
                self.state.scroll_offset = 0;
            }
            // End
            KeyCode::End => {
                self.state.current_record = self.state.records.records.len() - 1;
                self.state.scroll_offset = self.state.current_record - self.state.visible_lines;
            }

            _ => {}
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        // I use i32 all around here as I may get some negatives
        let mut current = self.state.current_record as i32;
        let mut new = current as i32 + delta;
        let max = self.state.records.records.len() as i32 - 1;

        if new <= 0 {
            new = 1;
        }
        if new > max {
            new = max;
        }
        current = new;

        let visible_lines = self.state.visible_lines as i32;
        let mut scroll_offset = self.state.scroll_offset as i32;
        // Make scroll_offset follow the selected_record. Must be between the third and the visible lines - 3
        if current > scroll_offset + visible_lines - 3 {
            scroll_offset = current - visible_lines + 3;
        }
        if current < scroll_offset + 3 {
            scroll_offset = current - 3;
        }
        // offset can not be negative
        if scroll_offset < 0 {
            scroll_offset = 0;
        }

        self.state.current_record = current as usize;
        self.state.scroll_offset = scroll_offset as usize;
    }

    pub fn run(&mut self) {
        loop {
            self.render().unwrap();
            self.handle_events().unwrap();

            if !self.state.running {
                break;
            }
        }
    }
}

// drop impl
impl Drop for TuiChrome {
    fn drop(&mut self) {
        // restore terminal
        ratatui::restore();
    }
}
