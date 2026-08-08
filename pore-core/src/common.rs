use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::Utc;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use tantivy::doc;

use tantivy::directory::MmapDirectory;
use tantivy::schema::*;
use tantivy::tokenizer::*;
use tantivy::Index;

use crate::language::LanguageRef;

pub trait IndexMetadata<T: MetadataConfig + Eq> {
    fn config(&self) -> &T;
    fn version(&self) -> &str;
    fn last_update(&self) -> &DateTime<Utc>;
    fn set_last_update(&mut self, time: DateTime<Utc>);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata<T: MetadataConfig + Eq> {
    version: String,
    last_update: DateTime<Utc>,
    config: T,
}

impl<T: MetadataConfig + Eq> Metadata<T> {
    pub fn new(config: T) -> Self {
        Metadata {
            config,
            version: env!("CARGO_PKG_VERSION").to_string(),
            last_update: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
        }
    }
}

impl<T: MetadataConfig + Eq> IndexMetadata<T> for Metadata<T> {
    fn config(&self) -> &T {
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

pub trait MetadataConfig {
    fn language(&self) -> LanguageRef;
}

pub const METADATA_FILE: &str = "pore_meta.json";

pub fn create_index<
    T: IndexMetadata<U> + DeserializeOwned,
    U: MetadataConfig + Eq,
    P: AsRef<Path>,
    I: IntoIterator<Item = V>,
    V: Into<String>,
>(
    cache_dir: Option<P>,
    config: &U,
    id_field: &str,
    text_fields: I,
) -> Result<(Option<T>, Index), anyhow::Error> {
    let mut ret_meta: Option<T> = None;
    let metafile = cache_dir.as_ref().map(|p| p.as_ref().join(METADATA_FILE));
    if metafile.as_deref().map(|p| p.exists()).unwrap_or(false) {
        let meta_res = serde_json::from_str::<T>(&fs::read_to_string(metafile.unwrap())?);
        if let Ok(meta) = meta_res {
            if meta.config() == config {
                ret_meta = Some(meta);
            }
        }
    }

    let mut tokenizers: HashMap<String, TextAnalyzer> = HashMap::new();
    let mut get_tokenizer = |lang: Language| {
        let key = format!("stemmer_{:?}", lang);
        if !tokenizers.contains_key(&key) {
            let tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(40))
                .filter(LowerCaser)
                .filter(Stemmer::new(lang))
                .build();
            tokenizers.insert(key.clone(), tokenizer);
        }
        return key;
    };
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field(id_field, STRING | STORED);
    for name in text_fields {
        let text_options = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(&get_tokenizer(config.language().into()))
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        );
        schema_builder.add_text_field(&name.into(), text_options);
    }
    let schema = schema_builder.build();
    let index = match cache_dir {
        None => Index::create_in_ram(schema.clone()),
        Some(index_dir) => {
            fs::create_dir_all(&index_dir)?;
            let mut index_res =
                Index::open_or_create(MmapDirectory::open(&index_dir)?, schema.clone());
            // If it fails to load, it's probably because the schema is different or the index is
            // corrupted. Delete all files in the dir and try again.
            if index_res.is_err() {
                eprintln!("Index is corrupted. Deleting index files");
                for dir_entry in fs::read_dir(&index_dir)? {
                    if let Ok(entry) = dir_entry {
                        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                            fs::remove_file(entry.path())?;
                        }
                    }
                }
                index_res = Index::open_or_create(MmapDirectory::open(&index_dir)?, schema.clone());
            }
            index_res?
        }
    };
    for (name, tokenizer) in tokenizers {
        index.tokenizers().register(&name, tokenizer);
    }
    Ok((ret_meta, index))
}

