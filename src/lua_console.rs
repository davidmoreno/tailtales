//! Lua Console Module
//!
//! This module provides the Lua console functionality for TailTales, including
//! console state management, text wrapping, and rendering.

use ratatui::prelude::*;
use ratatui::widgets::*;

/// Represents a line of output in the Lua console
#[derive(Debug, Clone)]
pub enum ConsoleLine {
    Stdout(String),
    Stderr(String),
}

/// Manages the Lua console state and functionality
pub struct LuaConsole {
    /// History of console output lines
    pub output_history: Vec<ConsoleLine>,
    /// Current scroll offset for viewing history
    pub scroll_offset: usize,
    /// Current input buffer
    pub input: String,
    /// Multiline input buffer
    pub multiline_buffer: Vec<String>,
    /// Whether currently in multiline input mode
    pub is_multiline: bool,
    /// Command history for navigation
    pub command_history: Vec<String>,
    /// Current position in command history
    pub history_index: Option<usize>,
    /// Temporary input storage during history navigation
    pub temp_input: String,
    /// Current cursor position in input
    pub text_edit_position: usize,
}

impl LuaConsole {
    /// Create a new Lua console instance
    pub fn new() -> Self {
        Self {
            output_history: Vec::new(),
            scroll_offset: 0,
            input: String::new(),
            multiline_buffer: Vec::new(),
            is_multiline: false,
            command_history: Vec::new(),
            history_index: None,
            temp_input: String::new(),
            text_edit_position: 0,
        }
    }

    /// Wrap text to fit within the specified width, splitting on word boundaries when possible
    pub fn wrap_text_to_width(&self, text: &str, max_width: usize) -> Vec<String> {
        if text.is_empty() {
            return vec![String::new()];
        }

        // If the text is already short enough, return as-is
        if text.len() <= max_width {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0;

        for word in text.split_whitespace() {
            let word_width = word.len();

            // If adding this word would exceed the width, start a new line
            if current_width + word_width + 1 > max_width && !current_line.is_empty() {
                lines.push(current_line.trim().to_string());
                current_line.clear();
                current_width = 0;
            }

            // Add the word to the current line
            if !current_line.is_empty() {
                current_line.push(' ');
                current_width += 1;
            }
            current_line.push_str(word);
            current_width += word_width;
        }

        // Add the last line if it's not empty
        if !current_line.is_empty() {
            lines.push(current_line.trim().to_string());
        }

        // If no lines were created (e.g., very long single word), force split
        if lines.is_empty() {
            lines.push(text.to_string());
        }

        lines
    }

    /// Add a stdout message to the console output history with text wrapping
    pub fn add_output(&mut self, message: String, visible_width: usize) {
        self.add_console_line(ConsoleLine::Stdout(message), visible_width);
    }

    /// Add a stderr message to the console output history with text wrapping
    pub fn add_error(&mut self, message: String, visible_width: usize) {
        self.add_console_line(ConsoleLine::Stderr(message), visible_width);
    }

    /// Internal function to add a console line with text wrapping
    fn add_console_line(&mut self, line: ConsoleLine, visible_width: usize) {
        let message = match &line {
            ConsoleLine::Stdout(msg) => msg.clone(),
            ConsoleLine::Stderr(msg) => msg.clone(),
        };

        let wrapped_lines = self.wrap_text_to_width(&message, visible_width.saturating_sub(4)); // Account for borders

        for wrapped_line in wrapped_lines {
            let console_line = match &line {
                ConsoleLine::Stdout(_) => ConsoleLine::Stdout(wrapped_line),
                ConsoleLine::Stderr(_) => ConsoleLine::Stderr(wrapped_line),
            };
            self.output_history.push(console_line);
        }

        // Limit the output history to prevent memory issues
        if self.output_history.len() > 1000 {
            self.output_history.drain(0..500);
        }
    }

    /// Add multiple error lines to the console output history
    #[allow(dead_code)]
    pub fn add_error_lines(&mut self, lines: Vec<String>, visible_width: usize) {
        for line in lines {
            self.add_error(line, visible_width);
        }
    }

    /// Initialize the console with welcome message if it's empty
    pub fn ensure_initialized(&mut self, visible_width: usize) {
        if self.output_history.is_empty() {
            self.add_output(
                "Welcome to Lua REPL! Type Lua code and press Enter to execute.".to_string(),
                visible_width,
            );
            self.add_output(
                "Supports multiline input: functions, if/do blocks, etc.".to_string(),
                visible_width,
            );
            self.add_output(
                "Use print() to output text, dir() to explore, help() for assistance.".to_string(),
                visible_width,
            );
            self.add_output(
                "Press Esc to exit, Ctrl+C to cancel multiline input.".to_string(),
                visible_width,
            );
            self.add_output(
                "Use ↑/↓ arrows to navigate command history, Ctrl+L to clear screen.".to_string(),
                visible_width,
            );
            self.add_output(
                "Press Tab for function/variable completion, Esc to exit.".to_string(),
                visible_width,
            );
            self.add_output("".to_string(), visible_width);
        }
    }

    /// Clear the console output history
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.scroll_offset = 0;
        self.output_history.clear();
    }

