use ratatui::style::{Color, Style};
use serde::{de::Deserializer, Deserialize};
use std::str::FromStr;

// singleton load settings

#[derive(Debug, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub global: GlobalSettings,
    #[serde(default)]
    pub rules: Vec<RulesSettings>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SettingsFromYaml {
    #[serde(default)]
    pub global: Option<GlobalSettings>,
    #[serde(default)]
    pub rules: Vec<RulesSettings>,
}

#[derive(Debug, Deserialize, Default)]
pub struct GlobalSettings {
    // pub reload_on_truncate: bool,
    pub colors: GlobalColorSettings,
}

#[derive(Debug, Deserialize, Default)]
pub struct GlobalColorSettings {
    #[serde(deserialize_with = "parse_style")]
    pub normal: Style,
    #[serde(deserialize_with = "parse_style")]
    pub highlight: Style,
    pub details: DetailsColorSettings,
    pub table: TableColorSettings,
}

#[derive(Debug, Deserialize, Default)]

pub struct TableColorSettings {
    #[serde(deserialize_with = "parse_style")]
    pub header: Style,
}

#[derive(Debug, Deserialize, Default)]
pub struct DetailsColorSettings {
    #[serde(deserialize_with = "parse_style")]
    pub title: Style,
    #[serde(deserialize_with = "parse_style")]
    pub key: Style,
    #[serde(deserialize_with = "parse_style")]
    pub value: Style,
    #[serde(deserialize_with = "parse_style")]
    pub border: Style,
}

#[derive(Debug, Deserialize, Default, PartialEq, Clone)]
pub enum Alignment {
    #[default]
    Left,
    Right,
    Center,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RulesSettings {
    pub name: String,
    #[serde(default)]
    pub file_patterns: Vec<String>,
    #[serde(default)]
    pub extractors: Vec<String>,
    // #[serde(default)]
    // pub filters: Vec<FilterSettings>,
    #[serde(default)]
    pub columns: Vec<ColumnSettings>,
}

// #[derive(Debug, Deserialize, Clone)]
// pub struct FilterSettings {
//     #[serde(default)]
//     pub name: String,
//     pub expression: String,
//     #[serde(default, deserialize_with = "parse_style")]
//     pub highlight: Style,
//     #[serde(default, deserialize_with = "optional_parse_style")]
//     pub gutter: Option<Style>,
// }

fn parse_style<'de, D>(deserializer: D) -> Result<Style, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    string_to_style(&s).map_err(serde::de::Error::custom)
}

fn string_to_style(s: &str) -> Result<Style, String> {
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

// fn optional_parse_style<'de, D>(deserializer: D) -> Result<Option<Style>, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     let s: String = Deserialize::deserialize(deserializer)?;
//     if s.is_empty() {
//         Ok(None)
//     } else {
//         let style = string_to_style(&s).map_err(serde::de::Error::custom)?;
//         Ok(Some(style))
//     }
// }

#[derive(Debug, Deserialize, Clone)]
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
    pub fn new() -> Settings {
        let mut settings = Settings::default();

        settings
            .read_from_string(include_str!("../settings.yaml"))
            .expect("Failed to read default settings from internal YAML");

        // Try to load from ~/.config/tailtales/settings.yaml. If does not exist, ignore.

        xdg::BaseDirectories::with_prefix("tailtales")
            .map(|xdg| xdg.find_config_file("settings.yaml"))
            .map(|path| {
                path.map(|path| {
                    if path.exists() {
                        settings.read_from_yaml(path.to_str().unwrap()).unwrap();
                    }
                })
            })
            .unwrap_or(None);

        settings
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

        // copy rules at the beginning, so have priority over default rules
        let mut other_rules = other.rules.clone();
        other_rules.extend(self.rules.clone());

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
        let mut settings = Settings::new();
        settings.read_from_yaml("settings.yaml").unwrap();
        println!("{:#?}", settings);
    }
}
