use std::path::PathBuf;
use ddk_core::utils::normalize_path;

// ═══════════════════════════════════════════════════════════════════════════════
//  Stripping the \\?\ prefix
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn strips_extended_length_prefix() {
    let input = r"\\?\C:\Users\foo\project";
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\Users\foo\project"));
}

#[test]
fn leaves_normal_absolute_path_unchanged() {
    let input = r"C:\Users\foo\project";
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\Users\foo\project"));
}

#[test]
fn leaves_relative_path_unchanged() {
    let input = r"src\main.rs";
    assert_eq!(normalize_path(input), PathBuf::from(r"src\main.rs"));
}

#[test]
fn leaves_unc_path_without_prefix_unchanged() {
    let input = r"\\server\share\file.txt";
    assert_eq!(normalize_path(input), PathBuf::from(r"\\server\share\file.txt"));
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Resolving `..` segments
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn resolves_parent_dir_in_absolute_path() {
    let input = r"C:\Users\foo\..\bar";
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\Users\bar"));
}

#[test]
fn resolves_multiple_parent_dirs() {
    let input = r"C:\a\b\c\..\..\d";
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\a\d"));
}

#[test]
fn resolves_parent_dir_at_end() {
    let input = r"C:\Users\foo\bar\..";
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\Users\foo"));
}

#[test]
fn resolves_parent_dir_in_relative_path() {
    let input = r"a\b\..\c";
    assert_eq!(normalize_path(input), PathBuf::from(r"a\c"));
}

#[test]
fn parent_dir_past_root_stays_at_root() {
    // `C:\..` should stay at `C:\`
    let input = r"C:\..";
    let result = normalize_path(input);
    assert_eq!(result, PathBuf::from(r"C:\"));
}

#[test]
fn parent_dir_in_relative_path_is_preserved_when_nothing_to_pop() {
    let input = r"..\foo\bar";
    assert_eq!(normalize_path(input), PathBuf::from(r"..\foo\bar"));
}

#[test]
fn consecutive_parent_dirs_in_relative_path() {
    let input = r"..\..\foo";
    assert_eq!(normalize_path(input), PathBuf::from(r"..\..\foo"));
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Resolving `.` (current dir) segments
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn removes_current_dir_segments() {
    let input = r"C:\Users\.\foo\.\bar";
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\Users\foo\bar"));
}

#[test]
fn removes_current_dir_at_start_of_relative_path() {
    let input = r".\src\main.rs";
    assert_eq!(normalize_path(input), PathBuf::from(r"src\main.rs"));
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Combined: prefix stripping + path resolution
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn strips_prefix_and_resolves_parent_dir() {
    let input = r"\\?\C:\Users\foo\..\bar\project";
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\Users\bar\project"));
}

#[test]
fn strips_prefix_and_resolves_current_dir() {
    let input = r"\\?\C:\Users\.\foo";
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\Users\foo"));
}

#[test]
fn strips_prefix_and_resolves_complex_path() {
    let input = r"\\?\C:\a\b\.\c\..\d\..\..\e";
    // After stripping prefix: C:\a\b\.\c\..\d\..\..\e
    //  components: Prefix(C:), RootDir, a, b, ., c, .., d, .., .., e
    //  skip `.`: a, b, c, .., d, .., .., e
    //  resolve `..`: a, b, [c popped], d, [d popped], [b popped], e → a, e
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\a\e"));
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Edge cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn empty_path_returns_dot() {
    assert_eq!(normalize_path(""), PathBuf::from("."));
}

#[test]
fn single_dot_returns_relative_empty() {
    // `.` alone → all components are CurDir, stack is empty → "."
    assert_eq!(normalize_path("."), PathBuf::from("."));
}

#[test]
fn root_only_stays_root() {
    let input = r"C:\";
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\"));
}

#[test]
fn no_op_on_already_clean_path() {
    let input = r"C:\Users\foo\project\src\main.rs";
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\Users\foo\project\src\main.rs"));
}

#[test]
fn accepts_pathbuf_input() {
    let input = PathBuf::from(r"C:\Users\foo\..\bar");
    assert_eq!(normalize_path(&input), PathBuf::from(r"C:\Users\bar"));
}

#[test]
fn accepts_path_reference() {
    let input = std::path::Path::new(r"C:\a\b\..\c");
    assert_eq!(normalize_path(input), PathBuf::from(r"C:\a\c"));
}
