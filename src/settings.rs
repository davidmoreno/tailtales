use ratatui::style::{Color, Style};
use serde::{de::Deserializer, Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, str::FromStr};

use crate::ast;

// singleton load settings

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub global: GlobalSettings,
    #[serde(default)]
    pub rules: Vec<RulesSettings>,
    #[serde(default)]
    pub default_arguments: Vec<String>,
    #[serde(default)]
    pub keybindings: HashMap<String, String>,
    #[serde(default)]
    pub colors: GlobalColorSettings,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SettingsFromYaml {
    #[serde(default)]
    pub global: Option<GlobalSettings>,
    #[serde(default)]
    pub rules: Vec<RulesSettings>,
    #[serde(default)]
    pub default_arguments: Vec<String>,
    #[serde(default)]
    pub keybindings: Option<HashMap<String, String>>,
    #[serde(default)]
    pub colors: Option<GlobalColorSettings>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GlobalSettings {
    // pub reload_on_truncate: bool,
    pub gutter_symbol: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GlobalColorSettings {
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub normal: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub highlight: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub mark: Style,
    #[serde(
        deserialize_with = "parse_style",
        serialize_with = "serialize_style",
        default = "default_highlight"
    )]
    pub mark_highlight: Style,
    pub details: DetailsColorSettings,
    pub table: TableColorSettings,
    pub footer: FooterColorSettings,
}

fn default_highlight() -> Style {
    Style::new().fg(Color::White).bg(Color::Black)
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FooterColorSettings {
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub command: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub filter: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub search: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub version: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub rule: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub line_number: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub other: Style,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TableColorSettings {
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub header: Style,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DetailsColorSettings {
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub title: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub key: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub value: Style,
    #[serde(deserialize_with = "parse_style", serialize_with = "serialize_style")]
    pub border: Style,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
pub enum Alignment {
    #[default]
    Left,
    Right,
    Center,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct FilterSettings {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "parse_expression",
        serialize_with = "serialize_expression"
    )]
    pub expression: ast::AST,
    #[serde(
        default,
        deserialize_with = "parse_optional_style",
        serialize_with = "serialize_optional_style"
    )]
    pub highlight: Option<Style>,
    #[serde(
        default,
        deserialize_with = "parse_optional_style",
        serialize_with = "serialize_optional_style"
    )]
    pub gutter: Option<Style>,
}

fn parse_expression<'de, D>(deserializer: D) -> Result<ast::AST, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    ast::AST::from_str(&s).map_err(serde::de::Error::custom)
}

fn serialize_expression<S>(expression: &ast::AST, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&expression.to_string())
}

fn parse_style<'de, D>(deserializer: D) -> Result<Style, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    string_to_style(&s).map_err(serde::de::Error::custom)
}

pub fn string_to_style(s: &str) -> Result<Style, String> {
    let mut parts = s.split_whitespace();
    let first = parts.next().ok_or_else(|| "Missing first color")?;
    let first_color = Color::from_str(first).map_err(|_| "Invalid color")?;
    let style = Style::new().fg(first_color);

    // optional second, default black
    let style = match parts.next() {
        Some(second) => {
            let second_color = Color::from_str(second).map_err(|_| "Invalid color")?;
            style.bg(second_color)
        }
        None => style,
    };

    Ok(style)
}

fn serialize_style<S>(style: &Style, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut s = String::new();
    if let Some(fg) = style.fg {
        s.push_str(&fg.to_string());
        if let Some(bg) = style.bg {
            s.push(' ');
            s.push_str(&bg.to_string());
        }
    }
    serializer.serialize_str(&s)
}

fn parse_optional_style<'de, D>(deserializer: D) -> Result<Option<Style>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(None)
    } else {
        let style = string_to_style(&s).map_err(serde::de::Error::custom)?;
        Ok(Some(style))
    }
}

fn serialize_optional_style<S>(style: &Option<Style>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match style {
        Some(style) => serialize_style(style, serializer),
        None => serializer.serialize_none(),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColumnSettings {
    pub name: String,
    pub width: usize,
    #[serde(
        default,
        deserialize_with = "parse_alignment",
        serialize_with = "serialize_alignment"
    )]
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

fn serialize_alignment<S>(align: &Alignment, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(match align {
        Alignment::Left => "left",
        Alignment::Right => "right",
        Alignment::Center => "center",
    })
}

impl Settings {
    pub fn new() -> Result<Settings, Box<dyn std::error::Error>> {
        let mut settings = Settings::default();

        settings.read_from_string(Self::default_settings_yaml_data())?;

        // Try to load from ~/.config/tailtales/settings.yaml. If does not exist, ignore.

        let filename = Self::local_settings_filename();

        if let Some(filename) = filename {
            if filename.exists() {
                settings
                    .read_from_yaml(filename.to_str().unwrap_or("unknown"))
                    .map_err(|e| {
                        format!("Error reading settings from {}: {}", filename.display(), e)
                    })?;
            }
        }

        Ok(settings)
    }
    pub fn default_settings_yaml_data() -> &'static str {
        include_str!("../settings.yaml")
    }

    pub fn local_settings_filename() -> Option<PathBuf> {
        let xdg = xdg::BaseDirectories::with_prefix("tailtales");

        if xdg.is_err() {
            return None;
        }

        xdg.unwrap().find_config_file("settings.yaml")
    }

    pub fn save_default_settings(&self) -> Result<(), Box<dyn std::error::Error>> {
        let xdg = xdg::BaseDirectories::with_prefix("tailtales")?;
        let path = xdg.place_config_file("settings.yaml")?;
        let filecontents = Self::default_settings_yaml_data();
        std::fs::write(path, filecontents)?;

        Ok(())
    }

    pub fn read_from_yaml(&mut self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = std::fs::File::open(filename)?;
        let reader = std::io::BufReader::new(file);
        let settings: SettingsFromYaml = serde_yaml::from_reader(reader)?;

        self.merge_with(settings);

        Ok(())
    }

    pub fn read_from_string(&mut self, s: &str) -> Result<(), Box<dyn std::error::Error>> {
        let settings: SettingsFromYaml = serde_yaml::from_str(s)?;
        self.merge_with(settings);

        Ok(())
    }

    pub fn merge_with(&mut self, other: SettingsFromYaml) {
        if other.global.is_some() {
            self.global = other.global.unwrap();
        }

        if other.default_arguments.len() > 0 {
            self.default_arguments = other.default_arguments.clone();
        }

        let mut other_rules = other.rules.clone();
        other_rules.extend(self.rules.clone());

        if other.keybindings.is_some() {
            self.keybindings.extend(other.keybindings.unwrap());
        }

        if other.colors.is_some() {
            let other_colors = other.colors.unwrap();
            self.colors.normal = other_colors.normal;
            self.colors.highlight = other_colors.highlight;
            self.colors.mark = other_colors.mark;
            self.colors.mark_highlight = other_colors.mark_highlight;
            self.colors.details = other_colors.details;
            self.colors.table = other_colors.table;
            self.colors.footer = other_colors.footer;
        }

        self.rules = other_rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_parse_style() {
    //     let input = "white black";
    //     let result = parse_style(input.into()).unwrap();
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
        let mut settings = Settings::new().unwrap();
        settings.read_from_yaml("settings.yaml").unwrap();
        println!("{:#?}", settings);
    }
}
