use crate::green::Green;

pub trait Language: Sized {
  type Kind: Clone + Copy + 'static;
  type Payload: Clone + 'static;

  fn compose_node(kind: Self::Kind, children: &[Green<Self>]) -> Self::Payload;
}
