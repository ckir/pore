# Spec: Search Output & Experience (Sub-Project 2)

## Objective
To improve the end-user search experience of `pore` by providing rich contextual snippets for matched results and enabling sorting capabilities. Currently, `pore` re-tokenizes entire files to print full matching lines, which is slow and verbose. We will transition to Tantivy's native `SnippetGenerator` to generate concise, highlighted context. Furthermore, we will add support for sorting search results by file modification date or file path.

## Tech Stack
- **Rust** (1.75+)
- **tantivy 0.26.1**
- **clap** (for CLI argument parsing in `pore-bin`)

## Commands
```bash
# Testing
cargo test -p pore-core
cargo test -p pore-bin

# Linting
cargo clippy -- -D warnings
cargo fmt
```

## Project Structure
- `pore-core/src/common.rs` - Update schema generation to support `FAST` fields.
- `pore-core/src/file.rs` - Add `modified` field to `PoreFileEntry`. Update `search` method to use `SnippetGenerator` instead of `location::positions_to_lines`.
- `pore-core/src/location.rs` - Can be heavily stripped down or removed since `SnippetGenerator` handles snippet extraction natively.
- `pore-bin/src/args.rs` - Add `--sort` CLI argument.
- `pore-bin/src/main.rs` & `pore-bin/src/output.rs` - Wire up the new sort argument and render the new snippets with ANSI colors.

## Code Style
```rust
// Example of snippet generation inside FileIndex::search
let snippet_generator = tantivy::SnippetGenerator::create(
    &searcher,
    query,
    *self.contents()
)?;

let snippet = snippet_generator.snippet_from_doc(&doc);
let highlighted_text = snippet.to_html()
    .replace("<b>", "\x1b[31m")
    .replace("</b>", "\x1b[0m"); // Red matches for terminal output
```

## Testing Strategy
- **Unit Tests:** Add tests for `args.rs` to ensure `--sort date` and `--sort path` are parsed correctly.
- **Integration Tests:** Update `generic_index_integration.rs` to verify that `search` returns snippet fragments instead of full lines, and verify that documents sorted by `date` return in the expected order.
- **Backward Compatibility:** Ensure existing tests that expect `FileSearchResult` still compile, though they will assert on snippets instead of full lines.

## Boundaries
- **Always:** Use `FAST` fields for sorting (sorting on un-fast fields is slow or impossible in Tantivy).
- **Always:** Pass the `sort` configuration explicitly down to `FileIndex::search` so the searcher can use `TopDocs::with_limit().order_by_fast_field()`.
- **Ask first:** If we need to perform an index migration. (We will bump the index version or rely on the user to re-index).
- **Never:** Use heavy HTML parsers to strip `<b>` tags; simple string replacement is sufficient for terminal output here.

## Success Criteria
1. Running `pore "foo"` prints contextual snippets instead of full lines.
2. The snippet matches are highlighted in color in the terminal.
3. Running `pore "foo" --sort date` returns results ordered by modification time.
4. Running `pore "foo" --sort path` returns results ordered alphabetically by path.
5. All tests pass.

## Open Questions
- Do we want to support ascending/descending sorts? (Defaulting to relevance descending for normal search, and ascending for `path`/`date` is probably fine for this phase).
