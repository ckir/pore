# Spec: Analytics & Structured Data (Sub-Project 3)

## Objective
Enhance `pore` beyond simple text search by adding analytical capabilities (aggregations) and structured data (JSON) support. Specifically, we want to allow users to aggregate search results by file extension (`ext`) and we want the `GenericIndex` to natively support indexing arbitrary JSON objects leveraging Tantivy 0.26's dynamic JSON fields.

## Tech Stack
- **Rust** (1.75+)
- **tantivy 0.26.1** (Specifically `tantivy::aggregation` and JSON fields)
- **clap** (for CLI parsing)

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
- `pore-core/src/common.rs` - Update schema generation to include `ext` as a `STRING | FAST` field for files. Update generic schema to include a `json_data` field if requested.
- `pore-core/src/file.rs` - Parse the file extension during indexing and add it to `PoreFileEntry`.
- `pore-core/src/generic.rs` - Add JSON field support to the schema builder for `GenericIndex`.
- `pore-bin/src/args.rs` - Add `--aggregate <field>` CLI argument.
- `pore-bin/src/main.rs` & `pore-bin/src/output.rs` - Handle the `--aggregate` flag to print term frequencies (e.g. counts per file extension) instead of or alongside file matches.

## Code Style
```rust
// Example of Aggregation in Tantivy
use tantivy::aggregation::agg_req::Aggregation;
use tantivy::aggregation::agg_req::BucketAggregationType;
use tantivy::aggregation::agg_req::TermsAggregation;
use tantivy::aggregation::AggregationCollector;

let mut aggs = HashMap::new();
aggs.insert(
    "ext_counts".to_string(),
    Aggregation::Bucket(BucketAggregationType::Terms(TermsAggregation {
        field: "ext".to_string(),
        ..Default::default()
    })),
);

let collector = AggregationCollector::from_aggs(aggs, Default::default());
let agg_results = searcher.search(query, &collector)?;
```

## Testing Strategy
- **Unit Tests:** Add tests for `args.rs` to ensure `--aggregate` is parsed.
- **Integration Tests:** Add tests to `generic_index_integration.rs` to index JSON objects and query them using JSON paths (e.g. `data.author: "Steve"`). Add tests to verify that aggregations return correct term counts for `ext`.
- **Backward Compatibility:** Default search without `--aggregate` should behave exactly as before.

## Boundaries
- **Always:** Use Tantivy's native `AggregationCollector` rather than manually iterating and counting results.
- **Always:** Ensure fast fields are used for any fields we want to aggregate on (like `ext`).
- **Ask first:** If we need to drastically alter the JSON output format of `pore` when `--aggregate` is used. (We will default to printing a summary table or JSON object of bucket counts).
- **Never:** Drop existing search functionality or overwrite the `filepath` and `modified` logic we added in Sub-Project 2.

## Success Criteria
1. Running `pore "foo" --aggregate ext` prints a breakdown of how many times "foo" appears per file extension (e.g., `rs: 15, md: 3`).
2. The `GenericIndex` can accept and query JSON fields using Tantivy's native JSON schema type.
3. All tests pass successfully.

## Open Questions
- Should `--aggregate` replace the normal search output, or be printed at the very end of the results? (Assumption: Replace the normal search output if `--aggregate` is provided, focusing purely on analytics).
