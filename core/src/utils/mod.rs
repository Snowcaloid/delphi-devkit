use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};

mod document;
pub use document::*;

/// Strip the Windows extended-length path prefix (`\\?\`) if present.
///
/// `std::fs::canonicalize()` on Windows returns paths like `\\?\C:\Users\…`
/// which are valid but ugly when stored in config files. This normalises
/// them back to regular paths (e.g. `C:\Users\…`).
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let s = path.as_ref().to_string_lossy();
    if s.starts_with(r"\\?\") {
        PathBuf::from(&s[4..])
    } else {
        path.as_ref().to_path_buf()
    }
}

pub trait FilePath {
    fn get_file_path() -> &'static PathBuf;
}

pub trait Load {
    fn load_from_file(path: &PathBuf) -> Self
    where
        Self: Serialize + Default + for<'de> Deserialize<'de>,
    {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(obj) = ron::from_str(&data) {
                return obj;
            }
        }
        return Self::default();
    }
}