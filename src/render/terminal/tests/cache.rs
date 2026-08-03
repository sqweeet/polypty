use super::*;

#[test]
fn terminal_cache_preserves_only_the_resize_intersection() {
    let mut cache = TermCache::default();
    cache.ensure(3, 2);
    cache.valid_cells.fill(true);
    let marked = cache.idx(1, 2);
    cache.cells[marked].text = "x".into();

    cache.ensure(5, 3);
    assert_eq!(cache.cells[cache.idx(1, 2)].text, "x");
    assert!(cache.valid_cells[cache.idx(1, 2)]);
    assert!(!cache.valid_cells[cache.idx(0, 3)]);
    assert!(!cache.valid_cells[cache.idx(2, 0)]);

    cache.ensure(2, 1);
    assert_eq!(cache.cells.len(), 2);
    assert!(cache.valid_cells.iter().all(|valid| *valid));
}
