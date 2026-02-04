use serde::{Serialize, Deserialize};
use std::path::PathBuf;

mod document;
pub use document::*;

#[macro_export]
macro_rules! defer_async {
    ($inner:expr) => {
        defer! {
            tokio::spawn(async move {
                $inner
            });
        }
    };
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