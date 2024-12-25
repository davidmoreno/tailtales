use crate::record;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use std::{io, time};
use symbols::line;

#[derive(PartialEq, Debug)]
pub enum Mode {
    Normal,
    Search,
}

pub struct TuiState {
    pub records: record::RecordList,
    pub tab_index: usize,
    pub visible_lines: usize,
    pub current_record: usize,
    pub scroll_offset: usize,
    pub running: bool,
    pub read_time: time::Duration,
    pub mode: Mode,
    pub search: String,
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
                visible_lines: 78,
                current_record: 0,
                scroll_offset: 0,
                running: true,
                read_time: time::Duration::new(0, 0),
                mode: Mode::Normal,
                search: String::new(),
            },
            terminal: terminal,
        })
    }

    pub fn render(&mut self) -> io::Result<()> {
        let size = self.terminal.size()?;

        let mut visible_lines = size.height as usize - 6;
        visible_lines -= self.state.records.records[self.state.current_record]
            .data
            .len()
            + 2;
        if self.state.visible_lines != visible_lines {
            self.state.visible_lines = visible_lines;
        }

        let header = Self::render_header(&self.state);
        let mainarea = Self::render_records(&self.state, size);
        let footer = Self::render_footer(&self.state);

        let result = self
            .terminal
            .draw(|rect| {
                let mut layout = Layout::default().direction(Direction::Vertical);

                let current_record = self
                    .state
                    .records
                    .records
                    .get(self.state.current_record)
                    .unwrap();

                let chunks = layout
                    .constraints(
                        [
                            Constraint::Length(1),
                            Constraint::Min(0),
                            Constraint::Length(current_record.data.len() as u16 + 2),
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(rect.area());
                rect.render_widget(header, chunks[0]);
                rect.render_widget(mainarea, chunks[1]);
                rect.render_widget(Self::render_record(current_record), chunks[2]);
                rect.render_widget(footer, chunks[3]);
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

    pub fn render_records<'a>(state: &'a TuiState, size: Size) -> Paragraph<'a> {
        let height = size.height as usize - 2;
        let width = size.width as usize;
        let start = state.scroll_offset;

        let mut lines: Vec<Line> = vec![];

        for record in state.records.records
            [start..std::cmp::min(start + height, state.records.records.len() - 1)]
            .iter()
        {
            let mut spans = vec![];
            let current_index = record
                .data
                .get("line_number")
                .unwrap()
                .parse::<usize>()
                .unwrap();

            if state.current_record == current_index {
                let style = Style::default().bg(Color::Yellow).fg(Color::Black);
                let style_hightlight = Style::default().fg(Color::Yellow).bg(Color::Black);
                Self::render_record_line(
                    record.original.as_str(),
                    state.search.as_str(),
                    &mut spans,
                    style,
                    style_hightlight,
                    width,
                );
            } else {
                let style = Style::default().bg(Color::Black).fg(Color::White);
                let style_hightlight = Style::default().fg(Color::Black).bg(Color::White);
                Self::render_record_line(
                    record.original.as_str(),
                    state.search.as_str(),
                    &mut spans,
                    style,
                    style_hightlight,
                    width,
                );
            }
            // add white space to fill the line

            lines.push(Line::from(spans));
        }

        let text = Text::from(lines);

        let ret = Paragraph::new(text);

        ret
    }

    pub fn render_record_line<'a>(
        record: &'a str,
        search: &'a str,
        spans: &mut Vec<Span<'a>>,
        style: Style,
        style_hightlight: Style,
        width: usize,
    ) {
        let parts = if search == "" {
            vec![record]
        } else {
            record.split(search).collect()
        };

        if record.starts_with(search) {
            spans.push(Span::styled(search, style_hightlight));
        }

        for part in parts[0..parts.len() - 1].iter() {
            spans.push(Span::styled(part.clone(), style));
            spans.push(Span::styled(search, style_hightlight));
        }
        spans.push(Span::styled(parts[parts.len() - 1], style));

        if record.ends_with(search) {
            spans.push(Span::styled(search, style_hightlight));
        }

        let remaining = width as i32 - record.len() as i32;
        if remaining > 0 {
            spans.push(Span::styled(" ".repeat(remaining as usize), style));
        }
    }

    pub fn render_record<'a>(record: &'a record::Record) -> Paragraph<'a> {
        let mut lines = vec![];

        // text have all the key: value pairs, one by line, in alphabetical order, with key in grey

        let mut keys: Vec<&String> = record.data.keys().collect();
        keys.sort();

        for key in keys {
            lines.push(Line::from(vec![
                Span::styled(format!("{}: ", key), Style::default().fg(Color::Yellow)),
                Span::raw(record.data.get(key).unwrap()),
            ]));
        }

        let text = Text::from(lines);

        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(record.original.clone()),
        )
    }

    pub fn render_footer<'a>(state: &TuiState) -> Block<'a> {
        if state.mode == Mode::Search {
            return Block::default()
                .title(format!("Search: {}", state.search))
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Yellow));
        } else {
            let position_hints = format!(
                "Position {}/{}. Read time {}ms",
                state.current_record,
                state.records.records.len(),
                state.read_time.as_millis()
            );

            Block::default()
                .title(position_hints)
                .borders(Borders::BOTTOM)
        }
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
        if self.state.mode == Mode::Normal {
            self.handle_normal_mode(key_event);
        } else {
            self.handle_search_mode(key_event);
        }
    }

    pub fn handle_normal_mode(&mut self, key_event: KeyEvent) {
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
                self.state.current_record = 0;
                self.state.scroll_offset = 0;
            }
            // End
            KeyCode::End => {
                self.state.current_record = self.state.records.records.len() - 1;
                self.state.scroll_offset = self.state.current_record - self.state.visible_lines;
            }
            // F3 search
            KeyCode::F(3) => {
                self.search_next();
            }
            // /
            KeyCode::Char('/') => {
                self.state.mode = Mode::Search;
            }

            _ => {}
        }
    }

    pub fn handle_search_mode(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                self.state.mode = Mode::Normal;
            }
            KeyCode::Char('\n') => {
                self.state.mode = Mode::Normal;
                self.search();
            }
            KeyCode::Char(c) => {
                // search for c
                self.state.search.push(c);
                self.search();
            }
            KeyCode::Backspace => {
                self.state.search.pop();
            }
            KeyCode::Enter => {
                self.state.mode = Mode::Normal;
                self.search();
            }
            KeyCode::F(3) => {
                self.search_next();
            }
            _ => {}
        }
    }

    pub fn search_next(&mut self) {
        if self.state.current_record >= self.state.records.records.len() {
            self.state.current_record = 0;
        } else {
            self.state.current_record += 1;
        }
        self.search();
    }

    pub fn search(&mut self) {
        let mut current = self.state.current_record;
        let search_text: &str = &self.state.search;

        let maybe_position = self.state.records.search(search_text, current);
        if maybe_position.is_none() {
            return;
        }
        current = maybe_position.unwrap();
        self.state.current_record = current;
        self.ensure_visible(current);
    }

    pub fn move_selection(&mut self, delta: i32) {
        // I use i32 all around here as I may get some negatives
        let mut current = self.state.current_record as i32;
        let mut new = current as i32 + delta;
        let max = self.state.records.records.len() as i32 - 1;

        if new <= 0 {
            new = 0;
        }
        if new > max {
            new = max;
        }
        current = new;

        self.state.current_record = current as usize;
        self.ensure_visible(current as usize);
    }

    pub fn ensure_visible(&mut self, current: usize) {
        let visible_lines = self.state.visible_lines as i32;
        let current_i32 = current as i32;

        let mut scroll_offset = self.state.scroll_offset as i32;
        // Make scroll_offset follow the selected_record. Must be between the third and the visible lines - 3
        if current_i32 > scroll_offset + visible_lines - 3 {
            scroll_offset = current_i32 - visible_lines + 3;
        }
        if current_i32 < scroll_offset + 3 {
            scroll_offset = current_i32 - 3;
        }
        // offset can not be negative
        if scroll_offset < 0 {
            scroll_offset = 0;
        }

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
