use crate::events::TuiEvent;
use crate::record;
use crate::settings::string_to_style;
use crate::state::Mode;
use crate::state::TuiState;
use crate::utils::ansi_to_style;
use crate::utils::clean_ansi_text;
use crate::utils::parse_tabs;
use crate::utils::reverse_style;

use crossterm::ExecutableCommand;
use ratatui::{prelude::*, widgets::*};
use std::cmp::max;
use std::cmp::min;
use std::io;
use std::sync::mpsc;

pub struct TuiChrome {
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    pub tx: mpsc::Sender<TuiEvent>,
    pub rx: mpsc::Receiver<TuiEvent>,
}

// Helper struct to track style changes and search matches
#[derive(Debug, Clone)]
struct StyleChange {
    position: usize,
    style: Style,
}

impl TuiChrome {
    pub fn new() -> io::Result<TuiChrome> {
        let terminal = ratatui::init();
        let (tx, rx) = mpsc::channel();

        Ok(TuiChrome { terminal, tx, rx })
    }

    pub fn update_state(&mut self, state: &mut TuiState) -> io::Result<()> {
        // update the viewport state
        let visible_width = self.terminal.size()?.width as i32;
        if visible_width != state.visible_width as i32 {
            state.visible_width = visible_width as usize;
        }

        let mut visible_lines = self.terminal.size()?.height as i32 - 2; // header and footer
        if state.view_details && state.records.visible_records.len() > 0 {
            if let Some(record) = state.records.visible_records.get(state.position - 1) {
                visible_lines = visible_lines - 3 - 2; // frame + separator + padding
                visible_lines = visible_lines - record.data.len() as i32; // data lines

                // If more than one line for being too wide, use several lines, minimum 1
                visible_lines =
                    visible_lines - ((record.data.len() as i32) / (visible_width / 2)) as i32 + 1;
            }
        }

        if visible_lines < 0 {
            visible_lines = 0;
        }
        if visible_lines != state.visible_height as i32 {
            state.visible_height = visible_lines as usize;
        }

        if state.pending_refresh {
            self.refresh_screen(state);
            state.pending_refresh = false;
        }

        Ok(())
    }

    /// Main render function - dispatches to appropriate renderer based on mode
    pub fn render(&mut self, state: &TuiState) -> io::Result<()> {
        let size = self.terminal.size()?;

        // Dispatch to appropriate renderer based on mode
        if state.mode == Mode::LuaRepl {
            self.render_repl_mode(state)?;
        } else {
            self.render_normal_mode(state, size)?;
        }

        // Set cursor appropriately for the current mode
        self.set_cursor_for_mode(state, size)?;
        Ok(())
    }

    /// Render REPL mode with output and input
    fn render_repl_mode(&mut self, state: &TuiState) -> io::Result<()> {
        let footer = Self::render_footer(state);
        let repl_output = Self::render_repl_output(state);

        self.terminal
            .draw(|rect| {
                let layout = Layout::default().direction(Direction::Vertical);
                let chunks = layout
                    .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
                    .split(rect.area());
                rect.render_widget(repl_output, chunks[0]);
                rect.render_widget(footer, chunks[1]);
            })
            .unwrap();

        Ok(())
    }

    /// Render normal mode with records table and optional details
    fn render_normal_mode(&mut self, state: &TuiState, size: Size) -> io::Result<()> {
        let footer = Self::render_footer(state);
        let mainarea = Self::render_records_table(state, size);
        let constraints = self.calculate_layout_constraints(state, size);
        let current_record = self.get_current_record_for_details(state);

        self.terminal
            .draw(|rect| {
                let layout = Layout::default().direction(Direction::Vertical);
                let chunks = layout.constraints(&constraints).split(rect.area());

                rect.render_widget(mainarea, chunks[0]);

                // Render record details if available
                if let Some(record) = current_record {
                    rect.render_widget(Self::render_record_details(state, record), chunks[1]);
                    rect.render_widget(footer, chunks[2]);
                } else {
                    rect.render_widget(footer, chunks[1]);
                }
            })
            .unwrap();

        Ok(())
    }

