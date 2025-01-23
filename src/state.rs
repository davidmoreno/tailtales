use std::time;

use crate::{
    ast, recordlist,
    settings::{RulesSettings, Settings},
};

#[derive(PartialEq, Debug)]
pub enum Mode {
    Normal,
    Search,
    Filter,
    Command,
    Warning,
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
    pub command: String,
    pub warning: String,
}

impl TuiState {
    pub fn search_next(&mut self) {
        let current = self.position;
        self.set_position_wrap(self.position as i32 + 1);
        if !self.search_fwd() {
            // if not found, go back to the original position
            self.set_position(current);
        }
    }

    pub fn search_fwd(&mut self) -> bool {
        let search_ast = self.search_ast.as_ref();
        if search_ast.is_none() {
            return false;
        }
        let search_ast = search_ast.unwrap();
        let mut current = self.position;

        let maybe_position = self.records.search_forward(search_ast, current);
        if maybe_position.is_none() {
            return false;
        }
        current = maybe_position.unwrap();
        self.set_position(current);
        true
    }

    pub fn search_prev(&mut self) {
        let current = self.position;
        self.set_position_wrap(self.position as i32 - 1);
        if !self.search_bwd() {
            self.set_position(current);
        }
    }

    pub fn search_bwd(&mut self) -> bool {
        let search_ast = self.search_ast.as_ref();
        if search_ast.is_none() {
            return false;
        }
        let search_ast = search_ast.unwrap();
        let mut current = self.position;

        let maybe_position = self.records.search_backwards(search_ast, current);
        if maybe_position.is_none() {
            return false;
        }
        current = maybe_position.unwrap();
        self.set_position(current);
        true
    }

    pub fn handle_filter(&mut self) {
        let parsed = ast::parse(&self.filter);
        match parsed {
            Ok(parsed) => {
                self.records.filter_parallel(parsed);
                self.set_position(0);
            }
            Err(_err) => {
                // panic!("TODO show error parsing: {}", err);
            }
        }
    }
    pub fn handle_command(&mut self) {
        let command = self.command.trim();
        match command {
            "quit" => {
                self.running = false;
            }
            "clear" => {
                self.records.clear();
                self.position = 0;
                self.scroll_offset_top = 0;
                self.scroll_offset_left = 0;
            }
            "help" => {
                self.open_help();
            }
            "command" => {
                self.command = String::new();
                self.mode = Mode::Command;
            }
            "search" => {
                self.mode = Mode::Search;
            }
            "filter" => {
                self.mode = Mode::Filter;
            }
            "search_next" => {
                self.search_next();
            }
            "search_prev" => {
                self.search_prev();
            }
            "move_up" => {
                self.move_selection(-1);
            }
            "move_down" => {
                self.move_selection(1);
            }
            "move_left" => {
                self.set_vposition(self.scroll_offset_left as i32 - 1);
            }
            "move_right" => {
                self.set_vposition(self.scroll_offset_left as i32 + 1);
            }
            "move_pageup" => {
                self.move_selection(-10);
            }
            "move_pagedown" => {
                self.move_selection(10);
            }
            "move_top" => {
                self.set_position(0);
                self.set_vposition(0);
            }
            "move_bottom" => {
                self.set_position(usize::max_value());
            }
            "clear_records" => {
                self.records.clear();
                self.set_position(0);
                self.set_vposition(0);
            }
            "warning" => {
                self.set_warning(format!("Warning: {}", command));
            }
            "mark" => {
                self.records.mark(self.position);
                self.set_position_wrap(self.position as i32 + 1);
            }
            "move_to_next_mark" => {
                self.move_to_next_mark();
            }
            _ => {
                self.set_warning(format!("Unknown command: {}", command));
            }
        }
    }
    pub fn set_warning(&mut self, command: String) {
        self.warning = format!("Unknown command: {}", command);
        self.mode = Mode::Warning;
    }

    pub fn move_selection(&mut self, delta: i32) {
        // I use i32 all around here as I may get some negatives
        let mut current = self.position as i32;
        let mut new = current as i32 + delta;
        let max = self.records.visible_records.len() as i32 - 1;

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
        let visible_lines = self.total_visible_lines as i32;
        let current_i32 = current as i32;

        let mut scroll_offset = self.scroll_offset_top as i32;
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

        self.scroll_offset_top = scroll_offset as usize;
    }

    pub fn set_position(&mut self, position: usize) {
        let visible_len = self.records.visible_records.len();
        if visible_len == 0 {
            self.position = 0;
        } else if position >= visible_len {
            self.position = visible_len - 1;
        } else {
            self.position = position;
        }
        self.ensure_visible(self.position);
    }

    pub fn set_position_wrap(&mut self, position: i32) {
        let max = self.records.visible_records.len() as i32;
        if max <= 1 {
            self.position = 0
        } else if position >= max {
            self.position = 0;
        } else if position < 0 {
            self.position = (max - 1) as usize;
        } else {
            self.position = position as usize;
        }
        self.ensure_visible(self.position);
    }

    pub fn set_vposition(&mut self, position: i32) {
        if position < 0 {
            self.scroll_offset_left = 0;
        } else {
            self.scroll_offset_left = position as usize;
        }
    }

    pub fn open_help(&self) {
        let line = &self
            .records
            .visible_records
            .get(self.position)
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
            .arg(self.settings.help_url.replace("{}", &urlencodedline))
            .output()
            .expect("failed to execute process");
    }

    pub fn move_to_next_mark(&mut self) {
        let current = self.position;
        let mut new = current + 1;
        let max = self.records.visible_records.len();

        while new < max {
            if self.records.visible_records[new].get("mark").is_some() {
                self.set_position(new);
                return;
            }
            new += 1;
        }
        self.set_position(0);
    }
}
