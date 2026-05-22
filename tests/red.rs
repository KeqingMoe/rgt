mod common;

use common::{Kind, TestLang, sample_tree, token};
use rgt::{
  green::Green,
  red::{AtOffset, Red, WalkEvent},
};
use text_size::{TextRange, TextSize};

fn root() -> Red<TestLang> {
  Red::new_root(sample_tree())
}

#[test]
fn red_root_exposes_green_position_and_payload() {
  let root = root();

  assert!(root.is_root());
  assert!(!root.is_token());
  assert_eq!(root.kind(), Kind::Root);
  assert_eq!(root.offset(), TextSize::new(0));
  assert_eq!(root.width(), TextSize::new(7));
  assert_eq!(root.range(), TextRange::new(0.into(), 7.into()));
  assert_eq!(root.payload(), "Root(foo Pair(+ bar))");
  assert_eq!(root.child_count(), Some(2));
  assert_eq!(root.root_green().payload(), root.green().payload());
}

#[test]
fn children_have_offsets_parent_and_indices() {
  let root = root();
  let mut children = root.children();
  let first = children.next().unwrap();
  let second = children.next().unwrap();

  assert!(children.next().is_none());
  assert_eq!(first.kind(), Kind::Ident);
  assert_eq!(first.index(), Some(0));
  assert_eq!(first.range(), TextRange::new(0.into(), 3.into()));
  assert_eq!(first.parent().unwrap().kind(), Kind::Root);

  assert_eq!(second.kind(), Kind::Pair);
  assert_eq!(second.index(), Some(1));
  assert_eq!(second.range(), TextRange::new(3.into(), 7.into()));
  assert_eq!(second.parent().unwrap().kind(), Kind::Root);

  let mut token_children = first.children();
  assert!(token_children.next().is_none());
  assert!(token_children.next_back().is_none());
}

#[test]
fn child_at_offset_finds_direct_children() {
  let root = root();

  assert_eq!(
    root
      .child_at_offset(TextSize::new(0))
      .unwrap()
      .left_biased()
      .kind(),
    Kind::Ident
  );
  assert_eq!(
    root
      .child_at_offset(TextSize::new(2))
      .unwrap()
      .left_biased()
      .kind(),
    Kind::Ident
  );

  let AtOffset::Between(left, right) =
    root.child_at_offset(TextSize::new(3)).unwrap()
  else {
    panic!("expected an offset between children");
  };
  assert_eq!(left.kind(), Kind::Ident);
  assert_eq!(right.kind(), Kind::Pair);

  assert_eq!(
    root
      .child_at_offset(TextSize::new(6))
      .unwrap()
      .left_biased()
      .kind(),
    Kind::Pair
  );
  assert_eq!(
    root
      .child_at_offset(TextSize::new(7))
      .unwrap()
      .right_biased()
      .kind(),
    Kind::Pair
  );
}

#[test]
fn at_offset_helpers_cover_single_and_between() {
  let root = root();

  let single = root.child_at_offset(TextSize::new(1)).unwrap();
  assert!(!single.is_between());
  assert_eq!(single.as_ref().left_biased().kind(), Kind::Ident);
  assert_eq!(
    single
      .map(|node| node.kind())
      .into_iter()
      .collect::<Vec<_>>(),
    vec![Kind::Ident]
  );

  let between = root.child_at_offset(TextSize::new(3)).unwrap();
  assert!(between.is_between());
  assert_eq!(between.as_ref().left_biased().kind(), Kind::Ident);
  assert_eq!(between.as_ref().right_biased().kind(), Kind::Pair);
  assert_eq!(
    between
      .map(|node| node.kind())
      .into_iter()
      .collect::<Vec<_>>(),
    vec![Kind::Ident, Kind::Pair]
  );
}

#[test]
fn token_at_offset_descends_to_leaf() {
  let root = root();

  let plus = root
    .token_at_offset(TextSize::new(3))
    .unwrap()
    .right_biased();
  assert_eq!(plus.kind(), Kind::Plus);
  assert!(plus.is_token());
  assert_eq!(plus.child_count(), None);
  assert_eq!(plus.range(), TextRange::new(3.into(), 4.into()));

  let bar = root
    .token_at_offset(TextSize::new(6))
    .unwrap()
    .left_biased();
  assert_eq!(bar.kind(), Kind::Ident);
  assert_eq!(bar.range(), TextRange::new(4.into(), 7.into()));
}

#[test]
fn token_navigation_walks_leaf_order() {
  let root = root();
  let foo = root
    .token_at_offset(TextSize::new(1))
    .unwrap()
    .left_biased();
  let plus = root
    .token_at_offset(TextSize::new(3))
    .unwrap()
    .right_biased();
  let bar = root
    .token_at_offset(TextSize::new(6))
    .unwrap()
    .left_biased();

  assert!(foo.prev_token().is_none());
  assert_eq!(foo.next_token().unwrap().kind(), Kind::Plus);

  assert_eq!(plus.prev_token().unwrap().kind(), Kind::Ident);
  assert_eq!(plus.next_token().unwrap().kind(), Kind::Ident);

  assert_eq!(bar.prev_token().unwrap().kind(), Kind::Plus);
  assert!(bar.next_token().is_none());
}

#[test]
fn preorder_emits_enter_and_leave_events() {
  let root = root();
  let events: Vec<_> = root
    .preorder()
    .map(|event| match event {
      WalkEvent::Enter(node) => ("enter", node.kind()),
      WalkEvent::Leave(node) => ("leave", node.kind()),
    })
    .collect();

  assert_eq!(
    events,
    vec![
      ("enter", Kind::Root),
      ("enter", Kind::Ident),
      ("leave", Kind::Ident),
      ("enter", Kind::Pair),
      ("enter", Kind::Plus),
      ("leave", Kind::Plus),
      ("enter", Kind::Ident),
      ("leave", Kind::Ident),
      ("leave", Kind::Pair),
      ("leave", Kind::Root),
    ]
  );
}

