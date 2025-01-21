use crate::ast;
use crate::events::TuiEvent;
use crate::record;
use crate::recordlist;
use crate::settings::RulesSettings;
use crate::settings::Settings;

use crossterm::ExecutableCommand;
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
    pub settings: Settings,
    pub current_rule: RulesSettings,
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
    pub search_ast: Option<ast::AST>,
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
                settings: Settings::new(),
                current_rule: RulesSettings::default(),
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
                search_ast: None,
                number: String::new(),
            },
            terminal,
            tx,
            rx,
        })
    }

    pub fn render(&mut self) -> io::Result<()> {
        let size = self.terminal.size()?;

        let mut visible_lines = size.height as usize - 4;
        if self.state.records.visible_records.len() > 0 {
            visible_lines -= self.state.records.visible_records[self.state.position]
                .data
                .len();
        }
        if self.state.total_visible_lines != visible_lines {
            self.state.total_visible_lines = visible_lines;
        }

        let mainarea = Self::render_records_table(&self.state, size);
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
                    rect.render_widget(Self::render_record(&self.state, current_record), chunks[1]);
                }
                rect.render_widget(footer, chunks[2]);
            })
            .unwrap();

        // set cursor at the begining of the last line
        self.terminal
            .backend_mut()
            .execute(crossterm::cursor::MoveTo(0, size.height - 1))
            .unwrap();

        Ok(())
    }

    pub fn render_records_table<'a>(state: &'a TuiState, size: Size) -> Table<'a> {
        let settings = &state.settings;
        // Columns from SETTINGS.current_rule
        let current_rules = &state.current_rule;
        let columns = &current_rules.columns;
        let start = state.scroll_offset_top;
        let end = min(
            start + state.total_visible_lines,
            state.records.visible_records.len(),
        );

        let records = &state.records.visible_records;
        let mut rows = Vec::new();
        for record in records[start..end].iter() {
            let mut cells: Vec<Cell> = columns
                .iter()
                .map(|column| {
                    let binding = "".to_string();
                    let value = record.data.get(&column.name).unwrap_or(&binding);
                    let cell =
                        Cell::from(Line::from(value.clone()).alignment(match column.align {
                            crate::settings::Alignment::Left => ratatui::layout::Alignment::Left,
                            crate::settings::Alignment::Center => {
                                ratatui::layout::Alignment::Center
                            }
                            crate::settings::Alignment::Right => ratatui::layout::Alignment::Right,
                        }));
                    cell
                })
                .collect();
            let vscroll_left = min(
                max(0, record.original.len() as i32),
                max(0, state.scroll_offset_left as i32),
            ) as usize;
            let vscroll_right: usize = min(
                record.original.len() as i32,
                state.scroll_offset_left as i32 + size.width as i32,
            ) as usize;
            let cell = Cell::from(String::from(&record.original[vscroll_left..vscroll_right]));
            cells.push(cell);

            let row = if record.index == state.position {
                Row::new(cells).style(Style::from(settings.global.colors.highlight))
            } else {
                Row::new(cells).style(Style::from(settings.global.colors.normal))
            };

            rows.push(row);
        }
        let mut header = columns
            .iter()
            .map(|column| Cell::from(column.name.clone()))
            .collect::<Vec<Cell>>();
        header.push(Cell::from("Original"));
        let header = Row::new(header).style(Style::from(settings.global.colors.table.header));
        let mut columns = columns
            .iter()
            .map(|column| column.width as u16)
            .collect::<Vec<u16>>();

        columns.push(size.width - columns.iter().sum::<u16>());

        let table = Table::new(rows, columns).header(header);
        table
    }

    // pub fn render_records<'a>(state: &'a TuiState, size: Size) -> Paragraph<'a> {
    //     let height = size.height as usize - 2;
    //     let width = size.width as usize;
    //     let start = state.scroll_offset_top;

    //     let mut lines: Vec<Line> = vec![];

    //     let style_hightlight = Style::default().fg(Color::Black).bg(Color::White);
    //     for record in state.records.visible_records
    //         [start..std::cmp::min(start + height, state.records.visible_records.len())]
    //         .iter()
    //     {
    //         let style = Self::get_style_for_record(record, state, &state.search_ast);

    //         let line = Self::render_record_line(
    //             record.original.as_str(),
    //             state.search.as_str(),
    //             style,
    //             style_hightlight,
    //             width,
    //         );
    //         // add white space to fill the line

    //         lines.push(line);
    //     }

    //     let text = Text::from(lines);
    //     let ret = Paragraph::new(text).scroll((0, state.scroll_offset_left as u16));

    //     ret
    // }

    // pub fn get_style_for_record<'a>(
    //     record: &'a record::Record,
    //     state: &TuiState,
    //     search: &Option<AST>,
    // ) -> Style {
    //     let settings = &state.settings;
    //     let current_record = state.position;
    //     let current_index = record.index;

    //     if current_index == current_record {
    //         return settings.global.colors.highlight.into();
    //     } else if search.is_some() && record.matches(&search.as_ref().unwrap()) {
    //         return settings.global.colors.normal.into();
    //     }
    //     Style::default().bg(Color::Black).fg(Color::White)
    // }

    // pub fn render_record_line<'a>(
    //     record: &'a str,
    //     search: &'a str,
    //     style: Style,
    //     style_hightlight: Style,
    //     width: usize,
    // ) -> Line<'a> {
    //     let mut spans = vec![];

    //     let parts = if search == "" {
    //         vec![record]
    //     } else {
    //         record.split(search).collect()
    //     };

    //     if record.starts_with(search) {
    //         spans.push(Span::styled(search, style_hightlight));
    //     }

    //     for part in parts[0..parts.len() - 1].iter() {
    //         spans.push(Span::styled(*part, style));
    //         spans.push(Span::styled(search, style_hightlight));
    //     }

    //     if parts[parts.len() - 1] == search {
    //         spans.push(Span::styled(search, style_hightlight));
    //     } else {
    //         spans.push(Span::styled(parts[parts.len() - 1], style));
    //     }

    //     // must be i32 to void underflow
    //     let remaining = width as i32 - record.len() as i32;
    //     if remaining > 0 {
    //         spans.push(Span::styled(" ".repeat(remaining as usize), style));
    //     }

    //     Line::from(spans)
    // }

    pub fn render_record<'a>(state: &TuiState, record: &'a record::Record) -> Paragraph<'a> {
        let settings = &state.settings;
        let mut lines = vec![];

        // text have all the key: value pairs, one by line, in alphabetical order, with key in grey

        let mut keys: Vec<&String> = record.data.keys().collect();
        keys.sort();

        for key in keys {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}: ", key),
                    Style::from(settings.global.colors.details.key),
                ),
                Span::styled(
                    record.data.get(key).unwrap(),
                    Style::from(settings.global.colors.details.value),
                ),
            ]));
        }

        let text = Text::from(lines);
        let title_span = Span::styled(
            record.original.clone(),
            Style::from(settings.global.colors.details.title),
        );

        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title_span)
                    .border_style(Style::from(settings.global.colors.details.border)),
            )
            .style(Style::from(settings.global.colors.details.border))
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
            .title(format!(
                "Search: {}█  AST: {:?}",
                state.search, state.search_ast
            ))
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
        let filter_ast = state.search_ast.as_ref().unwrap_or(&ast::AST::Empty);
        let position_hints = format!(
            "Position {}. Visible {}. Total {}. Read time {}ms. Filter {:?}. Rules {}",
            state.position,
            state.records.visible_records.len(),
            state.records.all_records.len(),
            state.read_time.as_millis(),
            filter_ast,
            state.current_rule.name
        );

        Block::default()
            .title(position_hints)
            .borders(Borders::BOTTOM)
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
                    if self.state.position == max(0, self.state.records.len() as i32 - 2) as usize {
                        self.move_selection(1);
                    }
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
                self.set_position(number);
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
                self.set_position(0);
                self.set_vposition(0);
            }
            KeyCode::End => {
                self.set_position(usize::max_value());
            }
            KeyCode::Right if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.set_vposition(self.state.scroll_offset_left as i32 + 10);
            }
            KeyCode::Left if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.set_vposition(self.state.scroll_offset_left as i32 - 10);
            }

            KeyCode::Right => {
                self.set_vposition(self.state.scroll_offset_left as i32 + 1);
            }
            KeyCode::Left => {
                self.set_vposition(self.state.scroll_offset_left as i32 - 1);
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
            KeyCode::F(1) => {
                self.open_help();
            }
            // control + L
            KeyCode::Char('l') if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.terminal.clear().unwrap();
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
                self.state.search_ast = ast::parse(&self.state.search).ok();
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
        let current = self.state.position;
        self.set_position_wrap(self.state.position as i32 + 1);
        if !self.search_fwd() {
            // if not found, go back to the original position
            self.set_position(current);
        }
    }

    pub fn search_fwd(&mut self) -> bool {
        let search_ast = self.state.search_ast.as_ref();
        if search_ast.is_none() {
            return false;
        }
        let search_ast = search_ast.unwrap();
        let mut current = self.state.position;

        let maybe_position = self.state.records.search_forward(search_ast, current);
        if maybe_position.is_none() {
            return false;
        }
        current = maybe_position.unwrap();
        self.set_position(current);
        true
    }

    pub fn search_prev(&mut self) {
        let current = self.state.position;
        self.set_position_wrap(self.state.position as i32 - 1);
        if !self.search_bwd() {
            self.set_position(current);
        }
    }

    pub fn search_bwd(&mut self) -> bool {
        let search_ast = self.state.search_ast.as_ref();
        if search_ast.is_none() {
            return false;
        }
        let search_ast = search_ast.unwrap();
        let mut current = self.state.position;

        let maybe_position = self.state.records.search_backwards(search_ast, current);
        if maybe_position.is_none() {
            return false;
        }
        current = maybe_position.unwrap();
        self.set_position(current);
        true
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
                self.set_position(0);
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

        self.set_position(current as usize);
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

    pub fn set_position(&mut self, position: usize) {
        let visible_len = self.state.records.visible_records.len();
        if visible_len == 0 {
            self.state.position = 0;
        } else if position >= visible_len {
            self.state.position = visible_len - 1;
        } else {
            self.state.position = position;
        }
        self.ensure_visible(self.state.position);
    }

    pub fn set_position_wrap(&mut self, position: i32) {
        let max = self.state.records.visible_records.len() as i32;
        if max <= 1 {
            self.state.position = 0
        } else if position >= max {
            self.state.position = 0;
        } else if position < 0 {
            self.state.position = (max - 1) as usize;
        } else {
            self.state.position = position as usize;
        }
        self.ensure_visible(self.state.position);
    }

    pub fn set_vposition(&mut self, position: i32) {
        if position < 0 {
            self.state.scroll_offset_left = 0;
        } else {
            self.state.scroll_offset_left = position as usize;
        }
    }

    pub fn open_help(&self) {
        let line = &self
            .state
            .records
            .visible_records
            .get(self.state.position)
            .unwrap()
            .original;

        // remove host name
        let hostname = match hostname::get() {
            Ok(name) => name.to_string_lossy().into_owned(),
            Err(_) => String::from("unknown"),
        };
        let line = line.replace(&hostname, "");
        // remove ips to xxx.xxx.xxx.xx
        let line = regex::Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}")
            .unwrap()
            .replace_all(&line, "xxx.xxx.xxx.xxx");
        // remove username
        let username = whoami::username();
        let line = line.replace(&username, "username");

        // open xdg-open
        let urlencodedline = urlencoding::encode(&line);
        let _output = std::process::Command::new("xdg-open")
            .arg(self.state.settings.help_url.replace("{}", &urlencodedline))
            .output()
            .expect("failed to execute process");
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
