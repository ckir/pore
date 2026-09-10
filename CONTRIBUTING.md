# Contributing to pore

Thanks for helping out. This file covers the things that are not obvious from reading the
code — mostly the places where a green local build can still mean a broken one.

pore is MIT licensed; contributions are accepted under the same terms.

## Getting set up

You need a stable Rust toolchain. Three extra tools are used by CI and are worth having
locally so you see the same results:

```bash
cargo binstall -y cargo-nextest   # test runner
cargo binstall -y typos-cli       # spell check
cargo binstall -y cargo-release   # releases only
```

They are declared in [`.claude/recommended-tools.json`](.claude/recommended-tools.json)
with the reason for each.

## Running the gate

CI runs four jobs. Run the same thing locally before pushing:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
typos
cargo nextest run --workspace -E 'not binary(ansi_e2e)'   # unit + integration
cargo nextest run -p pore --test ansi_e2e                 # e2e
cargo test --workspace --doc                              # doctests
cargo build -p pore-lua --no-default-features --features lua55,module
```

Three of those lines exist because of a trap, so please do not collapse them:

- **`cargo nextest` does not run doctests.** It has no runner for them. `cargo nextest run`
  alone is a *weaker* gate than `cargo test` — 131 tests versus 132 — so the doctest line
  is separate and must stay.
- **The e2e tests are split out** with `-E 'not binary(ansi_e2e)'` so a rendering
  regression is attributable at a glance rather than buried in the full run. The two sets
  are disjoint, so nothing runs twice.
- **The Lua module is invisible to every other command.** See below.

### The Lua module compiles only under one feature combination

`pore-lua`'s `#[mlua::lua_module]` entry point — the exported `get_file_index` and
`get_index` bindings — only compiles with mlua's `module` feature, and `module` is
mutually exclusive with the default `vendored`. So `cargo build`, `cargo clippy
--all-targets` and `cargo test` **all skip that code entirely**.

This is not theoretical: a call with a phantom argument once sat on `master` while CI was
green, because nothing type-checked it. If you touch `pore-lua/src/lib.rs`, run:

```bash
cargo build -p pore-lua --no-default-features --features lua55,module
```

The parsers in those bindings are also separate from the CLI's. If you change query
handling in `pore-bin/src/main.rs`, check whether `pore-lua` needs the same change —
`allow_regexes()` has already drifted once.

## Tests

| Location | What it is |
|---|---|
| `#[cfg(test)]` in `src/` | unit tests, including private access |
| `pore-core/tests/` | integration through the public API |
| `pore-bin/tests/cli_integration.rs` | spawns the real binary |
| `pore-bin/tests/ansi_e2e.rs` | spawns the real binary and asserts on raw stdout bytes |

A few conventions that have earned their place:

**Write the test first and watch it fail.** A test that passes the moment you write it has
proven nothing. Several tests here were verified by deliberately reintroducing the bug and
confirming the test catches it — worth doing for anything guarding a regression.

**Do not depend on the ambient environment.** Tests that read `HOME`, `USERPROFILE` or
`RUST_BACKTRACE` must set *and* remove what they need explicitly. Two tests once passed on
Windows and failed on Linux CI because they relied on `USERPROFILE` existing; another would
have broken had it assumed `RUST_BACKTRACE` was unset, since the CI test job sets it to `1`.

**Terminal output needs byte-level assertions.** Most CLI tests assert on `--json`, which
cannot see colour leaking into piped output or highlight markup reaching the user verbatim.
Both have happened. That is what `ansi_e2e.rs` is for.

## Things that will fail CI in a surprising way

- **`pore.example.toml` must list every `SearchConfig` field.** The
  `example_file_is_complete` test enforces it, so adding a config field means adding it to
  the example too. It has caught two omissions already.
- **The README's usage block is generated.** `.github/update_readme.py` rewrites everything
  between the `Usage` line and the next ``` fence from `--help` output, and a workflow
  commits the result on every push. Edit the clap definitions, not that block. Everything
  else in the README is hand-written and safe to edit.
- **`typos` runs over prose too.** `typos.toml` excludes `docs/superpowers/plans/*.txt`,
  which are captured diff artifacts — correcting a typo inside one would falsify a record.

## Platform notes

pore resolves its config and cache directories via `pore-bin/src/paths.rs`: the `XDG_*`
variable, then `HOME`, then `USERPROFILE`. `HOME` is deliberately tried first — Git Bash
and WSL point the two at different places, and existing installs depend on `HOME` winning.
An empty variable counts as unset, or a blank `XDG_CONFIG_HOME` would resolve the config
path to the process's current directory.

Because Git Bash sets `HOME`, Windows-specific path bugs hide during development. If you
touch path resolution, test with the variables genuinely removed rather than injected.

## Commits and pull requests

- Explain **why** in the commit message, not just what. If you fixed something subtle, say
  how you know it is fixed — the command you ran and what it printed.
- Keep every commit building. A non-building commit in history breaks `git bisect`.
- Update `CHANGELOG.md` under `## [Unreleased]` for anything user-visible.
- Open defects go in `ROADMAP.md`, which distinguishes what was **verified by measurement**
  from what is a **proposal**. Please keep that distinction.
- Pull requests run the full CI gate.

## Releases

Maintainers only, and the working tree must be clean:

```bash
cargo release patch --workspace       # dry run, prints what it would do
cargo release patch --workspace -x    # execute
```

All four crates move in lockstep and a single `v{version}` tag is pushed, which is what
`release.yml` triggers on. Nothing is published to crates.io — the `pore` name there
belongs to an unrelated project — so `publish = false` is correct and should stay.

The GitHub release body is generated from the `## [<version>]` section of `CHANGELOG.md`,
so that section is what readers actually see — write it as notes, not as a changelog of
commit subjects. If a release warrants more than the changelog gives, put the long form in
`docs/releases/<version>.md` and apply it with
`gh release edit v<version> --notes-file docs/releases/<version>.md`;
[`docs/releases/v0.3.0.md`](docs/releases/v0.3.0.md) is the worked example.