    /// Navigate console history up (older commands)
    pub fn history_up(&mut self) -> bool {
        match self.history_index {
            None => {
                // Start navigating from the end
                if !self.command_history.is_empty() {
                    self.temp_input = self.input.clone();
                    self.history_index = Some(self.command_history.len() - 1);
                    self.input = self.command_history[self.command_history.len() - 1].clone();
                    self.text_edit_position = self.input.len();
                    true
                } else {
                    false
                }
            }
            Some(index) => {
                if index > 0 {
                    self.history_index = Some(index - 1);
                    self.input = self.command_history[index - 1].clone();
                    self.text_edit_position = self.input.len();
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Navigate console history down (newer commands)
    pub fn history_down(&mut self) -> bool {
        match self.history_index {
            None => false,
            Some(index) => {
                if index < self.command_history.len() - 1 {
                    self.history_index = Some(index + 1);
                    self.input = self.command_history[index + 1].clone();
                    self.text_edit_position = self.input.len();
                    true
                } else {
                    // Back to current input
                    self.history_index = None;
                    self.input = self.temp_input.clone();
                    self.text_edit_position = self.input.len();
                    true
                }
            }
        }
    }

    /// Reset console history navigation
    pub fn reset_history_navigation(&mut self) {
        self.history_index = None;
        self.temp_input.clear();
    }

    /// Add a command to the command history
    pub fn add_to_history(&mut self, command: String) {
        // Don't add empty commands
        if command.trim().is_empty() {
            return;
        }

        // Don't add duplicate consecutive commands
        if self.command_history.last() != Some(&command) {
            self.command_history.push(command);
        }

        // Limit history size
        if self.command_history.len() > 100 {
            self.command_history.drain(0..50);
        }
    }

    /// Check if the current input is complete (balanced brackets, etc.)
    pub fn is_input_complete(&self) -> bool {
        let input = self.input.trim();
        if input.is_empty() {
            return false;
        }

        // Check for Lua-specific incomplete statements
        let input_lower = input.to_lowercase();

        // Check for incomplete if statements
        if input_lower.contains("if ") && !input_lower.contains(" then") {
            return false;
        }
        if input_lower.contains("if ")
            && input_lower.contains(" then")
            && !input_lower.contains(" end")
        {
            return false;
        }

        // Check for incomplete for loops
        if input_lower.contains("for ") && !input_lower.contains(" do") {
            return false;
        }
        if input_lower.contains("for ")
            && input_lower.contains(" do")
            && !input_lower.contains(" end")
        {
            return false;
        }

        // Check for incomplete while loops
        if input_lower.contains("while ") && !input_lower.contains(" do") {
            return false;
        }
        if input_lower.contains("while ")
            && input_lower.contains(" do")
            && !input_lower.contains(" end")
        {
            return false;
        }

        // Check for incomplete function definitions
        if input_lower.contains("function ") && !input_lower.contains(" end") {
            return false;
        }

        // Check for balanced brackets
        let mut paren_count = 0;
        let mut bracket_count = 0;
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for ch in input.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            if ch == '\\' && in_string {
                escape_next = true;
                continue;
            }

            if ch == '"' || ch == '\'' {
                in_string = !in_string;
                continue;
            }

            if !in_string {
                match ch {
                    '(' => paren_count += 1,
                    ')' => paren_count -= 1,
                    '[' => bracket_count += 1,
                    ']' => bracket_count -= 1,
                    '{' => brace_count += 1,
                    '}' => brace_count -= 1,
                    _ => {}
                }
            }
        }

        // Input is complete if all brackets are balanced
        paren_count == 0 && bracket_count == 0 && brace_count == 0
    }

    /// Scroll the console output up
    #[allow(dead_code)]
    pub fn scroll_up(&mut self, visible_height: usize) {
        let visible_lines = visible_height.saturating_sub(2);
        let total_lines = self.output_history.len() + 1; // +1 for current input line
        if total_lines > visible_lines
            && self.scroll_offset < total_lines.saturating_sub(visible_lines)
        {
            self.scroll_offset += 1;
        }
    }

    /// Scroll the console output down
    #[allow(dead_code)]
    pub fn scroll_down(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll to the top of the console output
    #[allow(dead_code)]
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll to the bottom of the console output
    #[allow(dead_code)]
    pub fn scroll_to_bottom(&mut self, visible_height: usize) {
        let visible_lines = visible_height.saturating_sub(2);
        let total_lines = self.output_history.len() + 1; // +1 for current input line
        if total_lines > visible_lines {
            self.scroll_offset = total_lines.saturating_sub(visible_lines);
        }
    }

    /// Page up in the console output
    #[allow(dead_code)]
    pub fn page_up(&mut self, visible_height: usize) {
        let visible_lines = visible_height.saturating_sub(2);
        for _ in 0..visible_lines {
            self.scroll_up(visible_height);
        }
    }

    /// Page down in the console output
    #[allow(dead_code)]
    pub fn page_down(&mut self, visible_height: usize) {
        let visible_lines = visible_height.saturating_sub(2);
        for _ in 0..visible_lines {
            self.scroll_down();
        }
    }
}

/// Render the Lua console output
pub fn render_console_output<'a>(console: &'a LuaConsole, visible_height: usize) -> Paragraph<'a> {
    let visible_lines = visible_height.saturating_sub(2); // Account for footer
    let start_line = console.scroll_offset;

    // Create a combined list of history lines
    let mut lines: Vec<Line> = console
        .output_history
        .iter()
        .skip(start_line)
        .take(visible_lines)
        .map(|console_line| {
            match console_line {
                ConsoleLine::Stdout(msg) => {
                    if msg.starts_with("> ") {
                        // Input line - style differently
                        Line::from(Span::styled(
                            msg.clone(),
                            Style::default().fg(Color::Green).bold(),
                        ))
                    } else {
                        // Output line - normal style
                        Line::from(Span::styled(msg.clone(), Style::default().fg(Color::White)))
                    }
                }
                ConsoleLine::Stderr(msg) => {
                    // Error line - style in red
                    Line::from(Span::styled(msg.clone(), Style::default().fg(Color::Red)))
                }
            }
        })
        .collect();

    // Add current input line with cursor (only if we have space)
    if lines.len() < visible_lines {
        let input_line = render_console_input_line(console);
        lines.push(input_line);
    }

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

/// Render the console input line with cursor
fn render_console_input_line(console: &LuaConsole) -> Line {
    let input = &console.input;
    let cursor_pos = console.text_edit_position;

    // Choose prompt based on multiline state
    let prompt = if console.is_multiline {
        ">> " // Continuation prompt
    } else {
        "> " // Main prompt
    };

    // Create spans for the input line with cursor visualization
    let mut spans = vec![Span::styled(
        prompt.to_string(),
        Style::default().fg(Color::Green).bold(),
    )];

    if input.is_empty() {
        // Show cursor at start when empty - use a space with reversed style
        spans.push(Span::styled(
            " ".to_string(),
            Style::default().fg(Color::Black).bg(Color::Green),
        ));
    } else {
        // Insert characters with cursor reversal
        let chars: Vec<char> = input.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            if i == cursor_pos {
                // Reverse the character under cursor
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(Color::Black).bg(Color::Green),
                ));
            } else {
                // Normal character
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(Color::Green).bold(),
                ));
            }
        }

        // If cursor is at the end, add a reversed space
        if cursor_pos >= chars.len() {
            spans.push(Span::styled(
                " ".to_string(),
                Style::default().fg(Color::Black).bg(Color::Green),
            ));
        }
    }

    Line::from(spans)
}

