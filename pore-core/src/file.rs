//! File-based full-text indexing.
//!
//! This module provides [`FileIndex`], which indexes the text contents of files
//! in a directory tree. It uses the `ignore` crate for efficient parallel
//! directory walking that respects `.gitignore`, `.gitexclude`, and other
//! ignore files.
//!
//! # Key types
//! - [`FileIndex`] — the index handle, created via [`FileIndex::get_or_create`].
//! - [`FileIndexOptions`] — configuration for file walking and tokenization.
//! - [`FileSearchOptions`] — parameters controlling search result limits and format.
//! - [`FileMetadata`] — persisted metadata about when and how the index was built.
//! - [`FileSearchResult`] and [`Line`] — search result structures.

use crate::common::create_index;
use crate::common::delete_index;
use crate::common::IndexMetadata;
use crate::common::MetadataConfig;
use crate::common::METADATA_FILE;
use crate::language::LanguageRef;
use crate::location;
use crate::location::DocResult;
use chrono::DateTime;
use chrono::Local;
use chrono::NaiveDateTime;
use chrono::Utc;
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use ignore::WalkState;
use macros::create_option_copy;
use mlua::IntoLua;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt::Display;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::Query;
use tantivy::ReloadPolicy;

use tantivy::schema::*;
use tantivy::Index;

/// A file-based full-text index.
///
/// Holds a Tantivy [`Index`] along with the schema fields (`filepath`, `contents`)
/// and metadata about the indexed directory. Created via [`FileIndex::get_or_create`].
#[derive(Debug, Clone)]
pub struct FileIndex {
    meta: FileMetadata,
    cache_dir: Option<PathBuf>,
    index: Index,
    filepath: Field,
    contents: Field,
}

/// Configuration options for building a [`FileIndex`].
///
/// Controls which files are included, whether to follow symlinks, the stemming
/// language, and how many threads to use for the directory walker.
///
/// The `#[create_option_copy]` macro generates a companion `*Shape` struct and
/// copy-conversion functions for Lua interop.
#[create_option_copy(FileIndexOptionsShape)]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct FileIndexOptions {
    /// Whether to follow symbolic links during file walking.
    pub follow: bool,
    /// Glob patterns to include files.
    pub glob: Vec<String>,
    /// Whether glob matching is case-insensitive.
    pub glob_case_insensitive: bool,
    /// Whether to include hidden files and directories.
    pub hidden: bool,
    /// Whether to respect .gitignore and similar ignore files.
    pub ignore_files: bool,
    /// Language used for stemming.
    pub language: LanguageRef,
    /// Glob patterns to exclude files (takes precedence over `glob`).
    pub oglob: Vec<String>,
    // TODO: move this field to a more appropriate location.
    /// Number of threads for parallel directory walking (0 = auto).
    pub threads: usize,
}

impl Default for FileIndexOptions {
    fn default() -> FileIndexOptions {
        FileIndexOptions {
            follow: false,
            hidden: false,
            language: LanguageRef::English,
            ignore_files: true,
            glob_case_insensitive: false,
            glob: vec![],
            oglob: vec![],
            threads: 0,
        }
    }
}

/// Options that control search result formatting and limits.
#[create_option_copy(FileSearchOptionsShape)]
#[derive(Debug)]
pub struct FileSearchOptions {
    /// Maximum number of results to return.
    pub limit: usize,
    /// Minimum score threshold (results below this are excluded).
    pub threshold: f32,
    /// When true, only file paths are returned without matching lines.
    pub filename_only: bool,
    /// Overrides the base directory for resolving file paths.
    pub root_dir: Option<String>,
}

impl Default for FileSearchOptions {
    fn default() -> Self {
        FileSearchOptions {
            limit: 1000,
            threshold: 0.0,
            filename_only: false,
            root_dir: None,
        }
    }
}

