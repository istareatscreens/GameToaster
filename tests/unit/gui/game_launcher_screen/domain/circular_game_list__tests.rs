use super::*;

fn ids(values: &[u64]) -> impl Iterator<Item = GameId> + '_ {
    values.iter().copied().map(GameId)
}

#[test]
fn empty_single_and_wrap_are_total() {
    let mut list = CircularGameList::default();
    list.rebuild(ids(&[]));
    assert_eq!(list.select_next(), None);
    list.rebuild(ids(&[7]));
    assert_eq!(list.select_next(), Some(GameId(7)));
    assert_eq!(list.select_previous(), Some(GameId(7)));
    list.rebuild(ids(&[1, 2, 3]));
    assert_eq!(list.select_previous(), Some(GameId(3)));
    assert_eq!(list.select_next(), Some(GameId(1)));
}

#[test]
fn rebuild_preserves_selection_or_prefers_next_survivor() {
    let mut list = CircularGameList::default();
    list.rebuild(ids(&[1, 2, 3, 4]));
    list.select_next();
    assert_eq!(list.selected(), Some(GameId(2)));
    list.rebuild(ids(&[4, 2, 1, 3]));
    assert_eq!(list.selected(), Some(GameId(2)));
    list.rebuild(ids(&[4, 1, 3]));
    assert_eq!(list.selected(), Some(GameId(1)));
    list.rebuild(ids(&[4, 3]));
    assert_eq!(list.selected(), Some(GameId(3)));
    list.rebuild(ids(&[4]));
    assert_eq!(list.selected(), Some(GameId(4)));
}

#[test]
fn visible_window_contains_selection_across_wrap() {
    let mut list = CircularGameList::default();
    list.rebuild(ids(&[1, 2, 3, 4, 5]));
    list.select_previous();
    let visible = list.visible_window(3);
    assert_eq!(visible, vec![GameId(4), GameId(5), GameId(1)]);
    assert!(visible.contains(&list.selected().unwrap()));
}

#[test]
fn every_link_is_reciprocal() {
    let mut list = CircularGameList::default();
    list.rebuild(ids(&[10, 20, 30, 40]));
    for (index, node) in list.nodes.iter().enumerate() {
        assert_eq!(list.nodes[node.next].previous, index);
        assert_eq!(list.nodes[node.previous].next, index);
    }
}
