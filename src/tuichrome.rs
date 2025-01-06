use crate::ast;
use crate::events::TuiEvent;
use crate::record;
use crate::recordlist;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use std::cmp::min;
use std::sync::mpsc;
use std::{cmp::max, io, time};

#[derive(PartialEq, Debug)]
pub enum Mode {
    Normal,
    Search,
    Filter,
}

pub struct TuiState {
    pub records: recordlist::RecordList,
    pub total_visible_lines: usize,
    pub position: usize,
    pub scroll_offset_top: usize,
    pub scroll_offset_left: usize,
    pub running: bool,
    pub read_time: time::Duration,
    pub mode: Mode,
    pub search: String,
    pub filter: String,
    pub number: String,
}

pub struct TuiChrome {
    pub state: TuiState,
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    pub tx: mpsc::Sender<TuiEvent>,
    pub rx: mpsc::Receiver<TuiEvent>,
}

impl TuiChrome {
    pub fn new() -> io::Result<TuiChrome> {
        let terminal = ratatui::init();
        let (tx, rx) = mpsc::channel();

        Ok(TuiChrome {
            state: TuiState {
                records: recordlist::RecordList::new(),
                total_visible_lines: 78,
                position: 0,
                scroll_offset_top: 0,
                scroll_offset_left: 0,
                running: true,
                read_time: time::Duration::new(0, 0),
                mode: Mode::Normal,
                search: String::new(),
                filter: String::new(),
                number: String::new(),
            },
            terminal,
            tx,
            rx,
        })
    }

    pub fn render(&mut self) -> io::Result<()> {
        let size = self.terminal.size()?;

        let mut visible_lines = size.height as usize - 6;
        if self.state.records.visible_records.len() > 0 {
            visible_lines -= self.state.records.visible_records[self.state.position]
                .data
                .len();
        }
        if self.state.total_visible_lines != visible_lines {
            self.state.total_visible_lines = visible_lines;
        }

        let mainarea = Self::render_records(&self.state, size);
        let footer = Self::render_footer(&self.state);

        self.terminal
            .draw(|rect| {
                let layout = Layout::default().direction(Direction::Vertical);

                let current_record = self.state.records.visible_records.get(self.state.position);

                let main_area_height = if let Some(current_record) = current_record {
                    min(size.height / 2, current_record.data.len() as u16 + 2)
                } else {
                    0
                };

                let chunks = layout
                    .constraints(
                        [
                            Constraint::Min(0),
                            Constraint::Length(main_area_height),
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(rect.area());
                // rect.render_widget(header, chunks[0]);
                rect.render_widget(mainarea, chunks[0]);
                if let Some(current_record) = current_record {
                    rect.render_widget(Self::render_record(current_record), chunks[1]);
                }
                rect.render_widget(footer, chunks[2]);
            })
            .unwrap();

        Ok(())
    }

    pub fn render_records<'a>(state: &'a TuiState, size: Size) -> Paragraph<'a> {
        let height = size.height as usize - 2;
        let width = size.width as usize;
        let start = state.scroll_offset_top;

        let mut lines: Vec<Line> = vec![];

        let style_hightlight = Style::default().fg(Color::Black).bg(Color::White);
        for record in state.records.visible_records
            [start..std::cmp::min(start + height, state.records.visible_records.len())]
            .iter()
        {
            let style = Self::get_style_for_record(record, state);

            let line = Self::render_record_line(
                record.original.as_str(),
                state.search.as_str(),
                style,
                style_hightlight,
                width,
            );
            // add white space to fill the line

            lines.push(line);
        }

        let text = Text::from(lines);
        let ret = Paragraph::new(text).scroll((0, state.scroll_offset_left as u16));

        ret
    }

