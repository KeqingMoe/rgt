pub trait Language {
  type Kind: Clone + Copy + 'static;
  type Payload: Clone + 'static;

  fn compose_node<'a>(
    kind: Self::Kind,
    base: Option<Self::Payload>,
    children: impl IntoIterator<Item = &'a Self::Payload>,
  ) -> Self::Payload;
}
