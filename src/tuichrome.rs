use crate::record;

use crossterm::{
    event::DisableMouseCapture,
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
use std::io;

pub struct TuiState {
    pub records: record::RecordList,
    pub tab_index: usize,
    pub selected_record: Option<usize>,
    pub scroll_offset: usize,
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
                tab_index: 0,
                selected_record: Option::Some(10),
                scroll_offset: 0,
            },
            terminal: terminal,
        })
    }

    pub fn draw(&mut self) -> Result<(), io::Error> {
        let size = self.terminal.size()?;
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
            state.records.records[start..start + height]
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

                    if state.selected_record.is_some()
                        && state.selected_record.unwrap() == current_index
                    {
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