    /// Calculate layout constraints for normal mode
    fn calculate_layout_constraints(&self, state: &TuiState, size: Size) -> Vec<Constraint> {
        if let Some(current_record) = self.get_current_record_for_details(state) {
            let main_area_height = min(
                size.height / 2,
                current_record.data.len() as u16
                    + 3
                    + Self::record_wrap_lines_count(current_record, state) as u16,
            );
            vec![
                Constraint::Min(0),
                Constraint::Length(main_area_height),
                Constraint::Length(1),
            ]
        } else {
            vec![Constraint::Min(0), Constraint::Length(1)]
        }
    }

    /// Get current record if details should be shown
    fn get_current_record_for_details<'a>(
        &self,
        state: &'a TuiState,
    ) -> Option<&'a crate::record::Record> {
        if state.view_details {
            state.records.visible_records.get(state.position - 1)
        } else {
            None
        }
    }

    /// Set cursor position based on current mode
    fn set_cursor_for_mode(&mut self, state: &TuiState, size: Size) -> io::Result<()> {
        if state.mode == Mode::LuaRepl {
            // Hide cursor in REPL mode since we show our own cursor in the output
            self.terminal
                .backend_mut()
                .execute(crossterm::cursor::Hide)
                .unwrap();
        } else {
            // Set cursor at the beginning of the last line for other modes
            self.terminal
                .backend_mut()
                .execute(crossterm::cursor::Show)
                .unwrap();
            self.terminal
                .backend_mut()
                .execute(crossterm::cursor::MoveTo(0, size.height - 1))
                .unwrap();
        }
        Ok(())
    }

    pub fn render_records_table<'a>(state: &'a TuiState, size: Size) -> Table<'a> {
        let settings = &state.settings;
        // Columns from SETTINGS.current_rule
        let current_rules = &state.current_rule;
        let columns = &current_rules.columns;
        let start = state.scroll_offset_top;
        let end = min(
            start + state.visible_height,
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
                Cell::from(Span::styled(" ", settings.colors.normal))
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
            let cell = Cell::from(Self::render_record_original(&state, &record));
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
        columns.push(min(
            size.width as i32 - state.records.max_record_size("Original") as i32,
            80,
        ) as u16);

        let table = Table::new(rows, columns).header(header);
        table
    }

    // Process text and return a list of style changes
    fn process_text_styles(text: &str, search: &str, initial_style: Style) -> Vec<StyleChange> {
        let mut style_changes = Vec::new();
        let mut current_style = initial_style;
        let mut in_ansi_escape = false;
        let mut ansi_code = String::new();
        let mut plain_text = String::new();
        let mut current_pos = 0;

        // First pass: collect ANSI codes and build plain text
        for c in text.chars() {
            if in_ansi_escape {
                if c == 'm' {
                    in_ansi_escape = false;
                    current_style = ansi_to_style(current_style, &ansi_code);
                    style_changes.push(StyleChange {
                        position: current_pos,
                        style: current_style,
                    });
                    ansi_code.clear();
                } else {
                    ansi_code.push(c);
                }
            } else if c == 0o33 as char {
                in_ansi_escape = true;
                ansi_code.push(c);
            } else {
                plain_text.push(c);
                current_pos += 1;
            }
        }

        // Second pass: find search matches and add them to style changes
        if !search.is_empty() {
            let text_lower = plain_text.to_lowercase();
            let search_lower = search.to_lowercase();
            let mut start = 0;

            while let Some(pos) = text_lower[start..].find(&search_lower) {
                let match_start = start + pos;
                let match_end = match_start + search.len();

                // Find the style at match_start
                let style_at_match = style_changes
                    .iter()
                    .rev()
                    .find(|change| change.position <= match_start)
                    .map(|change| change.style)
                    .unwrap_or(initial_style);

                // Add start of match
                style_changes.push(StyleChange {
                    position: match_start,
                    style: reverse_style(style_at_match),
                });

                // Add end of match
                style_changes.push(StyleChange {
                    position: match_end,
                    style: style_at_match,
                });

                start = match_end;
            }
        }

        // Sort style changes by position
        style_changes.sort_by_key(|change| change.position);
        style_changes
    }

    fn render_record_original<'a>(state: &'a TuiState, record: &record::Record) -> Line<'a> {
        let original = &record.original;
        let original = parse_tabs(original);
        let voffset = state.scroll_offset_left;
        let initial_style = Self::get_row_style(state, &record);

        // Skip characters at the beginning based on voffset, this converts from utf8 chars to skip to bytes to skip
        let mut skip_chars = voffset;
        let mut start_pos = 0;
        for (i, c) in original.char_indices() {
            if skip_chars > 0 {
                skip_chars -= 1;
                start_pos = i + c.len_utf8();
            } else {
                break;
            }
        }

        // Process text and get style changes, we get an array of style changes, with the position of the change, the style, and if it is a match
        let style_changes = Self::process_text_styles(&original, &state.search, initial_style);
        let clean_original = clean_ansi_text(&original);

        // Build spans based on style changes
        let mut spans = Vec::new();
        let mut current_pos = start_pos;
        let mut current_style = initial_style;

        for change in style_changes {
            if change.position > current_pos {
                let text = clean_original[current_pos..change.position].to_string();
                spans.push(Span::styled(text, current_style));
            }
            current_style = change.style;
            current_pos = max(current_pos, change.position);
        }

        // Add remaining text
        let text = &clean_original[current_pos..];
        if text.len() > 0 {
            spans.push(Span::styled(text.to_string(), current_style));
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
                if filter.highlight.is_some() {
                    return Style::from(filter.highlight.unwrap());
                }
            }
        }

        return Style::from(settings.colors.normal);
    }

    // Helper function to wrap text at word boundaries
    fn wrap_text(text: &str, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0;

        for word in text.split_whitespace() {
            let word_width = word.chars().count();

            // If adding this word would exceed the width, start a new line
            if current_width + word_width + (if current_width > 0 { 1 } else { 0 }) > width {
                if !current_line.is_empty() {
                    lines.push(current_line.trim().to_string());
                }
                current_line = word.to_string();
                current_width = word_width;
            } else {
                if current_width > 0 {
                    current_line.push(' ');
                    current_width += 1;
                }
                current_line.push_str(word);
                current_width += word_width;
            }
        }

        // Add the last line if it's not empty
        if !current_line.is_empty() {
            lines.push(current_line.trim().to_string());
        }

        lines
    }

    fn record_wrap_lines_count(record: &record::Record, state: &TuiState) -> usize {
        let title_width = state.visible_width - 2; // Account for borders
        let title_text = clean_ansi_text(&record.original);
        let wrapped_title = Self::wrap_text(&title_text, title_width);
        wrapped_title.len()
    }

    pub fn render_record_details<'a>(
        state: &'a TuiState,
        record: &'a record::Record,
    ) -> Paragraph<'a> {
        let settings = &state.settings;
        let mut lines = vec![];

        // Get the available width for the title (accounting for borders)
        let title_width = state.visible_width - 2; // Account for borders
        let title_text = clean_ansi_text(&record.original);
        let wrapped_title = Self::wrap_text(&title_text, title_width);

        // Add all wrapped lines at the beginning
        for line in &wrapped_title {
            lines.push(Line::from(vec![Span::styled(
                line.clone(),
                Style::from(settings.colors.details.title),
            )]));
        }

        // Add a blank line between title and key-value pairs
        if !wrapped_title.is_empty() {
            lines.push(Line::from(""));
        }

        // text have all the key: value pairs, one by line, in alphabetical order, with key in grey
        let mut keys: Vec<&String> = record.data.keys().collect();
        keys.sort();

        for key in keys {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} = ", key),
                    Style::from(settings.colors.details.key),
                ),
                Span::styled(
                    record.data.get(key).unwrap(),
                    Style::from(settings.colors.details.value),
                ),
            ]));
        }

        let text = Text::from(lines);

        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
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
            Mode::ScriptInput => Self::render_footer_script_input(state),
            Mode::LuaRepl => Self::render_footer_lua_repl(state),
        }
    }

    pub fn render_footer_search(state: &TuiState) -> Block {
        Self::render_textinput_block(
            "Search",
            &state.search,
            state.text_edit_position,
            state.settings.colors.footer.search,
        )
    }
    pub fn render_footer_filter(state: &TuiState) -> Block {
        let style = if state.filter_ok {
            state.settings.colors.footer.filter
        } else {
            Style::default().fg(Color::Red).bg(Color::Black)
        };
        Self::render_textinput_block("Filter", &state.filter, state.text_edit_position, style)
    }
    pub fn render_footer_command(state: &TuiState) -> Block {
        Self::render_textinput_block(
            "Command",
            &state.command,
            state.text_edit_position,
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

    pub fn render_footer_script_input(state: &TuiState) -> Block {
        Self::render_textinput_block(
            &state.script_prompt,
            &state.script_input,
            state.text_edit_position,
            state.settings.colors.footer.command, // Use command colors for now
        )
    }

    pub fn render_footer_lua_repl(state: &TuiState) -> Block {
        // Simple footer for REPL mode - no input field here since input is shown inline
        let mut spans = vec![];

        if state.repl_is_multiline {
            Self::render_tag(
                &mut spans,
                "Multiline Mode",
                "Ctrl+C to cancel",
                state.settings.colors.footer.command,
            );
        }

        Self::render_tag(
            &mut spans,
            "Lua REPL",
            "ESC to exit",
            state.settings.colors.footer.other,
        );
        Self::render_tag(
            &mut spans,
            "History",
            "↑↓ arrows",
            state.settings.colors.footer.other,
        );
        Self::render_tag(
            &mut spans,
            "Scroll",
            "Ctrl+↑↓ PgUp PgDn",
            state.settings.colors.footer.other,
        );

        // Show history position if navigating
        if let Some(index) = state.repl_history_index {
            if !state.repl_command_history.is_empty() {
                Self::render_tag(
                    &mut spans,
                    &format!("Pos {}/{}", index + 1, state.repl_command_history.len()),
                    "",
                    state.settings.colors.footer.command,
                );
            }
        } else if !state.repl_command_history.is_empty() {
            Self::render_tag(
                &mut spans,
                &format!("History: {}", state.repl_command_history.len()),
                "",
                state.settings.colors.footer.other,
            );
        }

        let line = Line::from(spans);
        Block::default()
            .title_style(Style::default().fg(Color::Black).bg(Color::LightGreen))
            .title(line)
    }

    pub fn render_repl_output<'a>(state: &'a TuiState) -> Paragraph<'a> {
        let visible_lines = state.visible_height.saturating_sub(2); // Account for footer
        let start_line = state.repl_scroll_offset;

        // Create a combined list of history + current input line
        let mut all_lines = state.repl_output_history.clone();

        // Add current input line with cursor
        let input_line = Self::render_repl_input_line(state);
        all_lines.push(input_line);

        let lines: Vec<Line> = all_lines
            .iter()
            .skip(start_line)
            .take(visible_lines)
            .map(|line| {
                if line.starts_with("> ") {
                    // Input line - style differently
                    Line::from(Span::styled(
                        line.clone(),
                        Style::default().fg(Color::Green).bold(),
                    ))
                } else if line.starts_with("Error: ") {
                    // Error line - style in red
                    Line::from(Span::styled(line.clone(), Style::default().fg(Color::Red)))
                } else {
                    // Output line - normal style
                    Line::from(Span::styled(
                        line.clone(),
                        Style::default().fg(Color::White),
                    ))
                }
            })
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Lua REPL Output")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: false });

        paragraph
    }

    fn render_repl_input_line(state: &TuiState) -> String {
        let input = &state.repl_input;
        let cursor_pos = state.text_edit_position;

        // Choose prompt based on multiline state
        let prompt = if state.repl_is_multiline {
            ">> " // Continuation prompt
        } else {
            "> " // Main prompt
        };

        // Create input line with cursor visualization
        let mut display_line = String::from(prompt);

        if input.is_empty() {
            // Show cursor at start when empty
            display_line.push('█'); // Block cursor
        } else {
            // Insert characters up to cursor position
            let chars: Vec<char> = input.chars().collect();
            for (i, &ch) in chars.iter().enumerate() {
                if i == cursor_pos {
                    display_line.push('█'); // Block cursor before this character
                }
                display_line.push(ch);
            }

            // If cursor is at the end, add it
            if cursor_pos >= chars.len() {
                display_line.push('█');
            }
        }

        display_line
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

    pub fn render_textinput_block<'a>(
        label: &'a str,
        value: &'a str,
        position: usize,
        style: Style,
    ) -> Block<'a> {
        let mut spans = vec![];
        let rstyle = reverse_style(style);

        spans.push(Span::styled(format!(" {} ", label), rstyle));

        // we split value in three, before cursor, cursor, after cursor
        let before_cursor = value.chars().take(position).collect::<String>();
        let cursor = value.chars().nth(position).unwrap_or(' ');
        let after_cursor = value.chars().skip(position + 1).collect::<String>();
        spans.push(Span::styled(" ", style));
        spans.push(Span::styled(before_cursor, style));
        spans.push(Span::styled(cursor.to_string(), rstyle));
        spans.push(Span::styled(after_cursor, style));
        spans.push(Span::styled(" ", style));

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
                state.position, // Already 1-based internally
                state.records.visible_records.len()
            )
            .as_str(),
            state.settings.colors.footer.line_number,
        );

        let right_line = Line::from(spans);

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

    fn refresh_screen(&mut self, _state: &TuiState) {
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
}

