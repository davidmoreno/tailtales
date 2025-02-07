use std::time;

use crate::{
    ast, recordlist,
    settings::{RulesSettings, Settings},
};

#[derive(PartialEq, Debug, Clone, Copy)]
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
    pub next_mode: Mode,
    pub search: String,
    pub search_ast: Option<ast::AST>,
    pub filter: String,
    pub filter_ok: bool,
    pub command: String,
    pub warning: String,
    pub view_details: bool,
    pub text_edit_position: usize,
}

impl TuiState {
    pub fn new() -> TuiState {
        TuiState {
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
            next_mode: Mode::Normal,
            search: String::new(),
            search_ast: None,
            filter: String::new(),
            filter_ok: true,
            command: String::new(),
            warning: String::new(),
            view_details: false,
            text_edit_position: 0,
        }
    }

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
                self.filter_ok = true;
            }
            Err(_err) => {
                self.filter_ok = false;
                // panic!("TODO show error parsing: {}", err);
            }
        }
    }
    pub fn handle_command(&mut self) {
        let mut args = self.command.split_whitespace();
        let command = args.next().unwrap_or("");

        match command {
            "" => {}
            "quit" => {
                self.running = false;
            }
            "clear" => {
                self.records.clear();
                self.position = 0;
                self.scroll_offset_top = 0;
                self.scroll_offset_left = 0;
            }
            "open_help" => {
                self.open_help();
            }
            "open_url" => {
                let args_vec: Vec<String> = args.map(String::from).collect();
                let url = args_vec.get(0);
                if let Some(url) = url {
                    self.open_url(url);
                } else {
                    self.set_warning("No URL provided".into());
                }
            }
            "command" => {
                self.command = String::new();
                self.mode = Mode::Command;
            }
            "search_next" => {
                self.search_next();
            }
            "search_prev" => {
                self.search_prev();
            }
            "vmove" => {
                let args: Vec<String> = args.map(String::from).collect();
                let def_arg1 = "1".to_string();
                let position = args.get(0).unwrap_or(&def_arg1);

                self.move_selection(position.parse::<i32>().unwrap());
            }
            "vgoto" => {
                let args: Vec<String> = args.map(String::from).collect();
                let def_arg1 = "0".to_string();
                let position = args.get(0).unwrap_or(&def_arg1);

                self.set_position(position.parse::<usize>().unwrap());
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
                let args_vec: Vec<String> = args.map(String::from).collect();
                let message = args_vec.join(" ");
                self.set_warning(message);
            }
            "toggle_mark" => {
                let default_color = "yellow".to_string();
                let args_vec: Vec<String> = args.map(String::from).collect();
                let color = args_vec.get(0).unwrap_or(&default_color);
                self.toggle_mark(color);
            }
            "move_to_next_mark" => {
                self.move_to_next_mark();
            }
            "move_to_prev_mark" => {
                self.move_to_prev_mark();
            }
            "settings" => {
                self.open_settings();
            }
            "mode" => {
                let args: Vec<String> = args.map(String::from).collect();
                self.set_mode(args.get(0).unwrap_or(&"normal".to_string()));
            }
            "toggle_details" => {
                self.view_details = !self.view_details;
            }
            "hmove" => {
                let args_vec: Vec<String> = args.map(String::from).collect();
                let def_arg1 = "1".to_string();
                let position = args_vec.get(0).unwrap_or(&def_arg1);
                self.set_vposition(
                    self.scroll_offset_left as i32 + position.parse::<i32>().unwrap(),
                );
            }
            _ => {
                self.set_warning(format!("Unknown command: {}", command));
            }
        }
    }
    pub fn set_warning(&mut self, warning: String) {
        self.warning = warning;
        self.mode = Mode::Warning;
    }

    pub fn set_mode(&mut self, mode: &str) {
        match mode {
            "normal" => {
                self.mode = Mode::Normal;
            }
            "search" => {
                self.mode = Mode::Search;
            }
            "filter" => {
                self.mode = Mode::Filter;
            }
            "command" => {
                self.mode = Mode::Command;
            }
            _ => {
                self.set_warning(format!("Unknown mode: {}", mode));
            }
        }
    }

    pub fn toggle_mark(&mut self, color: &str) {
        let color = color.to_string();
        let current = self.position;
        let record = self.records.visible_records.get_mut(current).unwrap();
        let current_value = record.get("mark");
        if current_value.is_some() {
            if *current_value.unwrap() == color {
                record.unset_data("mark");
            } else {
                record.set_data("mark", color);
            }
        } else {
            record.set_data("mark", color);
        }
        self.set_position_wrap(self.position as i32 + 1);
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
        self.open_url(&self.settings.help_url.replace("{}", &urlencodedline));
    }

    pub fn open_url(&self, url: &str) {
        let _output = std::process::Command::new("xdg-open")
            .arg(url)
            .output()
            .expect("failed to execute process");
    }

    pub fn move_to_next_mark(&mut self) {
        let current = self.position;
        let max = self.records.visible_records.len();

        for new in current + 1..max {
            if self.records.visible_records[new].get("mark").is_some() {
                self.set_position(new);
                return;
            }
        }
        for new in 0..current {
            if self.records.visible_records[new].get("mark").is_some() {
                self.set_position(new);
                return;
            }
        }
        self.set_warning("mark not found".into());
    }

    pub fn move_to_prev_mark(&mut self) {
        let current = self.position;
        let max = self.records.visible_records.len();

        for new in (0..current).rev() {
            if self.records.visible_records[new].get("mark").is_some() {
                self.set_position(new);
                return;
            }
        }
        for new in (current + 1..max).rev() {
            if self.records.visible_records[new].get("mark").is_some() {
                self.set_position(new);
                return;
            }
        }
        self.set_warning("mark not found".into());
    }

    pub fn open_settings(&mut self) {
        let filename = Settings::local_settings_filename();

        let filename = if filename.is_none() {
            // Create a settings file
            let _ = self.settings.save_default_settings();

            Settings::local_settings_filename().unwrap()
        } else {
            filename.unwrap()
        };

        // Open the file with xdg-open, using this same terminal, and wait for it to finish.
        // stdin, stderr and stdout are inherited from the parent process.
        let _output = std::process::Command::new("xdg-open")
            .arg(filename.to_str().unwrap())
            .spawn()
            .expect("failed to execute process")
            .wait()
            .expect("failed to wait for process");
    }

    pub fn get_completions(&self) -> (String, Vec<String>) {
        let current = self.command.trim();
        let mut completions: Vec<&str> = vec![
            "command",
            "quit",
            "clear",
            "open_help",
            "open_url",
            "search_next",
            "search_prev",
            "vmove",
            "hmove",
            "vgoto",
            "clear_records",
            "warning",
            "toggle_mark",
            "move_to_next_mark",
            "move_to_prev_mark",
            "settings",
            "mode",
            "toggle_details",
        ];

        completions.retain(|&c| c.starts_with(current));
        let completions: Vec<String> = completions.iter().map(|&s| s.to_string()).collect();

        // Find common prefix
        let mut common_prefix = String::new();
        if completions.len() == 1 {
            common_prefix = completions[0].to_string();
        } else if completions.len() > 1 {
            let mut pos = 0;
            let mut done = false;

            while !done {
                let mut c: Option<char> = None;
                for completion in &completions {
                    if pos >= completion.len() {
                        done = true;
                        break;
                    }
                    let current_char = completion.chars().nth(pos).unwrap();
                    if c.is_none() {
                        c = Some(current_char);
                    } else if c.unwrap() != current_char {
                        done = true;
                        break;
                    }
                }
                if !done {
                    common_prefix.push(c.unwrap());
                    pos += 1;
                }
            }
        }
        return (common_prefix, completions);
    }
}
