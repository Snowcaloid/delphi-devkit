use crate::projects::*;
use crate::utils::{FilePath, Load};
use anyhow::Result;
use fslock::LockFile;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait::async_trait]
pub trait Stateful {
    fn internal_change_flag() -> &'static AtomicBool;
    fn get_state() -> &'static Arc<RwLock<Self>>;

    fn initialize() -> Result<()>
    where
        Self: FilePath + Load + Serialize + Default,
    {

        let _lock = obtain_lock_blocking::<Self>()?;
        if !<Self as FilePath>::get_file_path().exists() {
            let path = <Self as FilePath>::get_file_path();
            if let Ok(serialized) = ron::ser::to_string_pretty(
                &Self::default(),
                PrettyConfig::default()
                    .struct_names(true)
                    .escape_strings(false)
            ) {
                Self::mark_state_changed(true);
                if let Err(e) = std::fs::write(&path, serialized) {
                    Self::mark_state_changed(false);
                    eprintln!("Failed to save state to {:?}: {}", path, e);
                }
            }
        }
        Ok(())
    }

    fn load() -> Self
    where
        Self: FilePath + Load + Serialize + Default + for<'de> Deserialize<'de>,
    {
        let path = <Self as FilePath>::get_file_path();
        return <Self as Load>::load_from_file(&path);
    }

    async fn save(&self) -> Result<()>
        where Self: FilePath + Stateful + Serialize + Sized
    {
        let path = <Self as FilePath>::get_file_path();
        let _lock = obtain_lock::<Self>().await?;
        if let Ok(serialized) = ron::to_string(&self) {
            Self::mark_state_changed(true);
            if let Err(e) = std::fs::write(&path, serialized) {
                Self::mark_state_changed(false);
                eprintln!("Failed to save state to {:?}: {}", path, e);
            }
        }
        Ok(())
    }

    fn mark_state_changed(changed: bool) {
        Self::internal_change_flag().store(changed, Ordering::SeqCst);
    }
}

pub static PROJECTS_DATA_CHANGED: AtomicBool = AtomicBool::new(false);
pub static COMPILER_CONFIGURATIONS_CHANGED: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    pub static ref PROJECTS_DATA: Arc<RwLock<ProjectsData>> = {
        ProjectsData::initialize().expect("Failed to initialize projects data");
        Arc::new(RwLock::new(ProjectsData::new()))
    };

    pub static ref COMPILER_CONFIGURATIONS: Arc<RwLock<CompilerConfigurations>> = {
        CompilerConfigurations::initialize().expect("Failed to initialize compiler configurations");
        Arc::new(RwLock::new(CompilerConfigurations::new()))
    };
}

fn obtain_lock_blocking<T: FilePath>() -> Result<LockFile> {
    let path = T::get_file_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    let mut tries = 100;

    while tries > 0 {
        tries -= 1;
        match LockFile::open(path) {
            Ok(lock) => {
                return Ok(lock);
            }
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
    anyhow::bail!("Failed to acquire lock for file {:?}", path);
}

async fn obtain_lock<T: FilePath>() -> Result<LockFile> {
    let path = T::get_file_path();
    std::fs::create_dir_all(path.parent().expect("Failed to get parent directory"))?;
    let mut tries = 100;

    while tries > 0 {
        tries -= 1;
        match LockFile::open(path) {
            Ok(lock) => {
                return Ok(lock);
            }
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }
    anyhow::bail!("Failed to acquire lock for file {:?}", path);
}