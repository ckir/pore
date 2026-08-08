//! Language support for Tantivy stemmers.
//!
//! This module defines [`LanguageRef`], a serializable wrapper around Tantivy's
//! [`tantivy::tokenizer::Language`] enum. It adds serde (de)serialization and
//! mlua conversion so that language settings can be configured from config files
//! and Lua scripts.
//!
//! Not all Tantivy languages are exposed — only those that make sense for the
//! pore use case.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tantivy::tokenizer::Language;

/// A supported stemming language.
///
/// Each variant maps 1:1 to a Tantivy [`Language`]. Serde serializes these to
/// snake_case (e.g., `LanguageRef::English` → `"english"`), and [`FromStr`]
/// parsing is case-insensitive.
#[derive(Debug, Deserialize, Serialize, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LanguageRef {
    /// Arabic
    Arabic,
    /// Danish
    Danish,
    /// Dutch
    Dutch,
    /// English
    English,
    /// Finnish
    Finnish,
    /// French
    French,
    /// German
    German,
    /// Greek
    Greek,
    /// Hungarian
    Hungarian,
    /// Italian
    Italian,
    /// Norwegian
    Norwegian,
    /// Portuguese
    Portuguese,
    /// Romanian
    Romanian,
    /// Russian
    Russian,
    /// Spanish
    Spanish,
    /// Swedish
    Swedish,
    /// Tamil
    Tamil,
    /// Turkish
    Turkish,
}

/// Converts this [`LanguageRef`] into the corresponding Tantivy [`Language`].
impl From<LanguageRef> for Language {
    fn from(val: LanguageRef) -> Self {
        match val {
            LanguageRef::Arabic => Language::Arabic,
            LanguageRef::Danish => Language::Danish,
            LanguageRef::Dutch => Language::Dutch,
            LanguageRef::English => Language::English,
            LanguageRef::Finnish => Language::Finnish,
            LanguageRef::French => Language::French,
            LanguageRef::German => Language::German,
            LanguageRef::Greek => Language::Greek,
            LanguageRef::Hungarian => Language::Hungarian,
            LanguageRef::Italian => Language::Italian,
            LanguageRef::Norwegian => Language::Norwegian,
            LanguageRef::Portuguese => Language::Portuguese,
            LanguageRef::Romanian => Language::Romanian,
            LanguageRef::Russian => Language::Russian,
            LanguageRef::Spanish => Language::Spanish,
            LanguageRef::Swedish => Language::Swedish,
            LanguageRef::Tamil => Language::Tamil,
            LanguageRef::Turkish => Language::Turkish,
        }
    }
}

/// Parses a language name from a string (case-insensitive).
///
/// # Examples
/// ```
/// use std::str::FromStr;
/// use pore_core::language::LanguageRef;
///
/// assert_eq!(LanguageRef::from_str("English").unwrap(), LanguageRef::English);
/// assert_eq!(LanguageRef::from_str("german").unwrap(), LanguageRef::German);
/// assert!(LanguageRef::from_str("klingon").is_err());
/// ```
impl FromStr for LanguageRef {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "arabic" => Ok(LanguageRef::Arabic),
            "danish" => Ok(LanguageRef::Danish),
            "dutch" => Ok(LanguageRef::Dutch),
            "english" => Ok(LanguageRef::English),
            "finnish" => Ok(LanguageRef::Finnish),
            "french" => Ok(LanguageRef::French),
            "german" => Ok(LanguageRef::German),
            "greek" => Ok(LanguageRef::Greek),
            "hungarian" => Ok(LanguageRef::Hungarian),
            "italian" => Ok(LanguageRef::Italian),
            "norwegian" => Ok(LanguageRef::Norwegian),
            "portuguese" => Ok(LanguageRef::Portuguese),
            "romanian" => Ok(LanguageRef::Romanian),
            "russian" => Ok(LanguageRef::Russian),
            "spanish" => Ok(LanguageRef::Spanish),
            "swedish" => Ok(LanguageRef::Swedish),
            "tamil" => Ok(LanguageRef::Tamil),
            "turkish" => Ok(LanguageRef::Turkish),
            _ => Err(anyhow!("Invalid language value '{}'", s)),
        }
    }
}