#[test]
fn descendants_are_preorder_and_include_self() {
  let root = root();
  let kinds: Vec<_> = root.descendants().map(|node| node.kind()).collect();

  assert_eq!(
    kinds,
    vec![Kind::Root, Kind::Ident, Kind::Pair, Kind::Plus, Kind::Ident]
  );
}

#[test]
fn tokens_filters_descendants_to_leaves() {
  let root = root();
  let tokens: Vec<_> = root
    .tokens()
    .map(|node| (node.kind(), node.range()))
    .collect();

  assert_eq!(
    tokens,
    vec![
      (Kind::Ident, TextRange::new(0.into(), 3.into())),
      (Kind::Plus, TextRange::new(3.into(), 4.into())),
      (Kind::Ident, TextRange::new(4.into(), 7.into())),
    ]
  );
}

#[test]
fn covering_node_returns_smallest_covering_node() {
  let root = root();

  let foo = root
    .covering_node(TextRange::new(1.into(), 2.into()))
    .unwrap();
  assert_eq!(foo.kind(), Kind::Ident);

  let pair = root
    .covering_node(TextRange::new(3.into(), 7.into()))
    .unwrap();
  assert_eq!(pair.kind(), Kind::Pair);

  let root_cover = root
    .covering_node(TextRange::new(2.into(), 5.into()))
    .unwrap();
  assert_eq!(root_cover.kind(), Kind::Root);
}

#[test]
fn ancestors_walk_to_root() {
  let root = root();
  let bar = root
    .token_at_offset(TextSize::new(6))
    .unwrap()
    .left_biased();
  let kinds: Vec<_> = bar.ancestors().map(|node| node.kind()).collect();

  assert_eq!(kinds, vec![Kind::Ident, Kind::Pair, Kind::Root]);
  assert_eq!(bar.root().kind(), Kind::Root);
}

#[test]
fn sibling_navigation_uses_parent_indices() {
  let root = root();
  let pair = root.child(1).unwrap();
  let foo = pair.prev_sibling().unwrap();

  assert_eq!(foo.kind(), Kind::Ident);
  assert!(foo.prev_sibling().is_none());
  assert_eq!(foo.next_sibling().unwrap().kind(), Kind::Pair);

  let sibling_kinds: Vec<_> =
    pair.siblings().unwrap().map(|node| node.kind()).collect();
  assert_eq!(sibling_kinds, vec![Kind::Ident, Kind::Pair]);
}

#[test]
fn red_replace_with_rebuilds_to_root() {
  let root = root();
  let pair = root.child(1).unwrap();
  let new_pair = Green::node(Kind::Pair, [token(Kind::Ident, 5, "hello")]);

  let new_root = pair.replace_with(new_pair);
  assert_eq!(new_root.payload(), "Root(foo Pair(hello))");
  assert_eq!(new_root.width(), TextSize::new(8));
}

#[test]
fn red_child_edits_rebuild_to_root() {
  let root = root();
  let pair = root.child(1).unwrap();

  let replaced = pair
    .replace_child(1, token(Kind::Ident, 5, "hello"))
    .unwrap();
  assert_eq!(replaced.payload(), "Root(foo Pair(+ hello))");
  assert_eq!(replaced.width(), TextSize::new(9));

  let removed = pair.remove_self().unwrap();
  assert_eq!(removed.payload(), "Root(foo)");
  assert_eq!(removed.width(), TextSize::new(3));
}

#[test]
fn covering_empty_range_inside_token_returns_token() {
  let root = root();

  let node = root
    .covering_node(TextRange::new(5.into(), 5.into()))
    .unwrap();

  assert_eq!(node.kind(), Kind::Ident);
  assert_eq!(node.range(), TextRange::new(4.into(), 7.into()));
}

#[test]
fn covering_empty_range_between_root_children_returns_root() {
  let root = root();

  let node = root
    .covering_node(TextRange::new(3.into(), 3.into()))
    .unwrap();

  assert_eq!(node.kind(), Kind::Root);
  assert_eq!(node.range(), TextRange::new(0.into(), 7.into()));
}

#[test]
fn covering_empty_range_between_nested_children_returns_parent() {
  let root = root();

  let node = root
    .covering_node(TextRange::new(4.into(), 4.into()))
    .unwrap();

  assert_eq!(node.kind(), Kind::Pair);
  assert_eq!(node.range(), TextRange::new(3.into(), 7.into()));
}

#[test]
fn covering_empty_range_at_tree_start_returns_first_token() {
  let root = root();

  let node = root
    .covering_node(TextRange::new(0.into(), 0.into()))
    .unwrap();

  assert_eq!(node.kind(), Kind::Ident);
  assert_eq!(node.range(), TextRange::new(0.into(), 3.into()));
}

#[test]
fn covering_empty_range_at_tree_end_returns_last_token() {
  let root = root();

  let node = root
    .covering_node(TextRange::new(7.into(), 7.into()))
    .unwrap();

  assert_eq!(node.kind(), Kind::Ident);
  assert_eq!(node.range(), TextRange::new(4.into(), 7.into()));
}

#[test]
fn covering_empty_range_outside_node_returns_none() {
  let root = root();

  assert!(
    root
      .covering_node(TextRange::new(8.into(), 8.into()))
      .is_none()
  );
}
