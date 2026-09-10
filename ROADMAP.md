# pore — ROADMAP

Open work, highest-severity first. Each item states what was **verified by measurement** and what is a
**proposal**, so the two are never confused.

---

## 1. Every CLI error prints a stray `<disabled>` line

**Status:** open · **Severity:** cosmetic, but on every error path · **Found:** 2026-09-09

### The defect

Any failing command prints the error and then a bare `<disabled>` line on stderr:

```
$ pore search --aggregate bogus todo .
Error: cannot aggregate on unknown field 'bogus'
<disabled>
```

**Verified by measurement.** Reproduced on two independent error paths — the `--aggregate` validation
above and the pre-existing `--jq` parse error (`pore search --jq '.[bad!!' ...`) — so it is not
specific to any one feature. Isolated to stderr; stdout is clean.

### Cause

`pore-bin/Cargo.toml` enables `anyhow`'s `backtrace` feature. `main` returns `Result`, so anyhow's
`Termination` impl renders the captured backtrace after the message, and `std::backtrace::Backtrace`
displays as the literal string `<disabled>` when backtrace capture is off (the default, i.e. whenever
`RUST_BACKTRACE` is unset). Users therefore see it on every error unless they opt into backtraces.

### Proposal (not yet verified)

Print the error chain explicitly instead of relying on `Termination`: have `main` catch the error, write
`{err:#}` to stderr, and `std::process::exit` with the current code. That keeps the `backtrace` feature
available for `RUST_BACKTRACE=1` debugging while keeping normal output clean. Alternatively drop the
`backtrace` feature if nothing depends on it.

### Acceptance

- [ ] A CLI test asserts stderr on a failing command contains the message and **not** `<disabled>`.
- [ ] `RUST_BACKTRACE=1` still yields a usable backtrace.

### Provenance

Found while verifying the `--aggregate` error path during the Tantivy 0.26 aggregation work. Predates
that work: reproduced on `--jq`, which is untouched by it.
