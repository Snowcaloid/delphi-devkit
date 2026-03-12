use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf, Component};

mod document;
pub use document::*;

/// Normalise a path by:
///   1. Resolving `.` and `..` segments purely (without touching the filesystem).
///   2. Stripping the Windows extended-length prefix (`\\?\`) if present.
///   3. Converting Delphi-style bare UNC paths (`UNC\server\share\...`) to the
///      proper `\\server\share\...` form.  Delphi project files sometimes write
///      UNC paths without the leading `\\`, which would otherwise be treated as
///      a relative path by the standard library.
///   4. On Windows, remapping `\\server\share\...` UNC paths to a local drive
///      letter (e.g. `Y:\...`) by querying which drive is mapped to that share.
///
/// `std::fs::canonicalize()` requires the path to exist and on Windows returns
/// paths like `\\?\C:\Users\...`, which are valid but ugly in config files.
/// This function works on any path string regardless of whether the target exists.
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();

    // Handle path prefixes in order of precedence:
    //   \\?\C:\...               -> C:\...
    //   \\?\UNC\server\share\... -> \\server\share\...
    //   UNC\server\share\...     -> \\server\share\...  (Delphi project files)
    let path = {
        let s = path.to_string_lossy();
        if s.starts_with(r"\\?\") {
            let stripped = &s[4..];
            if stripped.len() >= 4 && stripped[..4].eq_ignore_ascii_case("UNC\\") {
                PathBuf::from(format!(r"\\{}", &stripped[4..]))
            } else {
                PathBuf::from(stripped)
            }
        } else if s.len() >= 4 && s[..4].eq_ignore_ascii_case("UNC\\") {
            PathBuf::from(format!(r"\\{}", &s[4..]))
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

    let result: PathBuf = if components.is_empty() {
        PathBuf::from(".")
    } else {
        components.iter().collect()
    };

    // On Windows, try to remap \\server\share\... UNC paths to a local drive
    // letter by querying which drive is mapped to that share.
    #[cfg(windows)]
    {
        use windows_sys::Win32::NetworkManagement::WNet::WNetGetConnectionW;

        let s = result.to_string_lossy();
        if s.starts_with(r"\\") && !s.starts_with(r"\\?\") {
            let mut comps = result.components();
            if let Some(Component::Prefix(p)) = comps.next() {
                let unc_prefix = p.as_os_str().to_string_lossy().into_owned();

                for drive in b'A'..=b'Z' {
                    let local_name: Vec<u16> = format!("{}:", drive as char)
                        .encode_utf16()
                        .chain(std::iter::once(0u16))
                        .collect();
                    let mut buf_len = 512u32;
                    let mut buf = vec![0u16; buf_len as usize];
                    let ret = unsafe {
                        WNetGetConnectionW(local_name.as_ptr(), buf.as_mut_ptr(), &mut buf_len)
                    };
                    if ret == 0 {
                        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                        let mapped_unc = String::from_utf16_lossy(&buf[..end]);
                        if mapped_unc.eq_ignore_ascii_case(&unc_prefix) {
                            let rest: PathBuf = comps
                                .filter(|c| !matches!(c, Component::RootDir))
                                .collect();
                            return PathBuf::from(format!("{}:\\", drive as char)).join(rest);
                        }
                    }
                }
            }
        }
    }

    result
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
