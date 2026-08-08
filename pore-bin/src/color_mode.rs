use std::str::FromStr;

use serde::Deserialize;
use termcolor::ColorChoice;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    Auto,
    Always,
    Ansi,
    Never,
}

impl Into<ColorChoice> for ColorMode {
    fn into(self) -> ColorChoice {
        match self {
            ColorMode::Auto => ColorChoice::Auto,
            ColorMode::Always => ColorChoice::Always,
            ColorMode::Ansi => ColorChoice::AlwaysAnsi,
            ColorMode::Never => ColorChoice::Never,
        }
    }
}

impl FromStr for ColorMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "always" => Ok(ColorMode::Always),
            "ansi" => Ok(ColorMode::Ansi),
            "auto" => {
                if atty::is(atty::Stream::Stdout) {
                    Ok(ColorMode::Auto)
                } else {
                    Ok(ColorMode::Never)
                }
            }
            "never" => Ok(ColorMode::Never),
            _ => Err(anyhow!("Invalid color value '{}'", s)),
        }
    }
}

impl mlua::FromLua for ColorMode {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        return match &value {
            mlua::Value::String(str) => ColorMode::from_str(&str.to_str()?).map_err(|e| {
                mlua::Error::FromLuaConversionError {
                    from: "string",
                    to: "ColorMode".to_string(),
                    message: Some(e.to_string()),
                }
            }),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: "value",
                to: "ColorMode".to_string(),
                message: Some("Value is not a string".to_string()),
            }),
        };
    }
}
