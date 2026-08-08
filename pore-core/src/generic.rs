//! Generic document indexing for arbitrary text fields.
//!
//! Unlike [`FileIndex`](crate::file::FileIndex) which indexes files on disk,
//! [`GenericIndex`] accepts documents as maps of named text fields (via the
//! [`FieldMap`](crate::field_map::FieldMap) trait). This makes it suitable for
//! indexing data from databases, APIs, or any source that can produce
//! key-value text pairs.
//!
//! # Key types
//! - [`GenericIndex`] — the index handle.
//! - [`IndexOptions`] — configuration (currently only the stemming language).
//! - [`SearchOptions`] — limits and thresholds for search queries.
//! - [`SearchResult`] — a single search result with document ID and score.

use chrono::Utc;
use macros::create_option_copy;
use mlua::IntoLua;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::Query;
use tantivy::ReloadPolicy;

use tantivy::schema::*;
use tantivy::Index;

use crate::common::create_index;
use crate::common::delete_index;
use crate::common::IndexMetadata;
use crate::common::Metadata;
use crate::common::MetadataConfig;
use crate::common::METADATA_FILE;
use crate::field_map::FieldMap;
use crate::language::LanguageRef;

/// A generic full-text index for arbitrary documents.
///
/// Each document is a map of named text fields (see [`FieldMap`](crate::field_map::FieldMap)).
/// One field is designated as the stored ID field; all other fields are indexed
/// as searchable text.
#[derive(Debug, Clone)]
pub struct GenericIndex {
    meta: Metadata<IndexOptions>,
    cache_dir: Option<PathBuf>,
    index: Index,
}

/// Options controlling search result limits for a [`GenericIndex`] query.
#[create_option_copy(SearchOptionsShape)]
#[derive(Debug)]
pub struct SearchOptions {
    /// Maximum number of results to return.
    pub limit: usize,
    /// Minimum score threshold (results below this are excluded).
    pub threshold: f32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions {
            limit: 1000,
            threshold: 0.0,
        }
    }
}

/// Configuration for a [`GenericIndex`].
///
/// Currently only the stemming language is configurable.
#[create_option_copy(IndexOptionsShape)]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IndexOptions {
    /// Language used for stemming.
    pub language: LanguageRef,
}

impl Default for IndexOptions {
    fn default() -> Self {
        IndexOptions {
            language: LanguageRef::English,
        }
    }
}

impl MetadataConfig for IndexOptions {
    fn language(&self) -> LanguageRef {
        self.language
    }
}

/// A single search result from a [`GenericIndex`].
#[derive(Debug, Serialize)]
pub struct SearchResult {
    id: String,
    score: f32,
}

impl IntoLua for SearchResult {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let tbl = lua.create_table()?;
        tbl.set("id", self.id)?;
        tbl.set("score", self.score)?;
        Ok(mlua::Value::Table(tbl))
    }
}

impl GenericIndex {
    /// Returns a reference to the underlying Tantivy index.
    pub fn index(&self) -> &Index {
        &self.index
    }
    /// Deletes the index and its on-disk cache (if any).
    pub fn delete(&self) -> anyhow::Result<bool> {
        delete_index(&self.index, self.cache_dir.as_deref())
    }

    /// Opens an existing index or creates a new one.
    ///
    /// # Parameters
    /// * `id_field` — name of the stored document identifier field.
    /// * `text_fields` — names of the searchable text fields.
    /// * `config` — indexing options.
    /// * `cache_dir` — optional path for persisted index files.
    pub fn get_or_create<I, T>(
        id_field: &str,
        text_fields: I,
        config: &IndexOptions,
        cache_dir: Option<&Path>,
    ) -> Result<Self, anyhow::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let (meta_opt, index) = create_index(cache_dir, config, id_field, text_fields)?;
        let meta = meta_opt.unwrap_or_else(|| Metadata::new(config.clone()));
        Ok(Self {
            index,
            cache_dir: cache_dir.map(|p| fs::canonicalize(p).unwrap()),
            meta,
        })
    }

    /// Returns the stored ID field from the schema.
    ///
    /// The ID field is identified as the first stored field in the schema.
    fn get_id_field(&self) -> anyhow::Result<Field> {
        for (field, entry) in self.index.schema().fields() {
            if entry.is_stored() {
                return Ok(field);
            }
        }
        Err(anyhow!("Could not find stored ID field in index"))
    }

    /// Returns all text (non-stored) fields in the schema.
    pub fn get_text_fields(&self) -> Vec<Field> {
        let mut ret = Vec::new();
        for (field, entry) in self.index.schema().fields() {
            if !entry.is_stored() {
                ret.push(field);
            }
        }
        ret
    }

    /// Deletes documents with the given IDs from the index.
    pub fn delete_documents<I, T>(&mut self, document_ids: I) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut index_writer = self.index.writer::<tantivy::TantivyDocument>(50_000_000)?;
        let id_field = self.get_id_field()?;
        for id in document_ids {
            index_writer.delete_term(Term::from_field_text(id_field, id.into().as_str()));
        }
        index_writer.commit()?;
        Ok(())
    }

    /// Replaces existing documents with updated versions.
    ///
    /// Documents are deleted by ID and then re-added. Each document must
    /// provide its ID via the [`FieldMap`](crate::field_map::FieldMap) trait.
    pub fn update_documents<T: FieldMap>(&mut self, documents: Vec<T>) -> anyhow::Result<()> {
        let id_field = self.get_id_field()?;
        let schema = self.index.schema();
        let id_field_entry = schema.get_field_entry(id_field);
        let id_name = id_field_entry.name();
        let document_ids = documents
            .iter()
            .map(|d| d.get_field(id_name).unwrap().to_owned());
        self.delete_documents(document_ids)?;
        self.add_documents(documents)
    }

    /// Adds documents to the index.
    ///
    /// Each document must implement [`FieldMap`](crate::field_map::FieldMap) to
    /// provide values for all schema fields. The last-update timestamp and
    /// on-disk metadata file are updated after a successful commit.
    pub fn add_documents<T: FieldMap>(&mut self, documents: Vec<T>) -> anyhow::Result<()> {
        let mut index_writer = self.index.writer::<tantivy::TantivyDocument>(50_000_000)?;
        let now = Utc::now();
        for document in documents {
            let mut doc = tantivy::TantivyDocument::default();
            for (field, entry) in self.index.schema().fields() {
                let text = document.get_field(entry.name())?;
                doc.add_text(field, text.as_ref());
            }
            index_writer.add_document(doc)?;
        }
        index_writer.commit()?;
        self.meta.set_last_update(now);
        if let Some(index_dir) = &self.cache_dir {
            fs::write(
                index_dir.join(METADATA_FILE),
                serde_json::to_string(&self.meta)?,
            )?;
        }
        Ok(())
    }

    /// Executes a search query against the index.
    ///
    /// Returns results sorted by score, limited by [`SearchOptions::limit`].
    pub fn search(
        &self,
        query: &Box<dyn Query>,
        opts: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let searcher = reader.searcher();
        let top_docs = searcher.search(query, &TopDocs::with_limit(opts.limit).order_by_score())?;
        let id_field = self.get_id_field()?;
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            if score > opts.threshold {
                let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
                let id = doc.get_first(id_field).unwrap().as_str().unwrap().to_string();
                results.push(SearchResult { id, score });
            }
        }
        Ok(results)
    }
}
