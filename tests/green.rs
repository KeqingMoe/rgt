use rgt::{
  builder::{BuildError, Builder},
  green::Green,
  lang::Language,
};
use text_size::TextSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
  Root,
  Pair,
  Ident,
  Plus,
}

struct TestLang;

impl Language for TestLang {
  type Kind = Kind;
  type Payload = String;

  fn compose_node<'a>(
    kind: Self::Kind,
    base: Option<Self::Payload>,
    children: impl IntoIterator<Item = &'a Self::Payload>,
  ) -> Self::Payload {
    let mut parts = Vec::new();
    if let Some(base) = base {
      parts.push(base);
    }
    parts.extend(children.into_iter().cloned());

    format!("{kind:?}({})", parts.join(" "))
  }
}

fn token(kind: Kind, width: u32, payload: &str) -> Green<TestLang> {
  Green::token(kind, TextSize::new(width), payload.to_string())
}

fn sample_tree() -> Green<TestLang> {
  let mut builder = Builder::<TestLang>::new();
  builder.start_node(Kind::Root, Some("base".to_string()));
  builder.token(Kind::Ident, TextSize::new(3), "foo".to_string());
  builder.start_node(Kind::Pair, None);
  builder.token(Kind::Plus, TextSize::new(1), "+".to_string());
  builder.token(Kind::Ident, TextSize::new(3), "bar".to_string());
  builder.finish_node().unwrap();
  builder.finish_node().unwrap();
  builder.finish().unwrap()
}

#[test]
fn builder_constructs_single_root_tree() {
  let root = sample_tree();

  assert_eq!(root.kind(), Kind::Root);
  assert_eq!(root.width(), TextSize::new(7));
  assert_eq!(root.payload(), "Root(base foo Pair(+ bar))");
  assert_eq!(root.children().unwrap().len(), 2);
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
