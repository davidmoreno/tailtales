use ratatui::style::{Color, Style, Stylize};

const TAB_SIZE: usize = 8;

pub fn clean_ansi_text(orig: &str) -> String {
    // Lenght in real text, skips ANIS codes
    let mut text = String::new();
    // println!("orig: {}", orig);
    let mut in_ansi_escape = false;
    let mut pos = 0;
    for c in orig.chars() {
        if in_ansi_escape {
            if c == 'm' {
                in_ansi_escape = false;
            }
        } else if c == '\t' {
            // I need to put as many spaces to make for a multiple of 8 spaces
            let mut spaces = TAB_SIZE - pos % TAB_SIZE;
            if spaces == 0 {
                spaces = TAB_SIZE;
            }
            for _ in 0..spaces {
                text.push(' ');
                pos += 1;
            }
        } else if c == '\r' {
            // Ignore
        } else if c == 0o33 as char {
            in_ansi_escape = true;
        } else {
            text.push(c);
            pos += 1;
        }
    }
    // println!("text: {}", text);
    text
}

pub fn ansi_to_style(prev_style: Style, ansi_code: &str) -> Style {
    // Copy the style an dmodify with the ansi_code changes
    let mut style = prev_style.clone();
    let split_codes = ansi_code
        .trim_start_matches('\x1b')
        .trim_start_matches('[')
        .split(';');
    for code in split_codes {
        style = match code {
            "0" => Style::default(),
            "1" => style.bold(),
            "3" => style.italic(),
            "4" => style.underlined(),
            "30" => style.fg(Color::Black),
            "31" => style.fg(Color::Red),
            "32" => style.fg(Color::Green),
            "33" => style.fg(Color::Yellow),
            "34" => style.fg(Color::Blue),
            "35" => style.fg(Color::Magenta),
            "36" => style.fg(Color::Cyan),
            "37" => style.fg(Color::White),
            "40" => style.bg(Color::Black),
            "41" => style.bg(Color::Red),
            "42" => style.bg(Color::Green),
            "43" => style.bg(Color::Yellow),
            "44" => style.bg(Color::Blue),
            "45" => style.bg(Color::Magenta),
            "46" => style.bg(Color::Cyan),
            "47" => style.bg(Color::White),
            _ => {
                // println!("unknown ansi code: {:?}", code);
                style
            }
        };
    }
    style
}

pub fn reverse_style(style: Style) -> Style {
    let mut new_style = style.clone();
    if let Some(fg) = style.fg {
        new_style = new_style.bg(fg);
    }
    if let Some(bg) = style.bg {
        new_style = new_style.fg(bg);
    }
    new_style
}

pub fn parse_tabs(text: &str) -> String {
    // check if has tabs, and not just return the string
    if !text.contains('\t') {
        return text.to_string();
    }
    // println!("pre: {:?}", text);
    let mut new_text = String::new();
    let mut pos = 0;
    let mut in_ansi_escape = false;
    for c in text.chars() {
        // ansi is ignored for stop points, but added to the text
        if in_ansi_escape {
            new_text.push(c);
            if c == 'm' {
                in_ansi_escape = false;
            }
        } else if c == 0o33 as char {
            // inside ansi, add text
            new_text.push(c);
            in_ansi_escape = true;
        } else if c == '\t' {
            // tabs are treated as spaces with TAB_SIZE stop points
            let mut count = TAB_SIZE - pos % TAB_SIZE;
            if count == 0 {
                count = TAB_SIZE;
            }
            for _ in 0..count {
                new_text.push(' ');
                pos += 1;
            }
        } else {
            // normal characters are just added
            pos += c.len_utf8();
            new_text.push(c);
        }
    }
    // println!("post: {:?}", new_text);
    new_text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tabs() {
        assert_eq!(parse_tabs("test\t"), "test    ");
        assert_eq!(parse_tabs("test\tx"), "test    x");
        assert_eq!(parse_tabs("test\tx\t\tx"), "test    x               x");
        assert_eq!(
            parse_tabs("\ttest\tx\t\tx"),
            "        test    x               x"
        );
    }
    #[test]
    fn test_clean_ansi_text() {
        assert_eq!(
            clean_ansi_text("\x1b[32mINFO\x1b[0m\tLog line\t\x1b[31mError\x1b[0m"),
            "INFO    Log line        Error"
        );
    }
}