/// Persisted metadata for a file index.
///
/// Tracks the index configuration, version, last update time, and the
/// directory that was indexed. Serialized to `pore_meta.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    version: String,
    last_update: DateTime<Utc>,
    config: FileIndexOptions,
    for_dir: PathBuf,
}

impl MetadataConfig for FileIndexOptions {
    fn language(&self) -> LanguageRef {
        self.language
    }
}

impl FileMetadata {
    /// Creates new metadata for the given directory.
    ///
    /// The path is canonicalized to an absolute path.
    pub fn new<P: AsRef<Path>>(
        config: FileIndexOptions,
        for_dir: P,
    ) -> Result<Self, anyhow::Error> {
        let path = for_dir.as_ref();
        Ok(FileMetadata {
            config,
            version: env!("CARGO_PKG_VERSION").to_string(),
            last_update: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            for_dir: fs::canonicalize(if path.is_absolute() {
                path.to_path_buf()
            } else {
                env::current_dir()?.join(path)
            })?,
        })
    }
}

/// A single search result for a file.
///
/// Contains the file path, relevance score, and optionally the matching
/// lines (omitted when [`FileSearchOptions::filename_only`] is true).
#[derive(Debug, Serialize)]
pub struct FileSearchResult {
    file: PathBuf,
    score: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lines: Vec<Line>,
}

impl FileSearchResult {
    /// Creates a new FileSearchResult.
    pub fn new(file: PathBuf, score: f32, lines: Vec<Line>) -> Self {
        Self { file, score, lines }
    }

    /// Returns the path to the matched file.
    pub fn file(&self) -> &Path {
        &self.file
    }
    /// Returns the relevance score.
    pub fn score(&self) -> f32 {
        self.score
    }
    /// Returns the matching lines in the file (may be empty).
    pub fn lines(&self) -> &Vec<Line> {
        &self.lines
    }
}

impl IntoLua for FileSearchResult {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let tbl = lua.create_table()?;
        tbl.set("file", self.file.to_string_lossy())?;
        tbl.set("score", self.score)?;
        if !self.lines.is_empty() {
            tbl.set("lines", self.lines)?;
        }
        Ok(mlua::Value::Table(tbl))
    }
}

/// A single matching line within a file.
#[derive(Debug, Serialize)]
pub struct Line {
    /// 1-based line number.
    pub number: u32,
    /// The line text (trailing newline stripped).
    pub text: String,
}

impl IntoLua for Line {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let tbl = lua.create_table()?;
        tbl.set("number", self.number)?;
        tbl.set("text", self.text)?;
        Ok(mlua::Value::Table(tbl))
    }
}

impl FileMetadata {
    /// Returns the directory that was indexed.
    pub fn for_dir(&self) -> &Path {
        &self.for_dir
    }
}

impl IndexMetadata<FileIndexOptions> for FileMetadata {
    fn config(&self) -> &FileIndexOptions {
        &self.config
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn last_update(&self) -> &DateTime<Utc> {
        &self.last_update
    }
    fn set_last_update(&mut self, time: DateTime<Utc>) {
        self.last_update = time;
    }
}

impl FileIndex {
    /// Returns a reference to the underlying Tantivy index.
    pub fn index(&self) -> &Index {
        &self.index
    }
    /// Returns the schema field for file paths.
    pub fn filepath(&self) -> &Field {
        &self.filepath
    }
    /// Returns the schema field for file contents.
    pub fn contents(&self) -> &Field {
        &self.contents
    }
    /// Deletes the index and its on-disk cache (if any).
    pub fn delete(&self) -> anyhow::Result<bool> {
        delete_index(&self.index, self.cache_dir.as_deref())
    }
    /// Opens an existing index or creates a new one.
    ///
    /// If a persisted index exists at `cache_dir` with a matching config, it is
    /// reused. Otherwise a fresh index is created.
    ///
    /// # Parameters
    /// * `for_dir` — the directory to index.
    /// * `cache_dir` — optional path for persisted index files.
    /// * `config` — indexing options.
    pub fn get_or_create<P: AsRef<Path>>(
        for_dir: P,
        cache_dir: Option<P>,
        config: &FileIndexOptions,
    ) -> Result<Self, anyhow::Error> {
        let (meta_opt, index): (Option<FileMetadata>, Index) =
            create_index(cache_dir.as_ref(), config, "filepath", vec!["contents"])?;
        let meta = meta_opt.unwrap_or_else(|| FileMetadata::new(config.clone(), for_dir).unwrap());
        let filepath = index
            .schema()
            .get_field("filepath")
            .expect("No field named 'filepath'");
        let contents = index
            .schema()
            .get_field("contents")
            .expect("No field named 'contents'");
        Ok(Self {
            index,
            cache_dir: cache_dir.map(|p| fs::canonicalize(p).unwrap()),
            meta,
            filepath,
            contents,
        })
    }

