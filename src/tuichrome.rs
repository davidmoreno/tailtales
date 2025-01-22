use crate::ast;
use crate::events::TuiEvent;
use crate::record;
use crate::recordlist;
use crate::settings::RulesSettings;
use crate::settings::Settings;
use crate::state::Mode;
use crate::state::TuiState;

use crossterm::ExecutableCommand;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use std::cmp::min;
use std::sync::mpsc;
use std::{cmp::max, io, time};

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
                command: String::new(),
                warning: String::new(),
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
            Mode::Command => Self::render_footer_command(state),
            Mode::Warning => Self::render_footer_warning(state),
        }
    }

    pub fn render_footer_search(state: &TuiState) -> Block {
        Block::default()
            .title(format!("/{}█", state.search))
            .style(Style::default().fg(Color::Yellow))
    }
    pub fn render_footer_filter(state: &TuiState) -> Block {
        Block::default()
            .title(format!("|{}█", state.filter))
            .style(Style::default().fg(Color::Yellow))
    }
    pub fn render_footer_command(state: &TuiState) -> Block {
        Block::default()
            .title(format!(":{}█", state.command))
            .style(Style::default().fg(Color::Yellow))
    }
    pub fn render_footer_warning(state: &TuiState) -> Block {
        Block::default()
            .title(format!("Warning: {}", state.warning))
            .style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightYellow)
                    .bold(),
            )
    }
    pub fn render_footer_normal(state: &TuiState) -> Block {
        let filter_ast = state.search_ast.as_ref().unwrap_or(&ast::AST::Empty);
        let position_hints = format!(
            "line {} of {}. Read time {}ms. Filter {:?}. Rules {}. Total {} lines.",
            state.position,
            state.records.visible_records.len(),
            state.read_time.as_millis(),
            filter_ast,
            state.current_rule.name,
            state.records.all_records.len(),
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
                        self.state.move_selection(1);
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
            Mode::Command => {
                self.handle_command_mode(key_event);
            }
            Mode::Warning => {
                // Any key will dismiss the warning
                self.state.mode = Mode::Normal;
            }
        }
    }

    pub fn handle_normal_mode(&mut self, key_event: KeyEvent) {
        let keyname: &str = match key_event.code {
            // numbers add to number
            KeyCode::Char(c) if c.is_digit(10) => {
                self.state.number.push(c);
                return;
            }
            KeyCode::Char(x) => &String::from(x).to_lowercase(),
            KeyCode::F(x) => &String::from(x as char),

            x => &x.to_string().to_lowercase(),
        };
        let keyname = if key_event.modifiers.contains(event::KeyModifiers::SHIFT) {
            &format!("shift-{}", keyname)
        } else {
            keyname
        };
        let keyname = if key_event.modifiers.contains(event::KeyModifiers::CONTROL) {
            &format!("control-{}", keyname)
        } else {
            keyname
        };

        if self.state.settings.keybindings.contains_key(keyname) {
            self.state.command = self.state.settings.keybindings[keyname].clone();
            self.state.handle_command();
        } else {
            self.state
                .set_warning(format!("Unknown keybinding: {:?}", keyname));
        }
    }

    pub fn handle_search_mode(&mut self, key_event: KeyEvent) {
        let state = &mut self.state;
        match key_event.code {
            KeyCode::Esc => {
                self.state.mode = Mode::Normal;
            }
            KeyCode::Char('\n') => {
                state.mode = Mode::Normal;
                state.search_fwd();
            }
            KeyCode::Char(c) => {
                // search for c
                state.search.push(c);
                state.search_ast = ast::parse(&state.search).ok();
                state.search_fwd();
            }
            KeyCode::Backspace => {
                state.search.pop();
            }
            KeyCode::Enter => {
                state.mode = Mode::Normal;
                state.search_fwd();
            }
            KeyCode::F(3) => {
                state.search_next();
            }
            _ => {}
        }
    }

    pub fn handle_command_mode(&mut self, key_event: KeyEvent) {
        let state = &mut self.state;
        match key_event.code {
            KeyCode::Esc => {
                state.mode = Mode::Normal;
            }
            KeyCode::Char('\n') => {
                state.mode = Mode::Normal;
                state.handle_command();
            }
            KeyCode::Char(c) => {
                state.command.push(c);
            }
            KeyCode::Backspace => {
                state.command.pop();
            }
            KeyCode::Enter => {
                state.mode = Mode::Normal;
                state.handle_command();
            }
            _ => {}
        }
    }

    pub fn handle_filter_mode(&mut self, key_event: KeyEvent) {
        let state = &mut self.state;
        match key_event.code {
            KeyCode::Esc => {
                state.mode = Mode::Normal;
                state.filter = String::new();
                state.handle_filter()
            }
            KeyCode::Char('\n') => {
                state.mode = Mode::Normal;
                state.handle_filter()
            }
            KeyCode::Char(c) => {
                // filter for c
                state.filter.push(c);
                state.handle_filter()
            }
            KeyCode::Backspace => {
                state.filter.pop();
                state.handle_filter()
            }
            KeyCode::Enter => {
                state.mode = Mode::Normal;
                state.handle_filter()
            }
            _ => {}
        }
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