    pub fn get_style_for_record<'a>(record: &'a record::Record, state: &TuiState) -> Style {
        let current_record = state.position;
        let current_index = record.index;

        if current_index == current_record {
            return Style::default().bg(Color::Yellow).fg(Color::Black);
        }
        Style::default().bg(Color::Black).fg(Color::White)
    }

    pub fn render_record_line<'a>(
        record: &'a str,
        search: &'a str,
        style: Style,
        style_hightlight: Style,
        width: usize,
    ) -> Line<'a> {
        let mut spans = vec![];

        let parts = if search == "" {
            vec![record]
        } else {
            record.split(search).collect()
        };

        if record.starts_with(search) {
            spans.push(Span::styled(search, style_hightlight));
        }

        for part in parts[0..parts.len() - 1].iter() {
            spans.push(Span::styled(*part, style));
            spans.push(Span::styled(search, style_hightlight));
        }

        if parts[parts.len() - 1] == search {
            spans.push(Span::styled(search, style_hightlight));
        } else {
            spans.push(Span::styled(parts[parts.len() - 1], style));
        }

        // must be i32 to void underflow
        let remaining = width as i32 - record.len() as i32;
        if remaining > 0 {
            spans.push(Span::styled(" ".repeat(remaining as usize), style));
        }

        Line::from(spans)
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
        let title_span = Span::styled(record.original.clone(), Style::default().fg(Color::Yellow));

        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title_span))
    }

    pub fn render_footer<'a>(state: &'a TuiState) -> Block<'a> {
        match state.mode {
            Mode::Normal => Self::render_footer_normal(state),
            Mode::Search => Self::render_footer_search(state),
            Mode::Filter => Self::render_footer_filter(state),
        }
    }

    pub fn render_footer_search(state: &TuiState) -> Block {
        Block::default()
            .title(format!("Search: {}", state.search))
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Yellow))
    }
    pub fn render_footer_filter(state: &TuiState) -> Block {
        Block::default()
            .title(format!("Filter: {}█", state.filter))
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Yellow))
    }
    pub fn render_footer_normal(state: &TuiState) -> Block {
        let filter_ast = ast::parse(&state.filter).unwrap_or(ast::AST::Empty);
        let position_hints = format!(
            "Position {}. Visible {}. Total {}. Read time {}ms. Filter {:?}.",
            state.position,
            state.records.visible_records.len(),
            state.records.all_records.len(),
            state.read_time.as_millis(),
            filter_ast,
        );

        Block::default()
            .title(position_hints)
            .borders(Borders::BOTTOM)
    }

    /**
     * It waits for any key eent and inmediatly returns Ok, or
     * if it receives a new record, it will wait for 100ms and wait for more records
     */
    pub fn wait_for_events(&mut self) -> io::Result<()> {
        let mut timeout = time::Duration::from_millis(0);
        loop {
            let event = self.rx.recv_timeout(timeout);

            if event.is_err() {
                return Ok(());
            }
            let event = event.unwrap();

            match event {
                TuiEvent::Key(event) => match event {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_key_event(key_event);
                        return Ok(());
                    }
                    _ => {
                        // Do nothing
                    }
                },
                TuiEvent::NewRecord(record) => {
                    self.state.records.add(record);
                    timeout = time::Duration::from_millis(100);
                    // self.wait_for_event_timeout(time::Duration::from_millis(100))?;
                }
            }
            if timeout.as_millis() <= 0 {
                return Ok(());
            }
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.state.mode {
            Mode::Normal => {
                self.handle_normal_mode(key_event);
            }
            Mode::Search => {
                self.handle_search_mode(key_event);
            }
            Mode::Filter => {
                self.handle_filter_mode(key_event);
            }
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
            // numbers add to number
            KeyCode::Char(c) if c.is_digit(10) => {
                self.state.number.push(c);
            }
            // G go to number position
            KeyCode::Char('G') => {
                let number = self.state.number.parse::<usize>().unwrap_or(0);
                self.state.position = number;
                self.ensure_visible(number);
                self.state.number.clear();
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
            KeyCode::Home => {
                self.state.position = 0;
                self.ensure_visible(0);
            }
            KeyCode::End => {
                self.state.position = self.state.records.visible_records.len() - 1;
                self.ensure_visible(self.state.records.visible_records.len() - 1);
            }
            KeyCode::Right if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.state.scroll_offset_left += 10;
            }
            KeyCode::Left if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.state.scroll_offset_left =
                    max(0, self.state.scroll_offset_left as i32 - 10) as usize;
            }

            KeyCode::Right => {
                self.state.scroll_offset_left += 1;
            }
            KeyCode::Left => {
                self.state.scroll_offset_left =
                    max(0, self.state.scroll_offset_left as i32 - 1) as usize;
            }

            KeyCode::Char('n') => {
                self.search_next();
            }
            KeyCode::Char('N') => {
                self.search_prev();
            }
            KeyCode::Char('f') => {
                self.state.mode = Mode::Filter;
            }
            KeyCode::F(3) if key_event.modifiers.contains(event::KeyModifiers::SHIFT) => {
                self.search_prev();
            }
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
                self.search_fwd();
            }
            KeyCode::Char(c) => {
                // search for c
                self.state.search.push(c);
                self.search_fwd();
            }
            KeyCode::Backspace => {
                self.state.search.pop();
            }
            KeyCode::Enter => {
                self.state.mode = Mode::Normal;
                self.search_fwd();
            }
            KeyCode::F(3) => {
                self.search_next();
            }
            _ => {}
        }
    }

    pub fn search_next(&mut self) {
        if self.state.position >= self.state.records.visible_records.len() {
            self.state.position = 0;
        } else {
            self.state.position += 1;
        }
        self.search_fwd();
    }

    pub fn search_fwd(&mut self) {
        let mut current = self.state.position;
        let search_text: &str = &self.state.search;
        let ast = ast::parse(search_text).unwrap();

        let maybe_position = self.state.records.search_forward(&ast, current);
        if maybe_position.is_none() {
            return;
        }
        current = maybe_position.unwrap();
        self.state.position = current;
        self.ensure_visible(current);
    }

    pub fn search_prev(&mut self) {
        if self.state.position == 0 {
            self.state.position = self.state.records.visible_records.len() - 1;
        } else {
            self.state.position -= 1;
        }
        self.search_bwd();
    }

    pub fn search_bwd(&mut self) {
        let mut current = self.state.position;
        let search_text: &str = &self.state.search;
        let ast = ast::parse(search_text).unwrap();

        let maybe_position = self.state.records.search_backwards(&ast, current);
        if maybe_position.is_none() {
            return;
        }
        current = maybe_position.unwrap();
        self.state.position = current;
        self.ensure_visible(current);
    }

    pub fn handle_filter_mode(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                self.state.mode = Mode::Normal;
                self.state.filter = String::new();
                self.handle_filter()
            }
            KeyCode::Char('\n') => {
                self.state.mode = Mode::Normal;
                self.handle_filter()
            }
            KeyCode::Char(c) => {
                // filter for c
                self.state.filter.push(c);
                self.handle_filter()
            }
            KeyCode::Backspace => {
                self.state.filter.pop();
                self.handle_filter()
            }
            KeyCode::Enter => {
                self.state.mode = Mode::Normal;
                self.handle_filter()
            }
            _ => {}
        }
    }

    pub fn handle_filter(&mut self) {
        let parsed = ast::parse(&self.state.filter);
        match parsed {
            Ok(parsed) => {
                self.state.records.filter_parallel(parsed);
                self.state.position = 0;
                self.ensure_visible(self.state.position);
            }
            Err(_err) => {
                // panic!("TODO show error parsing: {}", err);
            }
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        // I use i32 all around here as I may get some negatives
        let mut current = self.state.position as i32;
        let mut new = current as i32 + delta;
        let max = self.state.records.visible_records.len() as i32 - 1;

        if new <= 0 {
            new = 0;
        }
        if new > max {
            new = max;
        }
        current = new;

        self.state.position = current as usize;
        self.ensure_visible(current as usize);
    }

    pub fn ensure_visible(&mut self, current: usize) {
        let visible_lines = self.state.total_visible_lines as i32;
        let current_i32 = current as i32;

        let mut scroll_offset = self.state.scroll_offset_top as i32;
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

        self.state.scroll_offset_top = scroll_offset as usize;
    }

    pub fn run(&mut self) {
        loop {
            self.render().unwrap();
            self.wait_for_events().unwrap();

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
