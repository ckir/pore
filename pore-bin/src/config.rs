//! Configuration file loading and option merging.
//!
//! This module handles loading the TOML config file from `$XDG_CONFIG_HOME/pore.toml`
//! (falling back to `$HOME/.config/pore.toml`) and merging options from three sources:
//!
//! 1. **Global defaults** — hardcoded `Default` implementations.
//! 2. **Config file globals** — top-level TOML keys apply to all directories.
//! 3. **Local config sections** — TOML tables keyed by an arbitrary name that contain a
//!    `path` field matching the query directory. Local sections may optionally nest an
//!    index-specific subsection (looked up by `index_name`).
//!
//! # Option types
//!
//! `SearchConfigOpt` and `FileIndexOptionsShape` are "optional" shapes where every field
//! is `Option<T>`. They are produced by the `#[create_option_copy]` macro and allow
//! incremental merging: only explicitly-set CLI flags or config-file keys populate a value,
//! so defaults from the config file do not accidentally override more specific settings.

use macros::create_option_copy;
use pore_core::FileIndexOptionsShape;
use pore_core::FileSearchOptions;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use toml::Value;

use crate::color_mode::ColorMode;
const CONFIG_FILE: &str = "pore.toml";

/// Resolved search configuration with concrete (non-optional) defaults.
///
/// All fields have sensible defaults and are populated either from the config file or
/// from `SearchConfig::default()`. The `#[create_option_copy]` macro generates the
/// companion `SearchConfigOpt` struct (with `Option<T>` fields) and a `merge_from` method.
#[create_option_copy(SearchConfigOpt)]
#[derive(Debug, Deserialize, Clone)]
pub struct SearchConfig {
    /// Output results as JSON instead of human-readable text.
    pub json: bool,
    /// Maximum number of files to return.
    pub limit: usize,
    /// Minimum relevance score threshold for including a result.
    pub threshold: f32,
    /// When true, only print matching filenames, not the matching lines.
    pub filename_only: bool,
    /// Color output mode (auto, always, ansi, never).
    pub color: ColorMode,
    /// Force a full index rebuild before searching.
    pub rebuild_index: bool,
    /// Whether to update the index before searching (default: true).
    pub update: bool,
    /// When true, keep the index in memory only (no disk cache).
    pub in_memory: bool,
}

impl Default for SearchConfig {
    fn default() -> SearchConfig {
        SearchConfig {
            json: false,
            limit: 1000,
            threshold: 0.0,
            filename_only: false,
            color: ColorMode::Auto,
            rebuild_index: false,
            update: true,
            in_memory: false,
        }
    }
}

impl SearchConfig {
    /// Convert search settings into a `FileSearchOptions` for executing a query.
    ///
    /// `search_dir` is the subdirectory within the index to restrict results to.
    pub fn to_opts(&self, search_dir: &str) -> FileSearchOptions {
        FileSearchOptions {
            limit: self.limit,
            threshold: self.threshold,
            filename_only: self.filename_only,
            root_dir: Some(search_dir.to_string()),
        }
    }
}