// drop impl
impl Drop for TuiChrome {
    fn drop(&mut self) {
        // restore terminal
        ratatui::restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_record_original_tab_and_colors() {
        let original = "\x1b[32mINFO\x1b[0m\tLog line\t\x1b[31m\tError\x1b[0m";
        let state = TuiState::new().unwrap();
        let record = record::Record::new(original.to_string());
        let line = TuiChrome::render_record_original(&state, &record);
        println!("line: {:?}", line);
        assert_eq!(line.spans.len(), 3);
        let line0 = line.spans.get(0).unwrap();
        let line1 = line.spans.get(1).unwrap();
        let line2 = line.spans.get(2).unwrap();
        assert_eq!(line0.content, "INFO");
        assert_eq!(line0.style.fg.unwrap(), Color::Green);
        assert_eq!(line1.content, "    Log line        ");
        assert!(line1.style.fg.is_none());
        assert_eq!(line2.content, "        Error");
        assert_eq!(line2.style.fg.unwrap(), Color::Red);
    }

    #[test]
    fn test_render_record_original_vscroll() {
        let original = "\x1b[32mINFO\x1b[0m\tLog line\t\x1b[31m\tError\x1b[0m";
        let mut state = TuiState::new().unwrap();
        let record = record::Record::new(original.to_string());
        let line = TuiChrome::render_record_original(&state, &record);
        println!("line: {:?}", line);
        let texts: Vec<Vec<&str>> = vec![
            vec!["INFO", "    Log line        ", "        Error"],
            vec!["NFO", "    Log line        ", "        Error"],
            vec!["FO", "    Log line        ", "        Error"],
            vec!["O", "    Log line        ", "        Error"],
            vec!["    Log line        ", "        Error"],
            vec!["   Log line        ", "        Error"],
            vec!["  Log line        ", "        Error"],
            vec![" Log line        ", "        Error"],
            vec!["Log line        ", "        Error"],
            vec!["og line        ", "        Error"],
            vec!["g line        ", "        Error"],
            vec![" line        ", "        Error"],
            vec!["line        ", "        Error"],
            vec!["ine        ", "        Error"],
            vec!["ne        ", "        Error"],
            vec!["e        ", "        Error"],
            vec!["        ", "        Error"],
            vec!["       ", "        Error"],
            vec!["      ", "        Error"],
            vec!["     ", "        Error"],
            vec!["    ", "        Error"],
            vec!["   ", "        Error"],
            vec!["  ", "        Error"],
            vec![" ", "        Error"],
            vec!["        Error"],
            vec!["       Error"],
            vec!["      Error"],
            vec!["     Error"],
            vec!["    Error"],
            vec!["   Error"],
            vec!["  Error"],
            vec![" Error"],
            vec!["Error"],
            vec!["rror"],
            vec!["ror"],
            vec!["or"],
            vec!["r"],
            vec![],
        ];
        for (i, text) in texts.iter().enumerate() {
            state.scroll_offset_left = i;
            let line = TuiChrome::render_record_original(&state, &record);
            println!("line: {:?}", line);
            assert_eq!(line.spans.len(), text.len());
            for (j, span) in line.spans.iter().enumerate() {
                assert_eq!(span.content, text[j]);
            }
        }
    }
}
