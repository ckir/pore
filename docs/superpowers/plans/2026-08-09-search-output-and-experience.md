# Search Output & Experience Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate SnippetGenerator for highlighted terminal output and add FAST fields to support `--sort date` and `--sort path`.

**Architecture:** We will replace the manual file tokenization in `location.rs` with Tantivy's native `SnippetGenerator`. We will also add a `modified` field to the index schema and mark both `filepath` and `modified` as `FAST` fields so Tantivy can sort by them efficiently. 

**Tech Stack:** Rust, tantivy 0.26.1, clap

---

### Task 1: Add FAST Fields to Schema and `PoreFileEntry`

**Files:**
- Modify: `pore-core/src/common.rs`
- Modify: `pore-core/src/file.rs`

- [ ] **Step 1: Update Schema in `common.rs`**
Modify `create_index` in `pore-core/src/common.rs` to add `FAST` to the `id_field` and add a new `modified` field:

```rust
use tantivy::schema::{FAST, INDEXED, STRING, STORED};

// Around line 166:
    let mut schema_builder = Schema::builder();
    // Add FAST to id_field (which is filepath)
    schema_builder.add_text_field(id_field, STRING | STORED | FAST);
    // Add a new modified date field
    schema_builder.add_u64_field("modified", INDEXED | FAST);
```

- [ ] **Step 2: Update `FileIndex` struct in `file.rs`**
In `pore-core/src/file.rs`, update `FileIndex` to hold the `modified` field:

```rust
pub struct FileIndex {
    index: Index,
    cache_dir: Option<PathBuf>,
    meta: FileMetadata,
    filepath: Field,
    contents: Field,
    modified: Field,
}
```

- [ ] **Step 3: Update `FileIndex::get_or_create` in `file.rs`**
In `pore-core/src/file.rs`, inside `get_or_create`, fetch the `modified` field:

```rust
        let modified = index
            .schema()
            .get_field("modified")
            .expect("No field named 'modified'");
        Ok(Self {
            index,
            cache_dir: cache_dir.map(|p| fs::canonicalize(p).unwrap()),
            meta,
            filepath,
            contents,
            modified,
        })
```

- [ ] **Step 4: Update `PoreFileEntry` in `file.rs` to support `modified`**
Modify `PoreFileEntry` to include `modified_field` and `modified: u64`. Since you need to yield both `&str` and `u64` from `iter_fields_and_values`, you must change `type Value<'a>` to `tantivy::schema::OwnedValue` (or a custom/reference enum if Tantivy 0.26 provides one, such as `tantivy::schema::ReferenceValue`). If you use `OwnedValue`, you can use `OwnedValue::Str` and `OwnedValue::U64`. 
*Note: Ensure you satisfy the `Document` trait bounds for Tantivy 0.26.*

Update the `update()` method where `PoreFileEntry` is instantiated to populate the `modified` field using `modified.timestamp() as u64`.

- [ ] **Step 5: Run tests**
Run: `cargo check -p pore-core`
Expected: PASS

- [ ] **Step 6: Commit**
```bash
git add pore-core/src/common.rs pore-core/src/file.rs
git commit -m "feat: add FAST fields and modified timestamp to index schema"
```

---

### Task 2: Replace Manual Line Extraction with `SnippetGenerator`

**Files:**
- Delete: `pore-core/src/location.rs`
- Modify: `pore-core/src/lib.rs` (remove `pub mod location;` if present)
- Modify: `pore-core/src/file.rs`

- [ ] **Step 1: Delete `location.rs`**
Remove `pore-core/src/location.rs` and its module declaration in `pore-core/src/lib.rs`.

- [ ] **Step 2: Update `FileSearchResult` in `file.rs`**
Replace `lines: Vec<Line>` with `snippets: Vec<String>` in `FileSearchResult` and its `new` method. You can completely remove the `Line` struct. Update the `IntoLua` implementation for `FileSearchResult` to use `snippets`.

- [ ] **Step 3: Update `FileIndex::search` to use `SnippetGenerator`**
Modify the `search` method to remove `location::get_search_results` and `location::positions_to_lines`. Instead, use `tantivy::SnippetGenerator`:

