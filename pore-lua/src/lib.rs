//! Lua bindings for `pore` — exposes Tantivy-backed indexing to Lua scripts.
//!
//! This crate builds a `pore_lua` Lua module that can be loaded from Lua via
//! `require("pore_lua")`. It exports two constructors — `get_file_index` and `get_index` —
//! which return userdata objects wrapping [`FileIndex`](pore_core::FileIndex) and
//! [`GenericIndex`](pore_core::GenericIndex) respectively.
//!
//! # Module API
//!
//! ```lua
//! -- Create a file index for a directory
//! local idx = pore.get_file_index("/path/to/dir", "/path/to/cache", {threads = 4})
//! idx:update()
//! local results = idx:search("some query", {limit = 10})
//! idx:delete()
//!
//! -- Create a generic index for custom documents
//! local gidx = pore.get_index("id", {"title", "body"}, {}, "/path/to/cache")
//! gidx:add_documents({{id = "1", title = "Hello", body = "world"}})
//! gidx:search("hello", {})
//! ```
//!
//! # Version info
//!
//! The module also exports a `version` table with `full`, `major`, `minor`, `patch`,
//! and `pre` fields, populated from Cargo package metadata at build time.

use std::path::PathBuf;
use std::str::FromStr;

use mlua::prelude::*;
use mlua::{UserData, UserDataMethods};
use pore_core::{
    FileIndex, FileIndexOptionsShape, FileSearchOptionsShape, GenericIndex, IndexOptionsShape,
    SearchOptionsShape,
};
use tantivy::query::QueryParser;

/// Entry point for the Lua module. Called when the shared library is loaded via
/// `require("pore_lua")`.
///
/// Returns a Lua table with the following keys:
/// - `version` — a table with `full`, `major`, `minor`, `patch`, `pre` fields.
/// - `get_file_index(for_dir, cache_dir, config)` — creates or opens a [`FileIndex`].
/// - `get_index(id_field, text_fields, config, cache_dir)` — creates or opens a [`GenericIndex`].
#[mlua::lua_module]
fn pore_lua(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;

    exports.set("version", make_version_tbl(lua)?)?;
    let get_file_index = lua.create_function(
        |_, (for_dir, cache_dir, config): (String, Option<String>, FileIndexOptionsShape)| {
            let index = FileIndex::get_or_create(
                PathBuf::from_str(&for_dir)
                    .map_err(|_| LuaError::RuntimeError(format!("Invalid path {}", for_dir)))?,
                cache_dir
                    .as_ref()
                    .map(|s| {
                        PathBuf::from_str(&s).map_err(|_| {
                            LuaError::RuntimeError(format!("Invalid path {:?}", cache_dir))
                        })
                    })
                    .transpose()?,
                &config.into(),
            )
            .map_err(|e| LuaError::RuntimeError(format!("Error creating index {:?}", e)))?;
            Ok(FileIndexLua { index })
        },
    )?;
    exports.set("get_file_index", get_file_index)?;

    let get_index = lua.create_function(
        |_,
         (id_field, text_fields, config, cache_dir): (
            String,
            Vec<String>,
            IndexOptionsShape,
            Option<String>,
        )| {
            let index = GenericIndex::get_or_create(
                &id_field,
                text_fields,
                &config.into(),
                cache_dir
                    .as_ref()
                    .map(|s| {
                        PathBuf::from_str(&s).map_err(|_| {
                            LuaError::RuntimeError(format!("Invalid path {:?}", cache_dir))
                        })
                    })
                    .transpose()?
                    .as_deref(),
            )
            .map_err(|e| LuaError::RuntimeError(format!("Error creating index {:?}", e)))?;
            Ok(GenericIndexLua { index })
        },
    )?;
    exports.set("get_index", get_index)?;

    Ok(exports)
}

