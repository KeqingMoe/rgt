use crate::lang::Language;
use std::{
  fmt::{self, Debug, Write},
  hash::Hash,
  iter,
  ops::{Deref, RangeBounds},
  sync::Arc,
};
use text_size::TextSize;

/// Immutable green tree data.
///
/// A green node stores kind, width, payload, and optional children. It has no
/// parent pointer and no absolute offset; use [`crate::red::Red`] for positioned
/// navigation.
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

  /// Returns this element's syntax kind.
  pub fn kind(&self) -> L::Kind {
    self.kind
  }

  /// Returns this element's text width.
  ///
  /// The tree does not store source text. Widths are enough to recover offsets
  /// in a red tree.
  pub fn width(&self) -> TextSize {
    self.width
  }

  /// Returns this element's user-defined payload.
  pub fn payload(&self) -> &L::Payload {
    &self.payload
  }

  /// Returns this node's green children.
  ///
  /// `None` means this element is a token. `Some([])` is a non-token node with
  /// no children.
  pub fn children(&self) -> Option<&[Green<L>]> {
    Some(self.children.as_ref()?.as_ref())
  }

  /// Returns the number of green children, or `None` for tokens.
  pub fn child_count(&self) -> Option<usize> {
    Some(self.children()?.len())
  }

  /// Returns whether this element is a token.
  pub fn is_token(&self) -> bool {
    self.children.is_none()
  }

  /// Replaces a range of children and rebuilds this green node.
  ///
  /// Returns `None` for tokens. For nodes, the returned green tree has a
  /// recomputed width and payload.
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
    let payload = L::compose_node(kind, &children);

    let node = GreenNode {
      kind,
      width,
      payload,
      children: Some(children.into_boxed_slice()),
    };

    Some(Green(Arc::new(node)))
  }

  /// Replaces one child and rebuilds this green node.
  ///
  /// Returns `None` for tokens or an out-of-bounds index.
  pub fn replace_child(
    &self,
    index: usize,
    new_child: Green<L>,
  ) -> Option<Green<L>> {
    self.splice_children(index..index + 1, iter::once(new_child))
  }

  /// Inserts one child and rebuilds this green node.
  ///
  /// Returns `None` for tokens or an out-of-bounds index.
  pub fn insert_child(
    &self,
    index: usize,
    new_child: Green<L>,
  ) -> Option<Green<L>> {
    self.splice_children(index..index, iter::once(new_child))
  }

  /// Removes one child and rebuilds this green node.
  ///
  /// Returns `None` for tokens or an out-of-bounds index.
  pub fn remove_child(&self, index: usize) -> Option<Green<L>> {
    self.splice_children(index..index + 1, iter::empty())
  }
}

/// Shared immutable green tree element.
///
/// `Green` is an `Arc` handle around [`GreenNode`]. Equality and hashing use
/// pointer identity, which makes immutable sharing visible without requiring an
/// interner.
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

  /// Creates a token green element.
  ///
  /// The token payload is supplied directly by the caller.
  pub fn token(kind: L::Kind, width: TextSize, payload: L::Payload) -> Self {
    Self::new(GreenNode {
      kind,
      width,
      payload,
      children: None,
    })
  }

  /// Creates a non-token green node from children.
  ///
  /// Width is the sum of child widths. Payload is produced by
  /// [`Language::compose_node`].
  pub fn node(
    kind: L::Kind,
    children: impl IntoIterator<Item = Green<L>>,
  ) -> Self {
    let children: Vec<_> = children.into_iter().collect();

    let mut width = TextSize::new(0);
    for child in &children {
      width += child.width;
    }

    let payload = L::compose_node(kind, &children);

    Self::new(GreenNode {
      kind,
      width,
      payload,
      children: Some(children.into_boxed_slice()),
    })
  }

  /// Dumps the green tree with kinds and ranges for debugging.
  pub fn dump(&self) -> String
  where
    L::Kind: Debug,
  {
    let mut output = String::new();
    self.0.dump(&mut output, 0, TextSize::new(0)).unwrap();
    output
  }

  /// Dumps the green tree with kinds, ranges, and payloads for debugging.
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

impl<L: Language> Debug for Green<L> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Green@{}", Arc::as_ptr(&self.0).addr())
  }
}
