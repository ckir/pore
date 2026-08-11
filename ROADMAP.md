# pore — ROADMAP

Open work, highest-severity first. Each item states what was **verified by measurement** and what is a
**proposal**, so the two are never confused.

---

## 1. `pore` is unusable on a stock Windows box — it hard-requires `HOME`

**Status:** open · **Severity:** blocks all real use on Windows · **Found:** 2026-08-10

### The defect

Both the config-path and the index-cache-path helpers fall back to `HOME` when the matching `XDG_*`
variable is unset, and propagate the lookup failure with `?`:

- `pore-bin/src/config.rs:110-112`
  ```rust
  let mut config_home = env::var("XDG_CONFIG_HOME").unwrap_or("".to_string());
  if config_home.is_empty() {
      config_home = env::var("HOME")? + "/.config";
  }
  ```
- `pore-bin/src/main.rs:116-118` — the same shape for `XDG_CACHE_HOME` / `HOME`.

**`HOME` is not a standard Windows environment variable.** Windows sets `USERPROFILE`; `HOME` exists only
if something else put it there (Git Bash, WSL, a manual user setting). On a machine without it, both
lookups return `VarError::NotPresent`.

### Why it surfaces badly

`VarError::NotPresent` renders as the bare string **`environment variable not found`** — it names neither
the variable nor a remedy. A user sees:

```
Error: environment variable not found
```

with exit code 2, and has nothing to act on.

### Measured blast radius

Verified on Windows 11 with `HOME` unset, against a debug build of `pore 0.1.0`:

| Command | Exit | Result |
|---|---|---|
| `pore --help` | 0 | works |
| `pore --version` | 0 | works |
| `pore --files --in-memory x .` | **2** | `Error: environment variable not found` |
| `pore x .` | **2** | `Error: environment variable not found` |

So **every command that does real work fails**; only the two that touch neither config nor cache survive.
The same binary works correctly when run from Git Bash, because Git Bash sets `HOME` — which is why this
can hide during development on a machine that has it.

### The test suite masks it

`pore-bin/tests/cli_integration.rs:9-15` already documents the problem and works around it:

```rust
/// Returns a Command with `HOME` set to a temp directory.
/// On Windows, `HOME` is not set by default, and pore requires it for
/// config file lookup (`$HOME/.config/pore.toml`).
fn pore_with_home() -> (Command, tempfile::TempDir) {
```

Every integration test that needs a config or an index therefore runs with `HOME` injected. **No test
exercises the stock-Windows path**, so CI is green while the shipped binary is unusable there. The
knowledge was present; only the coverage was missing.

### Proposed fix (not yet implemented)

1. **Resolve platform directories properly instead of hand-rolling XDG.** Use the `dirs` crate (or
   `etcetera`, which models the XDG-vs-Windows split explicitly): `dirs::config_dir()` returns
   `%APPDATA%` on Windows and honours `XDG_CONFIG_HOME` on Unix; `dirs::cache_dir()` does the same for
   `%LOCALAPPDATA%` / `XDG_CACHE_HOME`. Two call sites change.
   - Keep reading `XDG_CONFIG_HOME` / `XDG_CACHE_HOME` first if you want to preserve the documented
     override on all platforms — the change is only to the *fallback*.
   - Decide explicitly whether an existing `HOME` should still win on Windows. Continuing to honour it
     avoids breaking anyone who set it deliberately.
2. **Never surface a bare `VarError`.** Map the failure to a message naming the variable and the
   remedy, e.g. `could not determine a config directory: set XDG_CONFIG_HOME (or HOME)`.
3. **Update the docs.** `README.md` (Config section) and the module doc at `pore-bin/src/config.rs:3`
   both state the path as `${XDG_CONFIG_HOME}` / `$HOME/.config` with no Windows equivalent.

### Acceptance criteria

- [ ] `pore x .` succeeds on Windows with **both** `HOME` and `XDG_CONFIG_HOME` unset.
- [ ] A regression test runs the CLI with `HOME` **removed** (`Command::env_remove("HOME")`) and asserts
      success — not merely that a helper injected one. Without this the defect can silently return.
- [ ] An unresolvable config/cache directory produces an error naming the variable, not
      `environment variable not found`.
- [ ] README's Config section documents the Windows location.

### Provenance

Found while evaluating `pore --files` as an independent oracle for another project's file-discovery
walk. Source citations verified against the working tree at `29c97ad`; behaviour reproduced with the
existing debug build (`pore 0.1.0`). The working tree was dirty at the time of writing — re-confirm
against a clean checkout before starting.
