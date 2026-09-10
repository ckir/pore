//! Base-directory resolution for the config file and the index cache.
//!
//! pore follows the XDG layout, but `HOME` — which the XDG fallback depends on — is not
//! a standard Windows environment variable. Windows sets `USERPROFILE`; `HOME` exists
//! only if something else put it there, such as Git Bash or WSL. Depending on `HOME`
//! alone made every command that touches config or cache fail on a stock Windows box
//! with a bare "environment variable not found".

use anyhow::anyhow;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

/// Returns the value of `var` if it is set and non-empty.
///
/// An empty variable is treated as unset: an exported-but-blank `XDG_CONFIG_HOME` would
/// otherwise resolve the config path to the process's current directory.
fn non_empty_var(var: &str) -> Option<OsString> {
    env::var_os(var).filter(|v| !v.is_empty())
}

/// Resolves a base directory, preferring `xdg_var` and falling back to `<home>/<suffix>`.
///
/// `HOME` is consulted before `USERPROFILE` so that an explicitly set `HOME` keeps
/// winning. That is what every existing install relies on, including Git Bash and WSL,
/// where the two point at different places.
///
/// # Errors
///
/// Returns an error naming every variable it consulted when none of them is set, rather
/// than propagating `VarError::NotPresent`, which renders as "environment variable not
/// found" and names neither the variable nor a remedy.
pub fn resolve_base_dir(xdg_var: &str, suffix: &str) -> Result<PathBuf, anyhow::Error> {
    if let Some(dir) = non_empty_var(xdg_var) {
        return Ok(PathBuf::from(dir));
    }
    for var in ["HOME", "USERPROFILE"] {
        if let Some(home) = non_empty_var(var) {
            return Ok(PathBuf::from(home).join(suffix));
        }
    }
    Err(anyhow!(
        "cannot determine the {suffix} directory: none of {xdg_var}, HOME or USERPROFILE is set. \
         Set {xdg_var} to choose the location explicitly."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// These mutate process-wide environment state, so they are kept in one test to
    /// avoid racing the other tests in this binary.
    #[test]
    fn resolution_order_and_error() {
        let saved: Vec<_> = ["PORE_TEST_XDG", "HOME", "USERPROFILE"]
            .iter()
            .map(|v| (*v, env::var_os(v)))
            .collect();

        env::remove_var("PORE_TEST_XDG");
        env::remove_var("HOME");
        env::remove_var("USERPROFILE");

        // Nothing set: the error names each variable consulted.
        let err = resolve_base_dir("PORE_TEST_XDG", ".config")
            .expect_err("no variables set must not resolve")
            .to_string();
        assert!(err.contains("PORE_TEST_XDG"), "{err}");
        assert!(err.contains("USERPROFILE"), "{err}");

        // USERPROFILE alone is enough -- this is the Windows case that used to fail.
        env::set_var("USERPROFILE", "/up");
        assert_eq!(
            resolve_base_dir("PORE_TEST_XDG", ".config").unwrap(),
            PathBuf::from("/up").join(".config")
        );

        // HOME wins over USERPROFILE.
        env::set_var("HOME", "/home");
        assert_eq!(
            resolve_base_dir("PORE_TEST_XDG", ".config").unwrap(),
            PathBuf::from("/home").join(".config")
        );

        // The XDG variable wins over both, and is used as-is with no suffix.
        env::set_var("PORE_TEST_XDG", "/xdg");
        assert_eq!(
            resolve_base_dir("PORE_TEST_XDG", ".config").unwrap(),
            PathBuf::from("/xdg")
        );

        // An empty value counts as unset, or the path would resolve to the cwd.
        env::set_var("PORE_TEST_XDG", "");
        assert_eq!(
            resolve_base_dir("PORE_TEST_XDG", ".config").unwrap(),
            PathBuf::from("/home").join(".config")
        );

        for (var, val) in saved {
            match val {
                Some(v) => env::set_var(var, v),
                None => env::remove_var(var),
            }
        }
    }
}