/// Sets a key in a Lua table if the corresponding environment variable is non-empty.
macro_rules! set_nonempty_env {
    ($tbl:ident, $key:literal, $env_key:literal) => {{
        let value = env!($env_key);
        if !value.is_empty() {
            $tbl.set($key, value)?;
        }
    }};
}

/// Lua userdata wrapping a [`FileIndex`](pore_core::FileIndex).
///
/// Provides methods: `update(rebuild?)`, `delete()`, `search(query, opts)`, and `__tostring`.
#[derive(Debug, Clone)]
struct FileIndexLua {
    index: FileIndex,
}

impl UserData for FileIndexLua {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("update", |_, this, (rebuild,): (Option<bool>,)| {
            this.index
                .update(rebuild.unwrap_or(false))
                .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
            Ok(())
        });
        methods.add_method_mut("delete", |_, this, _: ()| {
            this.index
                .delete()
                .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
            Ok(())
        });
        methods.add_method(
            "search",
            |_, this, (query_str, opts): (String, FileSearchOptionsShape)| {
                let query_parser =
                    QueryParser::for_index(this.index.index(), vec![*this.index.contents()]);
                let query = query_parser
                    .parse_query(&query_str)
                    .map_err(|_| LuaError::RuntimeError("Error parsing query".to_string()))?;
                let results = this
                    .index
                    .search(&query, &opts.into())
                    .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                Ok(results)
            },
        );
        methods.add_method("__tostring", |_, this: &FileIndexLua, _: ()| {
            Ok(format!("{}", this.index))
        });
    }
}

/// Lua userdata wrapping a [`GenericIndex`](pore_core::GenericIndex).
///
/// Provides methods: `delete()`, `delete_documents(doc_ids)`, `update_documents(docs)`,
/// `add_documents(docs)`, `search(query, opts)`, and `__tostring`.
#[derive(Debug, Clone)]
struct GenericIndexLua {
    index: GenericIndex,
}

impl UserData for GenericIndexLua {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("delete", |_, this, _: ()| {
            this.index
                .delete()
                .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
            Ok(())
        });
        methods.add_method_mut("delete_documents", |_, this, (doc_ids,): (Vec<String>,)| {
            this.index
                .delete_documents(doc_ids)
                .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
            Ok(())
        });
        methods.add_method_mut(
            "update_documents",
            |_, this, (documents,): (Vec<mlua::Table>,)| {
                this.index
                    .update_documents(documents)
                    .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                Ok(())
            },
        );
        methods.add_method_mut(
            "add_documents",
            |_, this, (documents,): (Vec<mlua::Table>,)| {
                this.index
                    .add_documents(documents)
                    .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                Ok(())
            },
        );
        methods.add_method(
            "search",
            |_, this, (query_str, opts): (String, SearchOptionsShape)| {
                let query_parser =
                    QueryParser::for_index(this.index.index(), this.index.get_text_fields());
                let query = query_parser
                    .parse_query(&query_str)
                    .map_err(|_| LuaError::RuntimeError("Error parsing query".to_string()))?;
                let results = this
                    .index
                    .search(&query, &opts.into())
                    .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                Ok(results)
            },
        );
        methods.add_method("__tostring", |_, this: &GenericIndexLua, _: ()| {
            Ok(format!("{:?}", this.index))
        });
    }
}

/// Creates a Lua table with version metadata from Cargo environment variables.
fn make_version_tbl(lua: &Lua) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;

    set_nonempty_env!(tbl, "full", "CARGO_PKG_VERSION");
    set_nonempty_env!(tbl, "major", "CARGO_PKG_VERSION_MAJOR");
    set_nonempty_env!(tbl, "minor", "CARGO_PKG_VERSION_MINOR");
    set_nonempty_env!(tbl, "patch", "CARGO_PKG_VERSION_PATCH");
    set_nonempty_env!(tbl, "pre", "CARGO_PKG_VERSION_PRE");

    Ok(tbl)
}
