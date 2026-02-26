use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

//------------------------------------------------------
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Returns true if active was flipped.
pub fn activate() -> bool {
    !ACTIVE.swap(true, Ordering::SeqCst)
}

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::SeqCst)
}

//------------------------------------------------------
static CANCELLED: AtomicBool = AtomicBool::new(false);

pub fn is_cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

pub fn cancel() {
    CANCELLED.store(true, Ordering::SeqCst);
}

//------------------------------------------------------
static SUCCESS: AtomicBool = AtomicBool::new(false);

pub fn set_success(success: bool) {
    SUCCESS.store(success, Ordering::SeqCst);
}

pub fn is_success() -> bool {
    SUCCESS.load(Ordering::SeqCst)
}

//------------------------------------------------------
static CODE: AtomicI32 = AtomicI32::new(-1);

pub fn set_code(code: i32) {
    CODE.store(code, Ordering::SeqCst);
}

pub fn get_code() -> i32 {
    CODE.load(Ordering::SeqCst)
}

//------------------------------------------------------

pub fn reset() {
    ACTIVE.store(false, Ordering::SeqCst);
    SUCCESS.store(false, Ordering::SeqCst);
    CODE.store(-1, Ordering::SeqCst);
    CANCELLED.store(false, Ordering::SeqCst);
}

//------------------------------------------------------
static DIAGNOSED_FILES: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Record a file that had diagnostics published.
pub fn track_diagnosed_file(file: String) {
    let mut lock = DIAGNOSED_FILES.lock().unwrap();
    lock.get_or_insert_with(HashSet::new).insert(file);
}

/// Drain and return all previously diagnosed file paths.
pub fn take_diagnosed_files() -> HashSet<String> {
    let mut lock = DIAGNOSED_FILES.lock().unwrap();
    lock.take().unwrap_or_default()
}