    /// Returns a configured directory walker for scanning the indexed directory.
    ///
    /// The walker respects the index's `.gitignore`, hidden file, glob, and
    /// symlink-following settings.
    pub fn get_file_walker(&self) -> Result<WalkBuilder, anyhow::Error> {
        let mut builder = WalkBuilder::new(&self.meta.for_dir);
        builder
            .hidden(!self.meta.config.hidden)
            .threads(self.meta.config.threads)
            .ignore(self.meta.config.ignore_files)
            .git_global(self.meta.config.ignore_files)
            .git_ignore(self.meta.config.ignore_files)
            .git_exclude(self.meta.config.ignore_files)
            .follow_links(self.meta.config.follow);
        if !self.meta.config.glob.is_empty() {
            let mut globs = OverrideBuilder::new(&self.meta.for_dir);
            globs.case_insensitive(self.meta.config.glob_case_insensitive)?;
            for glob in &self.meta.config.glob {
                globs.add(&glob)?;
            }
            builder.overrides(globs.build()?);
        }
        if !self.meta.config.oglob.is_empty() {
            let mut globs = OverrideBuilder::new(&self.meta.for_dir);
            globs.case_insensitive(self.meta.config.glob_case_insensitive)?;
            for glob in &self.meta.config.oglob {
                globs.add(&glob)?;
            }
            let matcher = globs.build()?;
            builder.filter_entry(move |e| {
                if e.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    return true;
                } else {
                    return matcher.matched(e.path(), false).is_whitelist();
                };
            });
        }
        Ok(builder)
    }

    /// Scans the indexed directory and adds/updates documents in the index.
    ///
    /// If `rebuild` is true, all readable files are re-indexed regardless of
    /// their modification time. Otherwise, only files modified since the last
    /// update are added.
    pub fn update(&mut self, rebuild: bool) -> Result<&mut Self, anyhow::Error> {
        let mut index_writer = self.index.writer::<tantivy::TantivyDocument>(50_000_000)?;
        let walker = self.get_file_walker()?;
        let now = Utc::now();
        walker.build_parallel().run(|| {
            Box::new(|result| {
                if let Ok(entry) = result {
                    if let Ok(contents) = fs::read_to_string(entry.path()) {
                        let modified: DateTime<Utc> =
                            entry.metadata().unwrap().modified().unwrap().into();
                        if rebuild || modified > self.meta.last_update {
                            let filepath = entry.path().strip_prefix(&self.meta.for_dir).unwrap();
                            let doc = doc!(
                                self.filepath => String::from(filepath.to_string_lossy()),
                                self.contents => contents,
                            );
                            index_writer.add_document(doc);
                        }
                    }
                }
                WalkState::Continue
            })
        });

        index_writer.commit()?;
        self.meta.last_update = now;
        if let Some(index_dir) = &self.cache_dir {
            fs::write(
                index_dir.join(METADATA_FILE),
                serde_json::to_string(&self.meta)?,
            )?;
        }

        return Ok(self);
    }