```rust
        let snippet_generator = tantivy::SnippetGenerator::create(
            &searcher,
            query,
            *self.contents()
        )?;

        let mut results = Vec::new();
        for doc_result in doc_results {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_result.address)?;
            let filepath = doc.get_first(*self.filepath()).unwrap().as_str().unwrap();
            let fullpath = if let Some(root_dir) = opts.root_dir.as_deref() {
                PathBuf::from(root_dir).join(filepath)
            } else {
                PathBuf::from(self.meta.for_dir()).join(filepath)
            };

            let mut snippets = Vec::new();
            if !opts.filename_only {
                let snippet = snippet_generator.snippet_from_doc(&doc);
                if !snippet.fragments().is_empty() {
                    snippets.push(snippet.to_html());
                }
            }
            results.push(FileSearchResult {
                file: fullpath,
                score: doc_result.score,
                snippets,
            });
        }
        Ok(results)
```

- [ ] **Step 4: Fix integration tests**
Since we removed `Line` and changed `FileSearchResult`, any tests in `pore-bin` or `pore-core` that assert on `lines` will break. 
Run: `cargo test` and fix the compilation errors by updating test assertions to use `.snippets()` and check for the HTML formatted string (e.g., `"<b>hello</b> world"`).

- [ ] **Step 5: Commit**
```bash
git add pore-core/src/file.rs pore-core/src/location.rs pore-core/src/lib.rs pore-core/tests/
git commit -m "refactor: replace manual line extraction with SnippetGenerator"
```

---

### Task 3: Implement Sorting and ANSI Output in CLI

**Files:**
- Modify: `pore-core/src/file.rs`
- Modify: `pore-bin/src/args.rs`
- Modify: `pore-bin/src/main.rs`
- Modify: `pore-bin/src/output.rs`

- [ ] **Step 1: Update `FileSearchOptions` in `file.rs`**
Add `pub sort: Option<String>` to `FileSearchOptions`. Default it to `None`.
In `FileIndex::search`, use this sort parameter:
```rust
        let top_docs = if let Some(sort_field) = &opts.sort {
            if sort_field == "date" {
                searcher.search(query, &TopDocs::with_limit(opts.limit).order_by_fast_field::<u64>(*self.modified()))?
            } else if sort_field == "path" {
                searcher.search(query, &TopDocs::with_limit(opts.limit).order_by_fast_field::<&str>(*self.filepath()))?
            } else {
                searcher.search(query, &TopDocs::with_limit(opts.limit).order_by_score())?
            }
        } else {
            searcher.search(query, &TopDocs::with_limit(opts.limit).order_by_score())?
        };
```
*(You will need to add a `pub fn modified(&self) -> &Field` getter to `FileIndex`.)*

- [ ] **Step 2: Add `--sort` CLI argument in `pore-bin/src/args.rs`**
Update `SearchArgs` struct:
```rust
    /// Sort results by field (options: date, path). Defaults to relevance score.
    #[arg(short, long)]
    pub sort: Option<String>,
```
And pass it to `SearchConfig` and eventually to `FileSearchOptions`.

- [ ] **Step 3: Update `output.rs` to render snippets with ANSI colors**
In `pore-bin/src/output.rs`, update `print_results` to iterate over `result.snippets()` instead of `result.lines()`.
Replace the `<b>` and `</b>` tags from the snippet HTML with ANSI escape codes for terminal colors (e.g. `\x1b[31m` for red text, `\x1b[0m` to reset). 
Example:
```rust
                let colored_snippet = snippet
                    .replace("<b>", "\x1b[31m")
                    .replace("</b>", "\x1b[0m");
                writeln!(&mut stdout, "  {}", colored_snippet)?;
```
Ensure JSON output correctly outputs the `snippets` array.

- [ ] **Step 4: Run tests**
Run: `cargo test`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add pore-core/src/file.rs pore-bin/src/args.rs pore-bin/src/output.rs pore-bin/src/main.rs pore-bin/src/config.rs
git commit -m "feat: add sort CLI argument and render snippets with ANSI colors"
```
