# Design Spec: Core Indexing & Querying Upgrades

## Objective
Upgrade `pore` to utilize Tantivy's new `Document` trait for zero-copy indexing and explicitly enable new query parser capabilities (Regex, RegexPhraseQuery) introduced between Tantivy 0.16.1 and 0.26.1.

## Scope
This spec covers **Sub-Project 1** of the Tantivy 0.26.1 upgrade plan:
1. Replacing the `TantivyDocument` string copy overhead with a custom `Document` trait implementation.
2. Enabling `RegexPhraseQuery` and verifying standard regex querying support in the CLI.

## Architecture & Design

### 1. Zero-Copy Indexing (Custom `Document` Trait)
Instead of instantiating a `TantivyDocument` which takes ownership of strings, we will implement the `tantivy::schema::Document` trait.

**Component Changes:**
- **`pore-core/src/common.rs` or `file.rs`**: 
  - Define a struct `PoreFileEntry<'a>` that holds references to the schema fields (`Field`) and the text data (`&'a str`).
  - Implement `tantivy::schema::Document` for `PoreFileEntry`. The trait requires implementing a method to iterate over field values.
- **`pore-core/src/file.rs` (`update` function)**:
  - Inside the directory walker, continue using `fs::read_to_string` to read the file contents.
  - Instead of `let doc = doc!(...)` macro, instantiate `PoreFileEntry` with references to the filepath string and the contents string.
  - Pass the `PoreFileEntry` directly to `index_writer.add_document()`.
- **`pore-core/src/generic.rs` (`add_documents`)**:
  - Perform a similar conversion where generic documents are added. A generic wrapper implementing `Document` can be created to borrow from the `FieldMap` trait outputs instead of copying them.

**Trade-offs:**
- Avoids `mmap` to prevent file locking and unsafe UTF-8 conversion overhead.
- We still allocate a single `String` buffer per file, but eliminate the secondary allocation within `TantivyDocument`.

### 2. Advanced Querying Capabilities
Tantivy 0.26.1 automatically supports regex (`/pattern/`) and field grouping natively in the `QueryParser`. We will ensure `RegexPhraseQuery` is enabled if it requires an explicit toggle, and document these capabilities.

**Component Changes:**
- **`pore-core/src/common.rs` or query initialization**:
  - When setting up `tantivy::query::QueryParser`, explicitly call `.set_conjunction_by_default()` (if not already) and enable any flags required for `RegexPhraseQuery`.
- **Documentation (`README.md`)**:
  - Add examples to the README demonstrating the new capabilities: regex searches, leading wildcards, and field grouping.

## Error Handling
- Invalid regex queries provided via the CLI will automatically be caught by `QueryParser::parse_query`. These should be bubbled up to the user with a clean error message (which `pore` currently handles).
- File reading failures will remain unchanged (skipped or logged based on existing walker behavior).

## Testing
- Ensure the generic integration tests (`pore-core/tests/generic_index_integration.rs`) continue to pass.
- Add a test case querying a document with a regex pattern (e.g., `/f[oO]{2}/`).
