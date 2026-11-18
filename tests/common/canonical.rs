//! `fs::canonicalize` without the verbatim prefix of Windows: `\\?\C:\…` refuses `/` and does not
//! compare equal to the path a program builds. macOS still needs the call (`/tmp` → `/private/tmp`).

use std::path::{Path, PathBuf};

pub(crate) fn canonical(path: &Path) -> PathBuf {
    let resolved = std::fs::canonicalize(path).expect("canonical path");
    if cfg!(windows) {
        let text = resolved.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            if !rest.starts_with("UNC\\") {
                return PathBuf::from(rest);
            }
        }
    }
    resolved
}
