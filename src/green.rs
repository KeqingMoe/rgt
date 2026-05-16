use crate::lang::Language;
use std::{
  fmt::{self, Debug, Write},
  hash::Hash,
  iter,
  ops::{Deref, RangeBounds},
  sync::Arc,
};
use text_size::TextSize;

pub struct GreenNode<L: Language> {
  kind: L::Kind,
  width: TextSize,
  payload: L::Payload,
  children: Option<Box<[Green<L>]>>,
}

impl<L: Language> GreenNode<L> {
  fn dump(
    &self,
    sink: &mut String,
    depth: usize,
    mut start: TextSize,
  ) -> fmt::Result
  where
    L::Kind: Debug,
  {
    for _ in 0..depth {
      sink.push_str("  ");
    }

    writeln!(
      sink,
      "{:?}@{}..{}",
      self.kind,
      u32::from(start),
      u32::from(start + self.width)
    )?;

    if let Some(children) = &self.children {
      for child in children {
        child.0.dump(sink, depth + 1, start)?;
        start += child.width;
      }
    }

    Ok(())
  }

  fn dump_with_payload(
    &self,
    sink: &mut String,
    depth: usize,
    mut start: TextSize,
  ) -> fmt::Result
  where
    L::Kind: Debug,
    L::Payload: Debug,
  {
    for _ in 0..depth {
      sink.push_str("  ");
    }

    writeln!(
      sink,
      "{:?}@{}..{} {:?}",
      self.kind,
      u32::from(start),
      u32::from(start + self.width),
      self.payload
    )?;

    if let Some(children) = &self.children {
      for child in children {
        child.0.dump_with_payload(sink, depth + 1, start)?;
        start += child.width;
      }
    }

    Ok(())
  }

  pub fn kind(&self) -> L::Kind {
    self.kind
  }

  pub fn width(&self) -> TextSize {
    self.width
  }

  pub fn payload(&self) -> &L::Payload {
    &self.payload
  }

  pub fn children(&self) -> Option<&[Green<L>]> {
    Some(self.children.as_ref()?.as_ref())
  }

  pub fn child_count(&self) -> Option<usize> {
    Some(self.children()?.len())
  }

  pub fn is_token(&self) -> bool {
    self.children.is_none()
  }

  pub fn splice_children(
    &self,
    range: impl RangeBounds<usize>,
    replace_with: impl IntoIterator<Item = Green<L>>,
  ) -> Option<Green<L>> {
    let mut children: Vec<_> = self.children.as_ref()?.to_vec();
    children.splice(range, replace_with);

    let mut width = TextSize::new(0);
    for child in &children {
      width += child.width;
    }

    let kind = self.kind;
    let payload =
      L::compose_node(kind, None, children.iter().map(|child| &child.payload));

    let node = GreenNode {
      kind,
      width,
      payload,
      children: Some(children.into_boxed_slice()),
    };

    Some(Green(Arc::new(node)))
  }

  pub fn replace_child(
    &self,
    index: usize,
    new_child: Green<L>,
  ) -> Option<Green<L>> {
    self.splice_children(index..index + 1, iter::once(new_child))
  }

  pub fn insert_child(
    &self,
    index: usize,
    new_child: Green<L>,
  ) -> Option<Green<L>> {
    self.splice_children(index..index, iter::once(new_child))
  }

  pub fn remove_child(&self, index: usize) -> Option<Green<L>> {
    self.splice_children(index..index + 1, iter::empty())
  }
}

pub struct Green<L: Language>(Arc<GreenNode<L>>);

impl<L: Language> Clone for Green<L> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<L: Language> PartialEq for Green<L> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl<L: Language> Eq for Green<L> {}

impl<L: Language> Hash for Green<L> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    Arc::as_ptr(&self.0).hash(state);
  }
}

impl<L: Language> Green<L> {
  fn new(node: GreenNode<L>) -> Self {
    Self(Arc::new(node))
  }

  pub fn token(kind: L::Kind, width: TextSize, payload: L::Payload) -> Self {
    Self::new(GreenNode {
      kind,
      width,
      payload,
      children: None,
    })
  }

  pub fn node(
    kind: L::Kind,
    base: Option<L::Payload>,
    children: impl IntoIterator<Item = Green<L>>,
  ) -> Self {
    let children: Vec<_> = children.into_iter().collect();

    let mut width = TextSize::new(0);
    for child in &children {
      width += child.width;
    }

    let payload =
      L::compose_node(kind, base, children.iter().map(|child| &child.payload));

    Self::new(GreenNode {
      kind,
      width,
      payload,
      children: Some(children.into_boxed_slice()),
    })
  }

  pub fn dump(&self) -> String
  where
    L::Kind: Debug,
  {
    let mut output = String::new();
    self.0.dump(&mut output, 0, TextSize::new(0)).unwrap();
    output
  }

  pub fn dump_with_payload(&self) -> String
  where
    L::Kind: Debug,
    L::Payload: Debug,
  {
    let mut output = String::new();
    self
      .0
      .dump_with_payload(&mut output, 0, TextSize::new(0))
      .unwrap();
    output
  }
}

impl<L: Language> Deref for Green<L> {
  type Target = GreenNode<L>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
