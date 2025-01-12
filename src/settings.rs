use ratatui::style::Style;
use serde::{de::Deserializer, Deserialize};
use std::str::FromStr;

// singleton load settings
lazy_static::lazy_static! {
    pub static ref SETTINGS: Settings = Settings::read_from_yaml("settings.yaml").unwrap();
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub global: GlobalSettings,
    pub rules: Vec<RulesSettings>,
}

#[derive(Debug, Deserialize)]
pub struct GlobalSettings {
    pub reload_on_truncate: bool,
    pub colors: GlobalColorSettings,
}

#[derive(Debug, Deserialize)]
pub struct GlobalColorSettings {
    #[serde(deserialize_with = "parse_color_pair")]
    pub normal: ColorPair,
    #[serde(deserialize_with = "parse_color_pair")]
    pub highlight: ColorPair,
    pub details: DetailsColorSettings,
}

#[derive(Debug, Deserialize)]
pub struct DetailsColorSettings {
    #[serde(deserialize_with = "parse_color_pair")]
    pub title: ColorPair,
    #[serde(deserialize_with = "parse_color_pair")]
    pub key: ColorPair,
    #[serde(deserialize_with = "parse_color_pair")]
    pub value: ColorPair,
    #[serde(deserialize_with = "parse_color_pair")]
    pub border: ColorPair,
}

// All basic ANSI terminal colors
#[derive(Debug, Deserialize, Default, PartialEq, Clone, Copy)]
pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
    Cyan,
    Magenta,
    White,
    Black,
    #[default]
    Unknown,
}

#[derive(Debug, Deserialize, Default, PartialEq)]
pub enum Alignment {
    #[default]
    Left,
    Right,
    Center,
}

#[derive(Debug, Deserialize)]
pub struct RulesSettings {
    pub name: String,
    #[serde(default)]
    pub file_patterns: Vec<String>,
    #[serde(default)]
    pub extractors: Vec<String>,
    #[serde(default)]
    pub filters: Vec<FilterSettings>,
    #[serde(default)]
    pub columns: Vec<ColumnSettings>,
}

#[derive(Debug, Deserialize)]
pub struct FilterSettings {
    #[serde(default)]
    pub name: String,
    pub expression: String,
    #[serde(default, deserialize_with = "parse_color_pair")]
    pub highlight: ColorPair,
    #[serde(default, deserialize_with = "parse_color")]
    pub gutter: Option<Color>,
}

impl FromStr for Color {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "white" => Ok(Color::White),
            "red" => Ok(Color::Red),
            "green" => Ok(Color::Green),
            "blue" => Ok(Color::Blue),
            "yellow" => Ok(Color::Yellow),
            "cyan" => Ok(Color::Cyan),
            "magenta" => Ok(Color::Magenta),
            "black" => Ok(Color::Black),
            _ => Err(()),
        }
    }
}

fn parse_color_pair<'de, D>(deserializer: D) -> Result<ColorPair, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let mut parts = s.split_whitespace();
    let first = parts
        .next()
        .ok_or_else(|| serde::de::Error::custom("Missing first color"))?;
    let second = parts
        .next()
        .ok_or_else(|| serde::de::Error::custom("Missing second color"))?;

    let first_color =
        Color::from_str(first).map_err(|_| serde::de::Error::custom("Invalid color"))?;
    let second_color =
        Color::from_str(second).map_err(|_| serde::de::Error::custom("Invalid color"))?;

    Ok(ColorPair {
        fg: first_color,
        bg: second_color,
    })
}

fn parse_color<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Deserialize::deserialize(deserializer)?;
    match s {
        Some(s) => {
            let color =
                Color::from_str(&s).map_err(|_| serde::de::Error::custom("Invalid color"))?;
            Ok(Some(color))
        }
        None => Ok(None),
    }
}

#[derive(Debug, Deserialize)]
pub struct ColumnSettings {
    pub name: String,
    pub width: usize,
    #[serde(default, deserialize_with = "parse_alignment")]
    pub align: Alignment,
}

impl FromStr for Alignment {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "left" => Ok(Alignment::Left),
            "right" => Ok(Alignment::Right),
            "center" => Ok(Alignment::Center),
            _ => Err(()),
        }
    }
}

fn parse_alignment<'de, D>(deserializer: D) -> Result<Alignment, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Alignment::from_str(&s).map_err(|_| serde::de::Error::custom("Invalid alignment"))
}

impl Settings {
    // pub fn new() -> Self {
    //     Settings {
    //         global: GlobalSettings {
    //             reload_on_truncate: false,
    //             colors: GlobalColorSettings {
    //                 normal: ColorPair {
    //                     fg: Color::White,
    //                     bg: Color::Black,
    //                 },
    //                 highlight: ColorPair {
    //                     fg: Color::Yellow,
    //                     bg: Color::Black,
    //                 },
    //                 details: DetailsColorSettings {
    //                     title: ColorPair {
    //                         fg: Color::White,
    //                         bg: Color::Black,
    //                     },
    //                     key: ColorPair {
    //                         fg: Color::White,
    //                         bg: Color::Black,
    //                     },
    //                     value: ColorPair {
    //                         fg: Color::White,
    //                         bg: Color::Black,
    //                     },
    //                     border: ColorPair {
    //                         fg: Color::White,
    //                         bg: Color::Black,
    //                     },
    //                 },
    //             },
    //         },
    //         rules: Vec::new(),
    //     }
    // }

    pub fn read_from_yaml(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(filename)?;
        let reader = std::io::BufReader::new(file);
        let settings: Settings = serde_yaml::from_reader(reader)?;
        Ok(settings)
    }
}

// An .into from (settings::Color, settings::Color) to crossterm::style::Color
#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct ColorPair {
    #[serde(default)]
    pub fg: Color,
    #[serde(default)]
    pub bg: Color,
}

impl From<ColorPair> for Style {
    fn from(pair: ColorPair) -> Self {
        Style::default().fg(pair.fg.into()).bg(pair.bg.into())
    }
}

// Color to crossterm::style::Color
impl From<Color> for ratatui::prelude::Color {
    fn from(color: Color) -> Self {
        match color {
            Color::Red => ratatui::prelude::Color::Red,
            Color::Green => ratatui::prelude::Color::Green,
            Color::Blue => ratatui::prelude::Color::Blue,
            Color::Yellow => ratatui::prelude::Color::Yellow,
            Color::Cyan => ratatui::prelude::Color::Cyan,
            Color::Magenta => ratatui::prelude::Color::Magenta,
            Color::White => ratatui::prelude::Color::White,
            Color::Black => ratatui::prelude::Color::Black,
            Color::Unknown => ratatui::prelude::Color::Reset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_parse_color_pair() {
    //     let input = "white black";
    //     let result = parse_color_pair(input.into()).unwrap();
    //     assert_eq!(result, (Color::White, Color::Black));
    // }

    // #[test]
    // fn test_parse_color() {
    //     let input = "white";
    //     let result = parse_color(input.into()).unwrap();
    //     assert_eq!(result, Some(Color::White));
    // }

    // #[test]
    // fn test_parse_alignment() {
    //     let input = "left";
    //     let result = parse_alignment(input.into()).unwrap();
    //     assert_eq!(result, Alignment::Left);
    // }

    #[test]
    fn test_parse_settings() {
        let settings = Settings::read_from_yaml("settings.yaml").unwrap();
        println!("{:#?}", settings);
    }
}
