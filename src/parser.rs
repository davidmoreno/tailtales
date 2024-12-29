use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Parser {
    regex: regex::Regex,
    is_logfmt: bool,
}

impl Parser {
    pub fn new_from_pattern(linepattern: &str) -> Parser {
        // The linepatter lang is <_> equals .*
        // <name> is a named group "name"
        // Any other thing is to be matched exactly, including spaces, text and symbols.

        let mut repattern = "^".to_string();
        let mut inpattern: bool = false;
        let mut patternname = String::new();
        for c in linepattern.chars() {
            if c == '<' {
                inpattern = true;
                patternname.clear();
                continue;
            }
            if c == '>' {
                inpattern = false;
                if patternname != "_" && patternname != "" {
                    repattern.push_str("(?P<");
                    repattern.push_str(&patternname);
                    repattern.push_str(">");
                    repattern.push_str(".*?");
                    repattern.push_str(")");
                } else {
                    repattern.push_str("(");
                    repattern.push_str(".*?");
                    repattern.push_str(")");
                }
                continue;
            }
            if inpattern {
                if is_special_for_re(c) {
                    repattern.push('\\');
                }
                patternname.push(c);
            } else {
                if is_special_for_re(c) {
                    repattern.push('\\');
                }
                repattern.push(c);
            }
        }
        repattern.push_str("$");
        let re = regex::Regex::new(&repattern).unwrap();
        Parser {
            regex: re,
            is_logfmt: false,
        }
    }

    pub fn new_logfmt() -> Parser {
        let re = regex::Regex::new(r"(?P<key>[^ ]*?)=(?P<value>[^ ]*)( |$)").unwrap();
        Parser {
            regex: re,
            is_logfmt: true,
        }
    }

    pub fn new_from_regex(regex: &str) -> Parser {
        let re = regex::Regex::new(regex).unwrap();
        Parser {
            regex: re,
            is_logfmt: false,
        }
    }

    pub fn parse_line(&self, line: &str) -> HashMap<String, String> {
        if self.is_logfmt {
            return self.parse_logfmt(line);
        } else {
            return self.parse_regex(line);
        }
    }

    fn parse_logfmt(&self, line: &str) -> HashMap<String, String> {
        let mut data = HashMap::new();
        for caps in self.regex.captures_iter(line) {
            let key = caps["key"].to_string();
            let value = caps["value"].to_string();
            data.insert(key, value);
        }
        data
    }

    fn parse_regex(&self, line: &str) -> HashMap<String, String> {
        let mut data = HashMap::new();
        let caps = self.regex.captures(line);
        match caps {
            Some(caps) => {
                for name in self.regex.capture_names().flatten() {
                    let value = caps[name].to_string();
                    data.insert(name.to_string(), value);
                }
            }
            None => {}
        }
        data
    }
}

fn is_special_for_re(c: char) -> bool {
    match c {
        '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => true,
        _ => false,
    }
}
