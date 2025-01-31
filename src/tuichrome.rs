use crate::ast;
use crate::events::TuiEvent;
use crate::record;
use crate::settings::string_to_style;
use crate::state::Mode;
use crate::state::TuiState;
use crate::utils::ansi_to_style;
use crate::utils::clean_ansi_text;
use crate::utils::reverse_style;

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
            state: TuiState::new(),
            terminal,
            tx,
            rx,
        })
    }

    pub fn render(&mut self) -> io::Result<()> {
        let size = self.terminal.size()?;

        let mut visible_lines = size.height as usize - 2;
        if self.state.view_details && self.state.records.visible_records.len() > 0 {
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

                let current_record = if self.state.view_details {
                    self.state.records.visible_records.get(self.state.position)
                } else {
                    None
                };

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
                if current_record.is_some() {
                    rect.render_widget(
                        Self::render_record_details(&self.state, current_record.unwrap()),
                        chunks[1],
                    );
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

            let gutter = if let Some(gutter) = Self::get_gutter_from_record(state, &record) {
                Cell::from(Span::styled(&settings.global.gutter_symbol, gutter))
            } else {
                Cell::from(Span::styled(" ", Style::default()))
            };
            cells.insert(0, gutter);

            // let vscroll_left = min(
            //     max(0, record.original.len() as i32),
            //     max(0, state.scroll_offset_left as i32),
            // ) as usize;
            // let vscroll_right: usize = min(
            //     record.original.len() as i32,
            //     state.scroll_offset_left as i32 + size.width as i32,
            // ) as usize;
            let is_highlighted = state.position == record.index;
            let cell = Cell::from(Self::render_record_original(
                &state,
                &record,
                is_highlighted,
            ));
            cells.push(cell);

            let style = Self::get_row_style(state, &record);
            let row = Row::new(cells).style(style);

            rows.push(row);
        }
        let mut header = columns
            .iter()
            .map(|column| Cell::from(column.name.clone()))
            .collect::<Vec<Cell>>();
        header.insert(0, Cell::from(" "));
        header.push(Cell::from("Original"));
        let header = Row::new(header).style(Style::from(settings.colors.table.header));
        let mut columns = columns
            .iter()
            .map(|column| column.width as u16)
            .collect::<Vec<u16>>();
        columns.insert(0, 1);
        columns.push(size.width - columns.iter().sum::<u16>());

        let table = Table::new(rows, columns).header(header);
        table
    }

    fn render_record_original<'a>(
        state: &'a TuiState,
        record: &record::Record,
        highlight: bool,
    ) -> Line<'a> {
        let original = &record.original;
        // Original has ANSI color codes, we want to create a list of spans with the right colors
        // We will use the same colors as the table, but with a different background

        // First split by ANSI codes
        let mut in_ansi_escape = false;
        let mut ansi_code = String::new();
        let mut text = String::new();
        let mut spans = vec![];
        let mut voffset = state.scroll_offset_left;

        let mut current_style = Self::get_row_style(state, &record);
        if highlight {
            current_style = reverse_style(current_style);
        }

        for c in original.chars() {
            if in_ansi_escape {
                if c == 'm' {
                    in_ansi_escape = false;
                    current_style = ansi_to_style(current_style, &ansi_code);
                    ansi_code.clear();
                } else {
                    ansi_code.push(c);
                }
            } else if c == 0o33 as char {
                // Insert the current text (if any) with the current style
                if text.len() > 0 {
                    let style = if highlight {
                        reverse_style(current_style)
                    } else {
                        current_style
                    };
                    spans.push(Span::styled(text.clone(), style));
                    text.clear();
                }

                in_ansi_escape = true;
                ansi_code.push(c);
            } else {
                if voffset > 0 {
                    voffset -= 1;
                    continue;
                }
                if c == '\t' {
                    let spaces_for_next_tab = 8 - text.len() % 8;
                    for _ in 0..spaces_for_next_tab {
                        text.push(' ');
                    }
                } else {
                    text.push(c);
                }
            }
        }

        if text.len() > 0 {
            let style = if highlight {
                reverse_style(current_style)
            } else {
                current_style
            };
            spans.push(Span::styled(text.clone(), style));
            text.clear();
        }

        Line::from(spans)
    }

    pub fn get_gutter_from_record(state: &TuiState, record: &record::Record) -> Option<Style> {
        let filters = &state.current_rule.filters;

        for filter in filters {
            if record.matches(&filter.expression) {
                if filter.gutter.is_some() {
                    return Some(Style::from(filter.gutter.unwrap()));
                }
            }
        }

        return None;
    }

    pub fn get_row_style(state: &TuiState, record: &record::Record) -> Style {
        let settings = &state.settings;
        let filters = &state.current_rule;

        let mark = record.get("mark");
        let is_mark = mark.is_some();
        let is_selected = record.index == state.position;

        match (is_selected, is_mark) {
            (true, true) => return Style::from(settings.colors.mark_highlight),
            (true, false) => return Style::from(settings.colors.highlight),
            (false, true) => {
                let style = string_to_style(mark.unwrap());
                let style = if style.is_ok() {
                    style.unwrap()
                } else {
                    settings.colors.mark
                };
                return Style::from(style);
            }
            _ => {}
        }

        for filter in &filters.filters {
            if record.matches(&filter.expression) {
                return Style::from(filter.highlight);
            }
        }

        return Style::from(settings.colors.normal);
    }

    pub fn render_record_details<'a>(
        state: &TuiState,
        record: &'a record::Record,
    ) -> Paragraph<'a> {
        let settings = &state.settings;
        let mut lines = vec![];

        // text have all the key: value pairs, one by line, in alphabetical order, with key in grey

        let mut keys: Vec<&String> = record.data.keys().collect();
        keys.sort();

        for key in keys {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}: ", key),
                    Style::from(settings.colors.details.key),
                ),
                Span::styled(
                    record.data.get(key).unwrap(),
                    Style::from(settings.colors.details.value),
                ),
            ]));
        }

        let text = Text::from(lines);
        let title_span = Span::styled(
            clean_ansi_text(&record.original),
            Style::from(settings.colors.details.title),
        );

        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title_span)
                    .border_style(Style::from(settings.colors.details.border)),
            )
            .style(Style::from(settings.colors.details.border))
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
        Self::render_textinput_block("Search", &state.search, state.settings.colors.footer.search)
    }
    pub fn render_footer_filter(state: &TuiState) -> Block {
        Self::render_textinput_block("Filter", &state.filter, state.settings.colors.footer.filter)
    }
    pub fn render_footer_command(state: &TuiState) -> Block {
        Self::render_textinput_block(
            "Command",
            &state.command,
            state.settings.colors.footer.command,
        )
    }
    pub fn render_footer_warning(state: &TuiState) -> Block {
        Block::default().title(state.warning.clone()).style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightYellow)
                .bold(),
        )
    }

    pub fn render_tag(spans: &mut Vec<Span>, label: &str, value: &str, style: Style) {
        let rstyle = reverse_style(style);

        spans.push(Span::styled(format!(" {} ", label), rstyle));
        spans.push(Span::styled(format!(" {} ", value), style));
        spans.push(Span::styled(" ".to_string(), rstyle));
        spans.push(Span::styled(
            " ".to_string(),
            Style::default().fg(Color::Black).bg(Color::Black),
        ));
    }

    pub fn render_textinput_block<'a>(label: &'a str, value: &'a str, style: Style) -> Block<'a> {
        let mut spans = vec![];
        let rstyle = reverse_style(style);

        spans.push(Span::styled(format!(" {} ", label), rstyle));
        spans.push(Span::styled(format!(" {}█", value), style));

        let line = Line::from(spans);

        Block::default().title(line)
    }

    pub fn render_footer_normal(state: &TuiState) -> Block {
        // let filter_ast = state.search_ast.as_ref().unwrap_or(&ast::AST::Empty);

        // Blue for current line
        // Black for max line
        // Yellow for search
        // Green for filter

        let mut spans = vec![];

        Self::render_tag(&mut spans, "F1", "help", state.settings.colors.footer.other);
        Self::render_tag(
            &mut spans,
            ":",
            "commands",
            state.settings.colors.footer.other,
        );
        if state.search != "" {
            Self::render_tag(
                &mut spans,
                "Search",
                &state.search,
                state.settings.colors.footer.search,
            );
        }

        if state.filter != "" {
            Self::render_tag(
                &mut spans,
                "Filter",
                &state.filter,
                state.settings.colors.footer.filter,
            );
        }

        Self::render_tag(
            &mut spans,
            "Rule",
            &state.current_rule.name,
            state.settings.colors.footer.rule,
        );
        Self::render_tag(
            &mut spans,
            "Line",
            format!(
                " {:5} / {:5} ",
                state.position,
                state.records.visible_records.len()
            )
            .as_str(),
            state.settings.colors.footer.line_number,
        );

        let right_line = Line::from(spans);

        // On the left I want the text: "Tailtales (C) 2025 David Moreno"
        // On the right the line

        let version = format!("v{}", env!("CARGO_PKG_VERSION"));
        let mut spans = vec![];

        Self::render_tag(
            &mut spans,
            "Tailtales",
            version.as_str(),
            state.settings.colors.footer.version,
        );

        let left_line = Line::from(spans);

        Block::default()
            .title_style(Style::default().fg(Color::Black).bg(Color::LightGreen))
            .title(left_line)
            .title(right_line.right_aligned())
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
                self.state.mode = self.state.next_mode;
                self.state.next_mode = Mode::Normal;
                self.handle_key_event(key_event); // pass through
            }
        }
    }

    pub fn handle_normal_mode(&mut self, key_event: KeyEvent) {
        let keyname: &str = match key_event.code {
            // numbers add to number
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
        // F1 - F12 are \u{1}... \u{c}
        let keyname = match key_event.code {
            KeyCode::F(1) => "F1",
            KeyCode::F(2) => "F2",
            KeyCode::F(3) => "F3",
            KeyCode::F(4) => "F4",
            KeyCode::F(5) => "F5",
            KeyCode::F(6) => "F6",
            KeyCode::F(7) => "F7",
            KeyCode::F(8) => "F8",
            KeyCode::F(9) => "F9",
            KeyCode::F(10) => "F10",
            KeyCode::F(11) => "F11",
            KeyCode::F(12) => "F12",
            _ => keyname,
        };

        if self.state.settings.keybindings.contains_key(keyname) {
            let command = self.state.settings.keybindings[keyname].clone();

            if command == "refresh_screen" {
                self.refresh_screen();
                return;
            }

            self.state.command = command;
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
            KeyCode::Tab => {
                self.show_completions();
            }
            KeyCode::Esc => {
                state.mode = Mode::Normal;
            }
            KeyCode::Char('\n') => {
                state.mode = Mode::Normal;
                state.handle_command();
            }
            // Control k is delete line
            KeyCode::Char('k') if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                state.command = String::new();
            }
            KeyCode::Char('h') if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                state.command = String::new();
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

    pub fn show_completions(&mut self) {
        let state = &mut self.state;
        let (common_prefix, completions) = state.get_completions();

        if common_prefix != state.command {
            state.command = common_prefix;
            return;
        }
        if completions.len() == 1 {
            state.command = completions[0].clone();
        } else if completions.len() > 1 {
            let completions = completions.join(" █ ");
            state.next_mode = Mode::Command;
            state.set_warning(format!("{}", completions));
        } else {
            state.next_mode = Mode::Command;
            state.set_warning("No completions found".to_string());
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

    fn refresh_screen(&mut self) {
        // force refresh of all screen contents, as some damaged info came into it
        // just draw all black, and then render again
        self.terminal
            .draw(|rect| {
                let chunks = Layout::default()
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(rect.area());
                rect.render_widget(
                    Block::default().style(Style::default().bg(Color::Black)),
                    chunks[0],
                );
            })
            .unwrap();
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
