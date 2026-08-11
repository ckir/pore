# Analytics & Structured Data Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enhance `pore` by allowing users to aggregate search results (e.g. by file extension) and query structured JSON data natively.

**Architecture:** We will add an `ext` fast string field to `FileIndex` to store file extensions for aggregation. We will integrate Tantivy's `AggregationCollector` to process the `--aggregate` CLI flag. Finally, we will add an optional `json_data` schema field to `GenericIndex` to support arbitrary structured data.

**Tech Stack:** Rust, tantivy 0.26.1 (with `tantivy::aggregation` module), clap

---

### Task 1: Add `ext` Fast Field to `FileIndex`

**Files:**
- Modify: `pore-core/src/common.rs`
- Modify: `pore-core/src/file.rs`

- [ ] **Step 1: Update Schema in `common.rs`**
Modify `create_index` in `pore-core/src/common.rs`. Add a new text field named `ext` configured as `STRING | FAST` (similar to how `filepath` or `modified` are set up) to allow aggregations.

- [ ] **Step 2: Update `FileIndex` struct in `file.rs`**
In `pore-core/src/file.rs`, update `FileIndex` to hold the new `ext` field (type `Field`). Extract it inside `get_or_create` using `index.schema().get_field("ext").unwrap()`. Add a public getter for it: `pub fn ext(&self) -> &Field`.

- [ ] **Step 3: Update `PoreFileEntry` in `file.rs`**
Modify `PoreFileEntry` to include `ext_field: Field` and `ext: String`. Since Tantivy 0.26's `iter_fields_and_values` yields an iterator of `(Field, Value)`, populate `ext` using `OwnedValue::Str` or whatever type `PoreFileEntry` currently uses for `Value`.
When indexing inside `FileIndex::update()`, extract the file extension from `filepath` (using `filepath.extension().unwrap_or_default().to_string_lossy().to_string()`) and pass it to `PoreFileEntry`.

- [ ] **Step 4: Run tests**
Run: `cargo check -p pore-core --features vendored`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add pore-core/src/common.rs pore-core/src/file.rs
git commit -m "feat: add ext fast field to file index for aggregations"
```

---

### Task 2: Implement JSON Field in `GenericIndex`

**Files:**
- Modify: `pore-core/src/common.rs`
- Modify: `pore-core/src/generic.rs`

- [ ] **Step 1: Add JSON schema support in `common.rs`**
Update `create_index` signature to accept a boolean `add_json_field` (or unconditionally add a JSON field named `json_data`). If requested, add it:
```rust
    if add_json_field {
        schema_builder.add_json_field("json_data", tantivy::schema::TEXT | tantivy::schema::STORED | tantivy::schema::FAST);
    }
```
Update all callers of `create_index` (in `file.rs` and `generic.rs`) to pass this new parameter appropriately (e.g., `false` for `file.rs`, `true` for `generic.rs`).

- [ ] **Step 2: Update `GenericIndex` in `generic.rs`**
Update `GenericIndex` struct to hold `json_data: Option<Field>`. Initialize it in `get_or_create`. 
Add a getter: `pub fn json_data(&self) -> Option<&Field> { self.json_data.as_ref() }`.

- [ ] **Step 3: Run tests**
Run: `cargo check -p pore-core --features vendored`
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add pore-core/src/common.rs pore-core/src/generic.rs pore-core/src/file.rs
git commit -m "feat: add JSON field support to GenericIndex"
```

---

### Task 3: Aggregation CLI Interface (`--aggregate`)

**Files:**
- Modify: `pore-bin/src/args.rs`
- Modify: `pore-bin/src/config.rs`
- Modify: `pore-bin/src/main.rs`
- Modify: `pore-core/src/file.rs`

- [ ] **Step 1: Update CLI arguments in `args.rs`**
Add an `--aggregate` (or `-a`) flag to `SearchArgs`:
```rust
    /// Aggregate results by field (e.g., 'ext')
    #[arg(short, long)]
    pub aggregate: Option<String>,
```
Wire it to `SearchConfig` and ultimately pass it as an `Option<String>` down to `FileSearchOptions`.

- [ ] **Step 2: Perform Aggregation in `FileIndex::search`**
If `opts.aggregate` is `Some(field_name)`, bypass standard search logic and use `AggregationCollector`.
```rust
use tantivy::aggregation::agg_req::{Aggregation, BucketAggregationType, TermsAggregation};
use tantivy::aggregation::AggregationCollector;

if let Some(agg_field) = &opts.aggregate {
    let mut aggs = std::collections::HashMap::new();
    aggs.insert(
        agg_field.clone(),
        Aggregation::Bucket(BucketAggregationType::Terms(TermsAggregation {
            field: agg_field.clone(),
            ..Default::default()
        })),
    );
    let collector = AggregationCollector::from_aggs(aggs, Default::default());
    let agg_results = searcher.search(query, &collector)?;
    
    // Convert agg_results to JSON and return them.
    // For simplicity, we can create a new struct `AggregationResult` or just panic for now if not implemented.
}
```
*Note: Due to return type signatures in `search()`, you may need to introduce a new method `aggregate()` on `FileIndex` that returns JSON, or modify `FileSearchResult` to support an `Aggregations` enum variant. The easiest approach is a dedicated `pub fn aggregate(&self, query: &dyn Query, opts: &FileSearchOptions) -> Result<serde_json::Value, anyhow::Error>`.*

- [ ] **Step 3: Update `main.rs`**
In `main.rs`, if `conf.aggregate` is Some, call `index.aggregate(...)` instead of `index.search(...)`. Print the resulting JSON directly to stdout using `serde_json::to_string_pretty`.

- [ ] **Step 4: Run tests**
Run: `cargo test --features vendored`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add pore-bin/ pore-core/
git commit -m "feat: implement --aggregate CLI flag using Tantivy AggregationCollector"
```