/// Load configuration from the TOML config file and merge with defaults.
///
/// The config file is looked up at `$XDG_CONFIG_HOME/pore.toml` or `$HOME/.config/pore.toml`.
/// If the file does not exist, default options are returned.
///
/// # Merging order
///
/// 1. Start from global top-level TOML keys.
/// 2. Find a local config section whose `path` matches `path`. Merge it.
/// 3. If `index_name` is provided, look for a matching subsection within the local config
///    (e.g. `[local-1.my_index]`). Merge it.
/// 4. If `index_name` is provided but no local section matched, look for a global named
///    index (`[index-<name>]`). Merge it.
///
/// # Errors
///
/// Returns an error if the config file exists but cannot be parsed, or if `index_name`
/// is specified but no matching index configuration is found.
pub fn load_config(
    path: &Path,
    index_name: Option<&str>,
) -> Result<(FileIndexOptionsShape, SearchConfigOpt), anyhow::Error> {
    let path_str = path.to_string_lossy();
    let mut config_home = env::var("XDG_CONFIG_HOME").unwrap_or("".to_string());
    if config_home.is_empty() {
        config_home = env::var("HOME")? + "/.config";
    }
    let config_file = PathBuf::from(config_home).join(CONFIG_FILE);
    if config_file.exists() {
        let contents = &fs::read_to_string(&config_file)?;
        let value: Value = toml::from_str(contents)
            .map_err(|e| anyhow!("Error parsing config file {:?}: {}", config_file, e))?;
        let mut index: FileIndexOptionsShape = value.clone().try_into()?;
        let mut search: SearchConfigOpt = value.clone().try_into()?;

        let mut found_index = false;
        if let Value::Table(table) = &value {
            // Look for a local configuration with a matching path
            for (_, val) in table.iter() {
                if let Value::Table(local_config) = val {
                    if local_config.get("path") == Some(&Value::String(path_str.to_string())) {
                        index.merge_from(&val.clone().try_into()?);
                        search.merge_from(&val.clone().try_into()?);
                        // Look for an index the local config
                        if let Some(idx_name) = index_name {
                            if let Some(local_index_config) = local_config.get(idx_name) {
                                index.merge_from(&local_index_config.clone().try_into()?);
                                search.merge_from(&local_index_config.clone().try_into()?);
                                found_index = true;
                            }
                        }
                        break;
                    }
                }
            }
            // if index exists, find global index and load it
            if let Some(name) = index_name {
                if let Some(global_index) = table.get(&format!("index-{}", name)) {
                    index.merge_from(&global_index.clone().try_into()?);
                    search.merge_from(&global_index.clone().try_into()?);
                    found_index = true;
                }
            }
        }
        if let Some(name) = index_name {
            if !found_index {
                bail!("Could not find index '{}'", name);
            }
        }

        return Ok((index, search));
    }
    Ok((FileIndexOptionsShape::default(), SearchConfigOpt::default()))
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf, str::FromStr};

    use pore_core::FileIndexOptions;

    use crate::config::{FileIndexOptionsShape, SearchConfigOpt};

    use super::{load_config, CONFIG_FILE};

    #[test]
    fn parsing_opt_configs_works() {
        let contents = "follow = false
threads = 100
limit = 4
";
        let index: FileIndexOptionsShape = toml::from_str(contents).unwrap();
        assert_eq!(index.follow, Some(false));
        assert_eq!(index.threads, Some(100));
        assert_eq!(index.ignore_files, None);
        let search: SearchConfigOpt = toml::from_str(contents).unwrap();
        assert_eq!(search.limit, Some(4));
        assert_eq!(search.json, None);
    }

    #[test]
    fn merging_opt_configs_works() {
        let mut i1 = FileIndexOptionsShape {
            follow: Some(true),
            ..Default::default()
        };
        let i2 = FileIndexOptionsShape {
            threads: Some(20),
            ..Default::default()
        };
        i1.merge(&i2);
        assert_eq!(i1.follow, Some(true));
        assert_eq!(i1.threads, Some(20));
        assert_eq!(i1.language, None);
        let conf: FileIndexOptions = i1.into();
        assert!(conf.follow);
        assert_eq!(conf.threads, 20);
        assert!(!conf.hidden);
    }

    #[test]
    fn can_load_and_merge_defaults() {
        let tmpdir = tempfile::tempdir().unwrap();
        env::set_var("XDG_CONFIG_HOME", tmpdir.path().as_os_str());
        let conf_file = PathBuf::from(tmpdir.path()).join(CONFIG_FILE);
        fs::write(
            conf_file,
            r#"threads = 10

[index-global_index]
threads = 20

[local-1]
path = '/foo'
threads = 30

[local-1.local_index]
threads = 40
"#,
        )
        .unwrap();

        let (index, _) = load_config(&PathBuf::from_str("/").unwrap(), None).unwrap();
        assert_eq!(index.threads, Some(10));
        let (index, _) =
            load_config(&PathBuf::from_str("/").unwrap(), Some("global_index")).unwrap();
        assert_eq!(index.threads, Some(20));
        let (index, _) = load_config(&PathBuf::from_str("/foo").unwrap(), None).unwrap();
        assert_eq!(index.threads, Some(30));
        let (index, _) =
            load_config(&PathBuf::from_str("/foo").unwrap(), Some("local_index")).unwrap();
        assert_eq!(index.threads, Some(40));
    }

    #[test]
    fn example_file_is_complete() {
        let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pore.example.toml");
        let contents = &fs::read_to_string(&example).unwrap();
        let value: toml::Value =
            toml::from_str(contents).unwrap_or_else(|_| panic!("Error parsing config file {:?}", example));
        let index: FileIndexOptionsShape = value.clone().try_into().unwrap();
        let search: SearchConfigOpt = value.clone().try_into().unwrap();
        if let Err(missing_fields) = index.all() {
            panic!("pore.example.toml is missing fields: {:?}", missing_fields);
        }
        if let Err(missing_fields) = search.all() {
            panic!("pore.example.toml is missing fields: {:?}", missing_fields);
        }
    }
}