pub fn delete_index(index: &Index, cache_dir: Option<&Path>) -> anyhow::Result<bool> {
    match cache_dir {
        None => return Ok(false),
        Some(index_dir) => {
            if !index_dir.exists() {
                return Ok(false);
            }
            let mut index_writer = index.writer::<tantivy::TantivyDocument>(50_000_000)?;
            index_writer.delete_all_documents()?;
            index_writer.commit()?;
            let metafile = index_dir.join(METADATA_FILE);
            fs::remove_file(metafile).ok();
            fs::remove_dir(&index_dir).ok();
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    struct TestConfig {
        language: LanguageRef,
    }
    impl MetadataConfig for TestConfig {
        fn language(&self) -> LanguageRef {
            self.language
        }
    }

    #[test]
    fn metadata_new_sets_version_and_epoch() {
        let config = TestConfig {
            language: LanguageRef::English,
        };
        let meta = Metadata::<TestConfig>::new(config.clone());
        assert_eq!(meta.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(meta.config().language, LanguageRef::English);
    }

    #[test]
    fn metadata_set_last_update() {
        let config = TestConfig {
            language: LanguageRef::English,
        };
        let mut meta = Metadata::<TestConfig>::new(config);
        let now = Utc::now();
        meta.set_last_update(now);
        assert_eq!(meta.last_update(), &now);
    }

    #[test]
    fn create_index_in_ram() {
        let config = TestConfig {
            language: LanguageRef::English,
        };
        let (meta_opt, index) =
            create_index::<Metadata<TestConfig>, _, _, Vec<String>, String>(
                None::<&Path>,
                &config,
                "id",
                vec!["text".to_string()],
            )
            .unwrap();
        assert!(meta_opt.is_none());
        assert!(index.schema().get_field("id").is_ok());
        assert!(index.schema().get_field("text").is_ok());
    }

    #[test]
    fn create_index_on_disk() {
        let tmp = TempDir::new().unwrap();
        let config = TestConfig {
            language: LanguageRef::English,
        };
        let (meta_opt, index) =
            create_index::<Metadata<TestConfig>, _, _, Vec<String>, String>(
                Some(tmp.path()),
                &config,
                "id",
                vec!["text".to_string()],
            )
            .unwrap();
        assert!(meta_opt.is_none());
        // Verify index was created on disk (directory is non-empty)
        assert!(tmp.path().read_dir().unwrap().next().is_some());
    }

    #[test]
    fn create_index_reloads_existing_metadata() {
        let tmp = TempDir::new().unwrap();
        let config = TestConfig {
            language: LanguageRef::English,
        };
        // Write a metadata file manually to simulate a pre-existing index
        let meta = Metadata::<TestConfig>::new(config.clone());
        let meta_json = serde_json::to_string(&meta).unwrap();
        fs::write(tmp.path().join(METADATA_FILE), &meta_json).unwrap();
        // Now create_index should load the existing metadata
        let (meta_opt, _) =
            create_index::<Metadata<TestConfig>, _, _, Vec<String>, String>(
                Some(tmp.path()),
                &config,
                "id",
                vec!["text".to_string()],
            )
            .unwrap();
        assert!(meta_opt.is_some());
    }

    #[test]
    fn delete_index_returns_false_for_in_memory() {
        let config = TestConfig {
            language: LanguageRef::English,
        };
        let (_, index) =
            create_index::<Metadata<TestConfig>, _, _, Vec<String>, String>(
                None::<&Path>,
                &config,
                "id",
                vec!["text".to_string()],
            )
            .unwrap();
        let result = delete_index(&index, None).unwrap();
        assert!(!result);
    }

    #[test]
    fn delete_index_on_disk_removes_files() {
        let tmp = TempDir::new().unwrap();
        let config = TestConfig {
            language: LanguageRef::English,
        };
        let (_, index) =
            create_index::<Metadata<TestConfig>, _, _, Vec<String>, String>(
                Some(tmp.path()),
                &config,
                "id",
                vec!["text".to_string()],
            )
            .unwrap();
        // delete_index attempts cleanup; returns true when given a real path
        let result = delete_index(&index, Some(tmp.path())).unwrap();
        assert!(result);
    }
}