/// Render the console footer using proper color scheme
pub fn render_console_footer<'a>(
    console: &LuaConsole,
    settings: &crate::settings::Settings,
) -> Block<'a> {
    // Simple footer for REPL mode - no input field here since input is shown inline
    let mut spans = vec![];

    if console.is_multiline {
        render_tag(
            &mut spans,
            "Multiline Mode",
            "Ctrl+C to cancel",
            settings.colors.footer.command,
        );
    }

    render_tag(
        &mut spans,
        "Lua REPL",
        "ESC to exit",
        settings.colors.footer.other,
    );
    render_tag(
        &mut spans,
        "History",
        "↑↓ arrows",
        settings.colors.footer.other,
    );
    render_tag(
        &mut spans,
        "Scroll",
        "Ctrl+↑↓ PgUp PgDn",
        settings.colors.footer.other,
    );

    // Show history position if navigating
    if let Some(index) = console.history_index {
        if !console.command_history.is_empty() {
            render_tag(
                &mut spans,
                "Pos",
                &format!("{}/{}", index + 1, console.command_history.len()),
                settings.colors.footer.command,
            );
        }
    } else if !console.command_history.is_empty() {
        render_tag(
            &mut spans,
            "History",
            &format!("{}", console.command_history.len()),
            settings.colors.footer.other,
        );
    }

    let line = Line::from(spans);
    Block::default()
        .title_style(Style::default().fg(Color::Black).bg(Color::LightGreen))
        .title(line)
}

