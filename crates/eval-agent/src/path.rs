//! Workspace-relative path safety helpers.
//!
//! The previous implementation used `Path::starts_with` after joining a
//! user-controlled path. Because `workspace.join("../etc/passwd")` is still
//! lexically prefixed by the workspace, that check was always true. These
//! helpers reject `..`, absolute paths, NUL bytes and symlinked components,
//! and open the final component with `O_NOFOLLOW` where available.

use anyhow::{bail, Context, Result};
use regex::Regex;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

static APP_ALIAS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(^|[^A-Za-z0-9_])/app([^A-Za-z0-9_-]|$)").expect("valid /app alias regex")
});

/// Resolve a workspace-relative path without allowing traversal or symlink
/// escapes. The returned path is lexical (it may not exist yet), but every
/// existing ancestor is checked for symlinks.
pub fn safe_join(root: &Path, rel: &str) -> Result<PathBuf> {
    if rel.is_empty() {
        bail!("Empty path is not allowed");
    }
    if rel.contains('\0') {
        bail!("NUL byte in path is not allowed");
    }
    // Backslash is a path separator on Windows; reject it everywhere so a
    // Windows-style absolute path cannot bypass the component checks.
    if rel.contains('\\') {
        bail!("Backslash is not allowed in workspace paths: {rel}");
    }

    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        bail!("Absolute paths are not allowed: {rel}");
    }

    let mut out = root.to_path_buf();
    let mut components = 0usize;
    for component in rel_path.components() {
        match component {
            Component::Normal(part) => {
                if part.is_empty() {
                    bail!("Empty path component is not allowed");
                }
                out.push(part);
                components += 1;
                if let Ok(meta) = fs::symlink_metadata(&out) {
                    if meta.file_type().is_symlink() {
                        bail!("Symlink path component is not allowed: {}", out.display());
                    }
                }
            }
            Component::CurDir => {}
            Component::ParentDir => bail!("Path traversal is not allowed: {rel}"),
            Component::RootDir | Component::Prefix(_) => {
                bail!("Absolute paths are not allowed: {rel}")
            }
        }
    }
    if components == 0 {
        bail!("Path does not name a file: {rel}");
    }
    Ok(out)
}

fn ensure_parent_dirs(root: &Path, target: &Path) -> Result<()> {
    let parent = match target.parent() {
        Some(p) => p,
        None => bail!("Path has no parent: {}", target.display()),
    };
    let mut cur = root.to_path_buf();
    for component in parent
        .strip_prefix(root)
        .with_context(|| format!("Path escaped workspace: {}", target.display()))?
        .components()
    {
        if let Component::Normal(part) = component {
            cur.push(part);
            match fs::symlink_metadata(&cur) {
                Ok(meta) => {
                    if meta.file_type().is_symlink() {
                        bail!(
                            "Symlinked parent directory is not allowed: {}",
                            cur.display()
                        );
                    }
                    if !meta.is_dir() {
                        bail!("Parent path is not a directory: {}", cur.display());
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    fs::create_dir(&cur)
                        .with_context(|| format!("Failed to create directory {}", cur.display()))?;
                }
                Err(e) => {
                    return Err(e).with_context(|| format!("Failed to inspect {}", cur.display()))
                }
            }
        }
    }
    Ok(())
}

/// Write a file below `root`, rejecting symlinks on the final component and on
/// every existing parent component.
pub fn write_no_follow(root: &Path, rel: &str, bytes: &[u8]) -> Result<PathBuf> {
    let target = safe_join(root, rel)?;
    ensure_parent_dirs(root, &target)?;

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(&target)
        .with_context(|| format!("Failed to open {} for writing", target.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("Failed to write {}", target.display()))?;
    file.flush()?;
    Ok(target)
}

/// Read a UTF-8 file below `root` without following a final symlink.
pub fn read_string_no_follow(root: &Path, rel: &str) -> Result<String> {
    let target = safe_join(root, rel)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(&target)
        .with_context(|| format!("Failed to open {} for reading", target.display()))?;
    let mut s = String::new();
    std::io::Read::read_to_string(&mut file, &mut s)?;
    Ok(s)
}

/// Returns true for files that are part of a test harness and must be
/// restored before verification.
pub fn is_test_artifact(rel: &str) -> bool {
    let lower = rel.to_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    lower.starts_with("tests/")
        || lower.contains("/tests/")
        || name == "conftest.py"
        || name == "pytest.ini"
        || name == "tox.ini"
        || name.starts_with("test_")
        || name.ends_with("_test.py")
}

/// Remove Python import-shadowing files that a model could use to make the
/// test suite pass without changing the source.
pub fn remove_shadow_modules(root: &Path) {
    for rel in [
        "unittest.py",
        "conftest.py",
        "sitecustomize.py",
        "usercustomize.py",
        "pytest.ini",
        "tox.ini",
    ] {
        if let Ok(path) = safe_join(root, rel) {
            let _ = fs::remove_file(path);
        }
    }
}

/// Compatibility for datasets that refer to the historical virtual path
/// `/app/`. The workspace already seeds an `app/` directory; rewrite the
/// command to its real absolute path inside the sandbox instead of trying to
/// create `/app` on the read-only host root.
pub fn rewrite_app_alias(command: &str, workspace_root: &Path) -> String {
    let app = workspace_root.join("app");
    if !app.exists() {
        return command.to_string();
    }
    let app = app.to_string_lossy();
    APP_ALIAS_RE
        .replace_all(command, format!("${{1}}{app}${{2}}"))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_dir() {
        let root = tempfile::tempdir().unwrap();
        assert!(safe_join(root.path(), "../etc/passwd").is_err());
        assert!(safe_join(root.path(), "a/../../b").is_err());
    }

    #[test]
    fn rejects_absolute_and_backslash() {
        let root = tempfile::tempdir().unwrap();
        assert!(safe_join(root.path(), "/etc/passwd").is_err());
        assert!(safe_join(root.path(), "C:\\windows\\system32").is_err());
    }

    #[test]
    fn accepts_nested_relative_path() {
        let root = tempfile::tempdir().unwrap();
        let p = safe_join(root.path(), "src/lib.rs").unwrap();
        assert_eq!(p, root.path().join("src/lib.rs"));
    }

    #[test]
    fn write_does_not_follow_symlink() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("secret.txt");
        fs::write(&target, "original").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, root.path().join("link")).unwrap();
        let result = write_no_follow(root.path(), "link", b"overwritten");
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    }

    #[test]
    fn app_alias_is_rewritten_to_workspace_path() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("app/logs")).unwrap();
        let rewritten = rewrite_app_alias("cat /app/logs/error.log && ls /app", root.path());
        assert!(rewritten.contains(&format!("{}/app/logs", root.path().display())));
        assert!(rewritten.contains(&format!("ls {}/app", root.path().display())));
        assert_eq!(
            rewrite_app_alias("echo /application", root.path()),
            "echo /application"
        );
    }

    #[test]
    fn write_creates_directories() {
        let root = tempfile::tempdir().unwrap();
        write_no_follow(root.path(), "tests/test_x.py", b"assert True").unwrap();
        assert!(root.path().join("tests/test_x.py").is_file());
    }
}
