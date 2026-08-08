//! Color output mode handling.
//!
//! This module defines the [`ColorMode`] enum, which controls when the binary uses colored
//! output. It implements conversions from string parsing (for CLI args), TOML deserialization
//! (for config files), and `mlua::FromLua` (so the Lua module can accept the same values).

use std::str::FromStr;

use serde::Deserialize;
use termcolor::ColorChoice;

/// Controls when colored output is used.
///
/// The four variants map directly to `termcolor::ColorChoice`. `Auto` checks whether
/// stdout is a TTY and falls back to no colors otherwise.
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    /// Auto-detect terminal support (falls back to `Never` if stdout is not a TTY).
    Auto,
    /// Always emit color codes regardless of destination.
    Always,
    /// Always emit color codes using ANSI escape sequences.
    Ansi,
    /// Never emit color codes.
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
                // Auto-detect: when stdout is not a TTY (e.g. piped to a file),
                // disable colors to avoid polluting output with escape codes.
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

/// Allows Lua scripts to pass a color mode string (e.g., `"auto"`, `"always"`)
/// which is parsed into a [`ColorMode`].
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn color_mode_from_str_valid() {
        assert_eq!(ColorMode::from_str("auto").unwrap(), ColorMode::Auto);
        assert_eq!(ColorMode::from_str("always").unwrap(), ColorMode::Always);
        assert_eq!(ColorMode::from_str("ansi").unwrap(), ColorMode::Ansi);
        assert_eq!(ColorMode::from_str("never").unwrap(), ColorMode::Never);
    }

    #[test]
    fn color_mode_from_str_case_insensitive() {
        assert_eq!(ColorMode::from_str("AUTO").unwrap(), ColorMode::Auto);
        assert_eq!(ColorMode::from_str("Always").unwrap(), ColorMode::Always);
    }

    #[test]
    fn color_mode_from_str_invalid() {
        assert!(ColorMode::from_str("invalid").is_err());
    }

    #[test]
    fn color_mode_into_color_choice() {
        let choice: ColorChoice = ColorMode::Always.into();
        assert!(matches!(choice, ColorChoice::Always));

        let choice: ColorChoice = ColorMode::Ansi.into();
        assert!(matches!(choice, ColorChoice::AlwaysAnsi));

        let choice: ColorChoice = ColorMode::Never.into();
        assert!(matches!(choice, ColorChoice::Never));
    }
}
