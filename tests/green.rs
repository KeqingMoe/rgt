mod common;

use common::{Kind, TestLang, sample_tree, token};
use rgt::builder::{BuildError, Builder};
use text_size::TextSize;

#[test]
fn builder_constructs_single_root_tree() {
  let root = sample_tree();

  assert_eq!(root.kind(), Kind::Root);
  assert!(!root.is_token());
  assert_eq!(root.width(), TextSize::new(7));
  assert_eq!(root.payload(), "Root(base foo Pair(+ bar))");
  assert_eq!(root.child_count(), Some(2));
  assert_eq!(root.children().unwrap().len(), 2);
}

#[test]
fn token_reports_token_kind() {
  let token = token(Kind::Ident, 3, "foo");

  assert!(token.is_token());
  assert_eq!(token.child_count(), None);
}

#[test]
fn builder_reports_finish_errors() {
  assert!(matches!(
    Builder::<TestLang>::new().finish(),
    Err(BuildError::ExpectedSingleRoot)
  ));

  let mut builder = Builder::<TestLang>::new();
  builder.start_node(Kind::Root, None);

  assert!(matches!(builder.finish(), Err(BuildError::UnclosedNode)));
}

#[test]
fn builder_reports_finish_node_without_open_node() {
  assert_eq!(
    Builder::<TestLang>::new().finish_node().unwrap_err(),
    BuildError::NoOpenNode
  );
}

#[test]
fn green_equality_uses_pointer_identity() {
  let left = sample_tree();
  let left_clone = left.clone();
  let right = sample_tree();

  assert!(left == left_clone);
  assert!(left != right);
}

#[test]
fn debug_tree_dumps_nodes_and_tokens() {
  let root = sample_tree();

  insta::assert_snapshot!(root.dump());
}

#[test]
fn debug_tree_with_payload_dumps_nodes_and_tokens() {
  let root = sample_tree();

  insta::assert_snapshot!(root.dump_with_payload());
}

#[test]
fn token_child_edits_return_none() {
  let node = token(Kind::Ident, 3, "foo");

  assert!(node.remove_child(0).is_none());
  assert!(node.insert_child(0, token(Kind::Plus, 1, "+")).is_none());
  assert!(
    node
      .replace_child(0, token(Kind::Ident, 3, "bar"))
      .is_none()
  );
}

#[test]
fn replace_child_recomposes_payload_and_width() {
  let root = sample_tree();

  let replaced = root
    .replace_child(0, token(Kind::Ident, 5, "hello"))
    .unwrap();
  assert_eq!(replaced.payload(), "Root(hello Pair(+ bar))");
  assert_eq!(replaced.width(), TextSize::new(9));
}

#[test]
fn insert_child_recomposes_payload_and_width() {
  let root = sample_tree();

  let inserted = root.insert_child(1, token(Kind::Plus, 1, "+")).unwrap();
  assert_eq!(inserted.payload(), "Root(foo + Pair(+ bar))");
  assert_eq!(inserted.width(), TextSize::new(8));
}

#[test]
fn remove_child_recomposes_payload_and_width() {
  let root = sample_tree();

  let removed = root.remove_child(1).unwrap();
  assert_eq!(removed.payload(), "Root(foo)");
  assert_eq!(removed.width(), TextSize::new(3));
}
