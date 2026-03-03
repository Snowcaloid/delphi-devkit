use ddk_core::projects::*;
use ddk_core::lexorank::LexoRank;

// ═══════════════════════════════════════════════════════════════════════════════
//  Helper: build a Workspace with links for testing ProjectLinkContainer
// ═══════════════════════════════════════════════════════════════════════════════

fn make_link(id: usize, project_id: usize, rank: &str) -> ProjectLink {
    ProjectLink {
        id,
        project_id,
        sort_rank: LexoRank::from_string(rank).unwrap_or_default(),
    }
}

fn make_workspace_with_links(links: Vec<ProjectLink>) -> Workspace {
    Workspace {
        id: 1,
        name: "TestWS".to_string(),
        compiler_id: "12.0".to_string(),
        project_links: links,
        sort_rank: LexoRank::default(),
        ..Default::default()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  new_project_link
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn new_project_link_on_empty_container() {
    let mut ws = make_workspace_with_links(vec![]);
    ws.new_project_link(100, 50);
    assert_eq!(ws.project_links.len(), 1);
    assert_eq!(ws.project_links[0].id, 100);
    assert_eq!(ws.project_links[0].project_id, 50);
}

#[test]
fn new_project_link_appends_with_higher_rank() {
    let mut ws = make_workspace_with_links(vec![
        make_link(1, 10, "1|a"),
    ]);
    ws.new_project_link(2, 20);
    assert_eq!(ws.project_links.len(), 2);
    // New link should have a rank greater than existing
    assert!(ws.project_links[1].sort_rank > ws.project_links[0].sort_rank);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  index_of
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn index_of_found() {
    let ws = make_workspace_with_links(vec![
        make_link(10, 1, "1|a"),
        make_link(20, 2, "1|h"),
    ]);
    assert_eq!(ws.index_of(10), Some(0));
    assert_eq!(ws.index_of(20), Some(1));
}

#[test]
fn index_of_not_found() {
    let ws = make_workspace_with_links(vec![]);
    assert_eq!(ws.index_of(999), None);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  export_project_link
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn export_removes_link() {
    let mut ws = make_workspace_with_links(vec![
        make_link(10, 1, "1|a"),
        make_link(20, 2, "1|h"),
    ]);
    let exported = ws.export_project_link(10).unwrap();
    assert_eq!(exported.id, 10);
    assert_eq!(ws.project_links.len(), 1);
    assert_eq!(ws.project_links[0].id, 20);
}

#[test]
fn export_nonexistent_link_fails() {
    let mut ws = make_workspace_with_links(vec![
        make_link(10, 1, "1|a"),
    ]);
    assert!(ws.export_project_link(999).is_err());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  import_project_link
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn import_at_end() {
    let mut ws = make_workspace_with_links(vec![
        make_link(10, 1, "1|a"),
    ]);
    let new_link = make_link(20, 2, "1|z");
    ws.import_project_link(new_link, None).unwrap();
    assert_eq!(ws.project_links.len(), 2);
    assert_eq!(ws.project_links[1].id, 20);
}

#[test]
fn import_at_position() {
    let mut ws = make_workspace_with_links(vec![
        make_link(10, 1, "1|a"),
        make_link(30, 3, "1|z"),
    ]);
    let new_link = make_link(20, 2, "1|h");
    // Import before link 30 (at index of link 30)
    ws.import_project_link(new_link, Some(30)).unwrap();
    assert_eq!(ws.project_links.len(), 3);
    // The imported link should be at the position of the drop target
    assert_eq!(ws.project_links[1].id, 20);
}

#[test]
fn import_reorders_ranks() {
    let mut ws = make_workspace_with_links(vec![
        make_link(10, 1, "1|a"),
        make_link(20, 2, "1|h"),
    ]);
    let new_link = make_link(30, 3, "1|z");
    ws.import_project_link(new_link, None).unwrap();
    // After import, all ranks should be strictly ordered
    for w in ws.project_links.windows(2) {
        assert!(w[0].sort_rank < w[1].sort_rank);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  move_project_link (within same container)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn move_link_to_different_position() {
    let mut ws = make_workspace_with_links(vec![
        make_link(10, 1, "1|a"),
        make_link(20, 2, "1|h"),
        make_link(30, 3, "1|z"),
    ]);
    // Move link 10 to the position of link 30
    ws.move_project_link(10, Some(30)).unwrap();
    // Link 10 should now be at index of link 30 (after removal and insertion)
    // Verify all ranks are ordered after the move
    for w in ws.project_links.windows(2) {
        assert!(w[0].sort_rank < w[1].sort_rank);
    }
}

#[test]
fn move_link_to_end() {
    let mut ws = make_workspace_with_links(vec![
        make_link(10, 1, "1|a"),
        make_link(20, 2, "1|h"),
    ]);
    ws.move_project_link(10, None).unwrap();
    assert_eq!(ws.project_links.last().unwrap().id, 10);
}

#[test]
fn move_nonexistent_link_fails() {
    let mut ws = make_workspace_with_links(vec![
        make_link(10, 1, "1|a"),
    ]);
    assert!(ws.move_project_link(999, None).is_err());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  reorder_links
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn reorder_links_produces_strictly_ordered_ranks() {
    let mut ws = make_workspace_with_links(vec![
        make_link(10, 1, "1|z"), // intentionally out of order rank
        make_link(20, 2, "1|a"),
        make_link(30, 3, "1|a"),
    ]);
    ws.reorder_links();
    for w in ws.project_links.windows(2) {
        assert!(w[0].sort_rank < w[1].sort_rank);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GroupProject also implements ProjectLinkContainer
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn group_project_new_project_link() {
    let mut gp = GroupProject {
        name: "Test".to_string(),
        path: "test.groupproj".to_string(),
        project_links: vec![],
        ..Default::default()
    };
    gp.new_project_link(1, 100);
    assert_eq!(gp.project_links.len(), 1);
    assert_eq!(gp.project_links[0].project_id, 100);
}
