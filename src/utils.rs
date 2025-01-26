use ratatui::style::{Color, Style, Stylize};

pub fn clean_ansi_text(orig: &str) -> String {
    // Lenght in real text, skips ANIS codes
    let mut text = String::new();
    let mut in_ansi_escape = false;
    for c in orig.chars() {
        if in_ansi_escape {
            if c == 'm' {
                in_ansi_escape = false;
            }
        } else if c == '\t' {
            // I need to put as many spaces to make for a multiple of 8 spaces
            let spaces = 8 - text.len() % 8;
            for _ in 0..spaces {
                text.push(' ');
            }
        } else if c == '\r' {
            // Ignore
        } else if c == 0o33 as char {
            in_ansi_escape = true;
        } else {
            text.push(c);
        }
    }
    text
}

pub fn ansi_to_style(prev_style: Style, ansi_code: &str) -> Style {
    // Copy the style an dmodify with the ansi_code changes
    let mut style = prev_style.clone();
    for code in ansi_code.split(';') {
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
            _ => style,
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
