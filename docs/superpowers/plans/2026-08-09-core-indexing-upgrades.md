# Core Indexing Upgrades Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement zero-copy indexing via the `Document` trait and enable/verify Advanced Querying (Regex) for the `pore` CLI tool.

**Architecture:** We will replace `TantivyDocument` allocations in `file.rs` with a custom `PoreFileEntry` struct that implements the `tantivy::schema::Document` trait, avoiding string cloning. We will also update the documentation to showcase the new regex querying features provided by Tantivy 0.26.

**Tech Stack:** Rust, tantivy 0.26.1

---

### Task 1: Create `PoreFileEntry` and Zero-Copy Indexing in `file.rs`

**Files:**
- Modify: `pore-core/src/file.rs`

- [ ] **Step 1: Write the `PoreFileEntry` struct and `Document` implementation**
Add this to the top of `pore-core/src/file.rs` (below imports):

```rust
use tantivy::schema::{Document, Field};

pub struct PoreFileEntry<'a> {
    filepath_field: Field,
    filepath: &'a str,
    contents_field: Field,
    contents: &'a str,
}

impl<'a> Document for PoreFileEntry<'a> {
    type Value<'b> = tantivy::schema::ReferenceValue<'b> where Self: 'b;
    type ValueIter<'b> = std::vec::IntoIter<(Field, Self::Value<'b>)> where Self: 'b;

    fn iter_fields_and_values(&self) -> Self::ValueIter<'_> {
        vec![
            (self.filepath_field, tantivy::schema::ReferenceValue::Str(self.filepath)),
            (self.contents_field, tantivy::schema::ReferenceValue::Str(self.contents)),
        ].into_iter()
    }
}
```

- [ ] **Step 2: Update `update` method to use `PoreFileEntry`**
Modify the `update` method in `pore-core/src/file.rs` (around line 361) to use `PoreFileEntry` instead of the `doc!()` macro:

```rust
// Replace:
// let doc = doc!(
//     self.filepath => String::from(filepath.to_string_lossy()),
//     self.contents => contents,
// );
// let _ = index_writer.add_document(doc);

// With:
let filepath_str = filepath.to_string_lossy();
let doc = PoreFileEntry {
    filepath_field: self.filepath,
    filepath: &filepath_str,
    contents_field: self.contents,
    contents: &contents,
};
let _ = index_writer.add_document(doc);
```

- [ ] **Step 3: Check compilation**
Run: `cargo check -p pore-core`
Expected: Passes without errors (if `ReferenceValue` needs to be imported differently, fix it by checking `tantivy::schema` exports).

- [ ] **Step 4: Commit**
```bash
git add pore-core/src/file.rs
git commit -m "perf: use zero-copy indexing with custom Document trait in file.rs"
```

### Task 2: Advanced Querying Tests and Documentation

**Files:**
- Modify: `pore-core/tests/generic_index_integration.rs`
- Modify: `README.md`

- [ ] **Step 1: Write a test for regex queries**
Add this test to `pore-core/tests/generic_index_integration.rs`:

```rust
#[test]
fn test_regex_query() {
    let mut index = GenericIndex::new(None, "en".into()).unwrap();
    
    let mut doc = tantivy::TantivyDocument::default();
    doc.add_text(index.id_field(), "1");
    doc.add_text(index.schema().get_field("body").unwrap(), "The big bad wolf");
    index.add_documents(vec![doc]).unwrap();
    
    let results = index.search("/b.* wolf/", 10, 0.0).unwrap();
    assert_eq!(results.len(), 1);
}
```

- [ ] **Step 2: Run the test to verify Regex support in Tantivy 0.26**
Run: `cargo test -p pore-core test_regex_query`
Expected: PASS (Tantivy 0.26 supports regex natively in `QueryParser`).

- [ ] **Step 3: Update README.md with Regex usage**
Modify `README.md` around line 13 to highlight regex support:

```markdown
If you can express what you're looking for as a regular expression or exact text
string, use ripgrep. If you want something more like a Google search, use pore.

**New in Pore (Tantivy 0.26):**
Pore now supports native regular expressions and field grouping in your queries!
- Regex searches: `pore "/b.* wolf/"`
- Field grouping: `pore "path:(src AND *.rs)"`
- Wildcards: `pore "*foo"`
```

- [ ] **Step 4: Commit**
```bash
git add pore-core/tests/generic_index_integration.rs README.md
git commit -m "docs: add regex query test and update README for advanced querying"
```
