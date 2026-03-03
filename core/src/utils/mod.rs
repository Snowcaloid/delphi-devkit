use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf, Component};

mod document;
pub use document::*;

/// Normalise a path by:
///   1. Resolving `.` and `..` segments purely (without touching the filesystem).
///   2. Stripping the Windows extended-length prefix (`\\?\`) if present.
///
/// `std::fs::canonicalize()` requires the path to exist and on Windows returns
/// paths like `\\?\C:\Users\…`, which are valid but ugly in config files. This
/// function works on any path string regardless of whether the target exists.
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();

    // Strip \\?\ prefix first so Component parsing sees a normal path.
    let path = {
        let s = path.to_string_lossy();
        if s.starts_with(r"\\?\") {
            PathBuf::from(&s[4..])
        } else {
            path.to_path_buf()
        }
    };

    // Resolve `.` and `..` using a stack-based approach.
    let mut components: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => { /* skip `.` */ }
            Component::ParentDir => {
                // Pop the last normal component if possible;
                // if we're already at a root, just ignore the `..`.
                match components.last() {
                    Some(Component::Normal(_)) => { components.pop(); }
                    Some(Component::RootDir) | Some(Component::Prefix(_)) => { /* can't go above root */ }
                    _ => { components.push(component); }
                }
            }
            _ => { components.push(component); }
        }
    }

    if components.is_empty() {
        PathBuf::from(".")
    } else {
        components.iter().collect()
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