/// Helper function to render footer tags with proper styling
fn render_tag(spans: &mut Vec<Span>, label: &str, value: &str, style: Style) {
    use crate::utils::reverse_style;

    let rstyle = reverse_style(style);

    spans.push(Span::styled(format!(" {} ", label), rstyle));
    spans.push(Span::styled(format!(" {} ", value), style));
    spans.push(Span::styled(" ".to_string(), rstyle));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_to_width() {
        let console = LuaConsole::new();

        // Test short text (should not wrap)
        let short_text = "Hello world";
        let wrapped = console.wrap_text_to_width(short_text, 76); // 80 - 4 for borders
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "Hello world");

        // Test long text (should wrap)
        let long_text = "This is a very long line of text that should definitely wrap when the width is limited to a reasonable size for console output";
        let wrapped = console.wrap_text_to_width(long_text, 40);
        assert!(wrapped.len() > 1);

        // All wrapped lines should be within the width limit
        for line in &wrapped {
            assert!(
                line.len() <= 40,
                "Line '{}' is too long: {} chars",
                line,
                line.len()
            );
        }

        // Test empty text
        let empty_wrapped = console.wrap_text_to_width("", 40);
        assert_eq!(empty_wrapped.len(), 1);
        assert_eq!(empty_wrapped[0], "");

        // Test very long single word (should not break word)
        let long_word = "supercalifragilisticexpialidocious";
        let wrapped_word = console.wrap_text_to_width(long_word, 20);
        assert_eq!(wrapped_word.len(), 1);
        assert_eq!(wrapped_word[0], long_word);
    }

    #[test]
    fn test_add_output_with_wrapping() {
        let mut console = LuaConsole::new();
        let visible_width = 50; // Set a small width for testing

        // Add a long message
        let long_message = "This is a very long error message that should be wrapped across multiple lines when added to the Lua console output history";
        console.add_output(long_message.to_string(), visible_width);

        // Should have multiple lines in history
        assert!(console.output_history.len() > 1);

        // All lines should be within the width limit
        for console_line in &console.output_history {
            let line_len = match console_line {
                ConsoleLine::Stdout(msg) => msg.len(),
                ConsoleLine::Stderr(msg) => msg.len(),
            };
            assert!(
                line_len <= 46,
                "Line '{:?}' is too long: {} chars",
                console_line,
                line_len
            ); // 50 - 4 for borders
        }
    }

    #[test]
    fn test_input_completion() {
        let mut console = LuaConsole::new();

        // Test incomplete input
        console.input = "if true then".to_string();
        assert!(!console.is_input_complete());

        // Test complete input
        console.input = "if true then end".to_string();
        assert!(console.is_input_complete());

        // Test with nested brackets
        console.input = "local t = { a = 1, b = { c = 2 } }".to_string();
        assert!(console.is_input_complete());

        // Test with strings
        console.input = r#"print("hello world")"#.to_string();
        assert!(console.is_input_complete());
    }
}
