use rgt::{builder::Builder, green::Green, lang::Language};
use text_size::TextSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
  Root,
  Pair,
  Ident,
  Plus,
}

pub struct TestLang;

impl Language for TestLang {
  type Kind = Kind;
  type Payload = String;

  fn compose_node(kind: Self::Kind, children: &[Green<Self>]) -> Self::Payload {
    let parts: Vec<_> = children
      .iter()
      .map(|child| child.payload().clone())
      .collect();

    format!("{kind:?}({})", parts.join(" "))
  }
}

pub fn token(kind: Kind, width: u32, payload: &str) -> Green<TestLang> {
  Green::token(kind, TextSize::new(width), payload.to_string())
}

pub fn sample_tree() -> Green<TestLang> {
  let mut builder = Builder::<TestLang>::new();
  builder.start_node(Kind::Root);
  builder.token(Kind::Ident, TextSize::new(3), "foo".to_string());
  builder.start_node(Kind::Pair);
  builder.token(Kind::Plus, TextSize::new(1), "+".to_string());
  builder.token(Kind::Ident, TextSize::new(3), "bar".to_string());
  builder.finish_node().unwrap();
  builder.finish_node().unwrap();
  builder.finish().unwrap()
}