/// Converts a Lua value to a [`LanguageRef`].
///
/// Accepts a Lua string and delegates to [`FromStr`]. Non-string values
/// produce an error.
impl mlua::FromLua for LanguageRef {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match &value {
            mlua::Value::String(str) => LanguageRef::from_str(&str.to_str()?).map_err(|e| {
                mlua::Error::FromLuaConversionError {
                    from: "string",
                    to: "Language".to_string(),
                    message: Some(e.to_string()),
                }
            }),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: "value",
                to: "Language".to_string(),
                message: Some("Value is not a string".to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn all_variants_serialize_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&LanguageRef::Arabic).unwrap(),
            "\"arabic\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Danish).unwrap(),
            "\"danish\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Dutch).unwrap(),
            "\"dutch\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::English).unwrap(),
            "\"english\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Finnish).unwrap(),
            "\"finnish\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::French).unwrap(),
            "\"french\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::German).unwrap(),
            "\"german\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Greek).unwrap(),
            "\"greek\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Hungarian).unwrap(),
            "\"hungarian\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Italian).unwrap(),
            "\"italian\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Norwegian).unwrap(),
            "\"norwegian\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Portuguese).unwrap(),
            "\"portuguese\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Romanian).unwrap(),
            "\"romanian\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Russian).unwrap(),
            "\"russian\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Spanish).unwrap(),
            "\"spanish\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Swedish).unwrap(),
            "\"swedish\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Tamil).unwrap(),
            "\"tamil\""
        );
        assert_eq!(
            serde_json::to_string(&LanguageRef::Turkish).unwrap(),
            "\"turkish\""
        );
    }

    #[test]
    fn all_variants_deserialize_from_snake_case() {
        assert_eq!(
            serde_json::from_str::<LanguageRef>("\"arabic\"").unwrap(),
            LanguageRef::Arabic
        );
        assert_eq!(
            serde_json::from_str::<LanguageRef>("\"english\"").unwrap(),
            LanguageRef::English
        );
        assert_eq!(
            serde_json::from_str::<LanguageRef>("\"turkish\"").unwrap(),
            LanguageRef::Turkish
        );
    }

    #[test]
    fn from_str_accepts_lowercase() {
        assert_eq!(
            LanguageRef::from_str("english").unwrap(),
            LanguageRef::English
        );
        assert_eq!(
            LanguageRef::from_str("arabic").unwrap(),
            LanguageRef::Arabic
        );
    }

    #[test]
    fn from_str_accepts_mixed_case() {
        assert_eq!(
            LanguageRef::from_str("English").unwrap(),
            LanguageRef::English
        );
        assert_eq!(
            LanguageRef::from_str("ENGLISH").unwrap(),
            LanguageRef::English
        );
        assert_eq!(
            LanguageRef::from_str("German").unwrap(),
            LanguageRef::German
        );
    }

    #[test]
    fn from_str_rejects_invalid() {
        assert!(LanguageRef::from_str("invalid").is_err());
        assert!(LanguageRef::from_str("").is_err());
    }

    #[test]
    fn lua_from_string_converts() {
        let lua = mlua::Lua::new();
        let val: mlua::Result<LanguageRef> = lua.load("'english'").eval();
        assert_eq!(val.unwrap(), LanguageRef::English);
    }

    #[test]
    fn lua_from_string_rejects_non_string() {
        let lua = mlua::Lua::new();
        let val: mlua::Result<LanguageRef> = lua.load("42").eval();
        assert!(val.is_err());
    }

    #[test]
    fn lua_from_string_rejects_invalid_language() {
        let lua = mlua::Lua::new();
        let val: mlua::Result<LanguageRef> = lua.load("'bogus'").eval();
        assert!(val.is_err());
    }
}