    /// Executes a search query against the file index.
    ///
    /// Returns results sorted by relevance, limited by [`FileSearchOptions::limit`],
    /// with matching line numbers and text when `filename_only` is false.
    pub fn search(
        &self,
        query: &Box<dyn Query>,
        opts: &FileSearchOptions,
    ) -> Result<Vec<FileSearchResult>, anyhow::Error> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let searcher = reader.searcher();
        let top_docs = searcher.search(query, &TopDocs::with_limit(opts.limit).order_by_score())?;
        let mut doc_results = Vec::new();
        for (score, doc_address) in top_docs {
            if score > opts.threshold {
                doc_results.push(DocResult {
                    score,
                    address: doc_address,
                });
            }
        }
        let mut position_map = location::get_search_results(self, query, &searcher, &doc_results)?;
        let mut results = Vec::new();
        for doc_result in doc_results {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_result.address)?;
            let filepath = doc.get_first(*self.filepath()).unwrap().as_str().unwrap();
            let fullpath = if let Some(root_dir) = opts.root_dir.as_deref() {
                PathBuf::from(root_dir).join(filepath)
            } else {
                PathBuf::from(self.meta.for_dir()).join(filepath)
            };

            let mut lines = Vec::new();
            if !opts.filename_only {
                if let Some(mut position_data) = position_map.get_mut(&doc_result.address) {
                    location::positions_to_lines(&self, &fullpath, &mut position_data, &mut lines)?
                };
            }
            results.push(FileSearchResult {
                file: fullpath,
                score: doc_result.score,
                lines,
            });
        }
        Ok(results)
    }
}

impl Display for FileIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Index({:?})", self.meta.for_dir)?;
        writeln!(f, "  version: {}", self.meta.version)?;
        if let Some(index_dir) = &self.cache_dir {
            writeln!(f, "  location: {:?}", index_dir)?;
            writeln!(
                f,
                "  last updated: {}",
                DateTime::<Local>::from(self.meta.last_update)
            )?;
        } else {
            writeln!(f, "  location: in-memory")?;
        }
        for field in serde_json::to_string_pretty(&self.meta.config)
            .unwrap_or("".to_string())
            .split("\n")
        {
            writeln!(f, "  {}", field.replace(" =", ":"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn file_index_options_default() {
        let opts = FileIndexOptions::default();
        assert!(!opts.follow);
        assert!(!opts.hidden);
        assert!(opts.ignore_files);
        assert_eq!(opts.language, LanguageRef::English);
        assert_eq!(opts.threads, 0);
    }

    #[test]
    fn file_search_options_default() {
        let opts = FileSearchOptions::default();
        assert_eq!(opts.limit, 1000);
        assert_eq!(opts.threshold, 0.0);
        assert!(!opts.filename_only);
        assert!(opts.root_dir.is_none());
    }

    #[test]
    fn file_metadata_new() {
        let tmp = TempDir::new().unwrap();
        let opts = FileIndexOptions::default();
        let meta = FileMetadata::new(opts.clone(), tmp.path()).unwrap();
        assert_eq!(meta.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(meta.for_dir(), fs::canonicalize(tmp.path()).unwrap());
    }

    #[test]
    fn file_metadata_serialization_round_trip() {
        let tmp = TempDir::new().unwrap();
        let opts = FileIndexOptions::default();
        let meta = FileMetadata::new(opts, tmp.path()).unwrap();
        let json = serde_json::to_string(&meta).unwrap();
        let restored: FileMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta.version, restored.version);
        assert_eq!(meta.config, restored.config);
    }

    #[test]
    fn file_index_display_format() {
        let tmp = TempDir::new().unwrap();
        let opts = FileIndexOptions::default();
        let index = FileIndex::get_or_create(tmp.path(), Some(tmp.path()), &opts).unwrap();
        let display = format!("{}", index);
        assert!(display.contains("Index("));
        assert!(display.contains("version:"));
        assert!(display.contains("location:"));
    }
}
