use crate::{green::Green, lang::Language};
use std::{
  fmt::{self, Debug},
  hash::Hash,
  iter,
  ops::{Deref, RangeBounds},
  sync::Arc,
};
use text_size::{TextRange, TextSize};

struct Parent<L: Language> {
  node: Red<L>,
  index: usize,
}

/// Positioned red node data.
///
/// A red node wraps a green element with parent, child index, and absolute text
/// offset information. Most users work through the [`Red`] handle.
pub struct RedNode<L: Language> {
  green: Green<L>,
  parent: Option<Parent<L>>,
  offset: TextSize,
}

/// Positioned view over an immutable green tree.
///
/// Red nodes are created lazily from green children. They provide navigation
/// APIs over parent/child/sibling relationships, offsets, token order, and
/// subtree replacement.
pub struct Red<L: Language>(Arc<RedNode<L>>);

impl<L: Language> Clone for Red<L> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<L: Language> PartialEq for Red<L> {
  fn eq(&self, other: &Self) -> bool {
    self.green == other.green
  }
}

impl<L: Language> Eq for Red<L> {}

impl<L: Language> Hash for Red<L> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.green.hash(state);
  }
}

/// Result of an offset query.
///
/// A text offset can point inside a single element, or sit exactly between two
/// adjacent elements. `Between(left, right)` preserves both choices so callers
/// can choose a bias explicitly.
pub enum AtOffset<T> {
  /// The offset resolves to one element.
  Single(T),
  /// The offset is on a boundary between the left and right elements.
  Between(T, T),
}

impl<T> AtOffset<T> {
  /// Borrows the contained value or values.
  pub fn as_ref(&self) -> AtOffset<&T> {
    match self {
      Self::Single(node) => AtOffset::Single(node),
      Self::Between(left, right) => AtOffset::Between(left, right),
    }
  }

  /// Returns whether this value is [`AtOffset::Between`].
  pub fn is_between(&self) -> bool {
    matches!(self, Self::Between(_, _))
  }

  /// Maps the contained value or values.
  pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> AtOffset<U> {
    match self {
      Self::Single(node) => AtOffset::Single(f(node)),
      Self::Between(left, right) => AtOffset::Between(f(left), f(right)),
    }
  }

  /// Returns the single value, or the left value for a boundary.
  pub fn left_biased(self) -> T {
    match self {
      Self::Single(node) | Self::Between(node, _) => node,
    }
  }

  /// Returns the single value, or the right value for a boundary.
  pub fn right_biased(self) -> T {
    match self {
      Self::Single(node) | Self::Between(_, node) => node,
    }
  }
}

/// Iterator produced by [`AtOffset`]'s `IntoIterator` implementation.
pub struct AtOffsetIter<T> {
  first: Option<T>,
  second: Option<T>,
}

impl<T> Iterator for AtOffsetIter<T> {
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    self.first.take().or_else(|| self.second.take())
  }
}

impl<T> IntoIterator for AtOffset<T> {
  type Item = T;
  type IntoIter = AtOffsetIter<T>;

  fn into_iter(self) -> Self::IntoIter {
    match self {
      Self::Single(node) => AtOffsetIter {
        first: Some(node),
        second: None,
      },
      Self::Between(left, right) => AtOffsetIter {
        first: Some(left),
        second: Some(right),
      },
    }
  }
}

/// Preorder walk event.
pub enum WalkEvent<T> {
  /// Emitted before visiting children.
  Enter(T),
  /// Emitted after visiting children.
  Leave(T),
}

/// Lazy iterator over child metadata before red nodes are materialized.
pub struct LazyChildren<L: Language> {
  parent: Red<L>,
  front_index: usize,
  front_offset: TextSize,
  back_index: usize,
  back_offset: TextSize,
}

/// Lazy child entry with green data, child index, and absolute offset.
pub struct LazyChild<L: Language> {
  parent: Red<L>,
  index: usize,
  green: Green<L>,
  offset: TextSize,
}

impl<L: Language> LazyChild<L> {
  /// Returns this child's absolute text range.
  pub fn range(&self) -> TextRange {
    TextRange::at(self.offset, self.green.width())
  }

  /// Materializes this lazy child as a red node.
  pub fn into_red(self) -> Red<L> {
    Red::with_parent(self.green, self.parent, self.index, self.offset)
  }
}

impl<L: Language> Iterator for LazyChildren<L> {
  type Item = LazyChild<L>;

  fn next(&mut self) -> Option<Self::Item> {
    let parent = self.parent.clone();
    if self.front_index >= self.back_index {
      return None;
    }

    // The iterator carries offsets from both ends so `next` and `next_back`
    // can materialize children without scanning from the start each time.
    let green = parent.green.children()?.get(self.front_index)?.clone();
    let offset = self.front_offset;
    let index = self.front_index;

    self.front_index += 1;
    self.front_offset += green.width();

    Some(LazyChild {
      parent,
      index,
      green,
      offset,
    })
  }
}

impl<L: Language> DoubleEndedIterator for LazyChildren<L> {
  fn next_back(&mut self) -> Option<Self::Item> {
    let parent = self.parent.clone();
    if self.front_index >= self.back_index {
      return None;
    }

    self.back_index -= 1;
    // Walking from the back subtracts the selected child's width first; the
    // resulting offset is the child's start.
    let green = parent.green.children()?.get(self.back_index)?.clone();
    self.back_offset -= green.width();

    Some(LazyChild {
      parent,
      index: self.back_index,
      green,
      offset: self.back_offset,
    })
  }
}

/// Iterator over materialized red children.
///
/// Tokens have no children, so their `Children` iterator is empty.
pub struct Children<L: Language>(LazyChildren<L>);

impl<L: Language> Iterator for Children<L> {
  type Item = Red<L>;

  fn next(&mut self) -> Option<Self::Item> {
    self.0.next().map(LazyChild::into_red)
  }
}

impl<L: Language> DoubleEndedIterator for Children<L> {
  fn next_back(&mut self) -> Option<Self::Item> {
    self.0.next_back().map(LazyChild::into_red)
  }
}

/// Iterator over [`WalkEvent`]s in preorder.
pub struct Preorder<L: Language> {
  stack: Vec<WalkEvent<Red<L>>>,
}

impl<L: Language> Iterator for Preorder<L> {
  type Item = WalkEvent<Red<L>>;

  fn next(&mut self) -> Option<Self::Item> {
    let event = self.stack.pop()?;

    if let WalkEvent::Enter(node) = &event {
      self.stack.push(WalkEvent::Leave(node.clone()));

      self
        .stack
        .extend(node.children().rev().map(WalkEvent::Enter));
    }

    Some(event)
  }
}

/// Iterator over red descendants in preorder, including the starting node.
pub struct Descendants<L: Language> {
  preorder: Preorder<L>,
}

impl<L: Language> Iterator for Descendants<L> {
  type Item = Red<L>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match self.preorder.next()? {
        WalkEvent::Enter(node) => return Some(node),
        WalkEvent::Leave(_) => {}
      }
    }
  }
}

impl<L: Language> Red<L> {
  /// Creates a red root view at offset 0.
  pub fn new_root(green: Green<L>) -> Self {
    Self(Arc::new(RedNode {
      green,
      parent: None,
      offset: TextSize::new(0),
    }))
  }

  fn with_parent(
    green: Green<L>,
    parent: Self,
    index: usize,
    offset: TextSize,
  ) -> Self {
    Self(Arc::new(RedNode {
      green,
      parent: Some(Parent {
        node: parent,
        index,
      }),
      offset,
    }))
  }

  /// Returns the underlying green element.
  pub fn green(&self) -> &Green<L> {
    &self.green
  }

  /// Returns this element's syntax kind.
  pub fn kind(&self) -> L::Kind {
    self.green.kind()
  }

  /// Returns this red node's absolute start offset.
  pub fn offset(&self) -> TextSize {
    self.offset
  }

  /// Returns this element's text width.
  pub fn width(&self) -> TextSize {
    self.green.width()
  }

  /// Returns this red node's absolute text range.
  pub fn range(&self) -> TextRange {
    TextRange::at(self.offset, self.width())
  }

  /// Returns this element's payload.
  pub fn payload(&self) -> &L::Payload {
    self.green.payload()
  }

  /// Returns whether this element is a token.
  pub fn is_token(&self) -> bool {
    self.green.is_token()
  }

  fn lazy_children(&self) -> LazyChildren<L> {
    let child_count = self.green.child_count().unwrap_or(0);
    let range = self.range();

    LazyChildren {
      parent: self.clone(),
      front_index: 0,
      front_offset: range.start(),
      back_index: child_count,
      back_offset: range.end(),
    }
  }

  /// Returns an iterator over this node's red children.
  ///
  /// Tokens return an empty iterator instead of `None`.
  pub fn children(&self) -> Children<L> {
    Children(self.lazy_children())
  }

  /// Walks this subtree in preorder, emitting enter and leave events.
  pub fn preorder(&self) -> Preorder<L> {
    Preorder {
      stack: vec![WalkEvent::Enter(self.clone())],
    }
  }

  /// Returns descendants in preorder, including `self`.
  pub fn descendants(&self) -> Descendants<L> {
    Descendants {
      preorder: self.preorder(),
    }
  }

  /// Returns all tokens under this node in source order.
  pub fn tokens(&self) -> impl Iterator<Item = Self> {
    self.descendants().filter(|node| node.is_token())
  }

  /// Returns the number of children, or `None` for tokens.
  pub fn child_count(&self) -> Option<usize> {
    self.green.child_count()
  }

  /// Returns the child at `index`.
  pub fn child(&self, index: usize) -> Option<Self> {
    self.lazy_children().nth(index).map(LazyChild::into_red)
  }

  /// Returns the first child.
  pub fn first_child(&self) -> Option<Self> {
    self.children().next()
  }

  /// Returns the last child.
  pub fn last_child(&self) -> Option<Self> {
    self.children().next_back()
  }

  /// Finds the direct child at an absolute offset.
  ///
  /// If the offset is exactly between two children, returns
  /// [`AtOffset::Between`]. The offset must be inside this node's inclusive
  /// range.
  pub fn child_at_offset(&self, offset: TextSize) -> Option<AtOffset<Self>> {
    if !self.range().contains_inclusive(offset) {
      return None;
    }

    let mut left = None;
    for child in self.lazy_children() {
      let range = child.range();

      if range.start() == offset {
        let right = child.into_red();
        return Some(match left {
          Some(left) => AtOffset::Between(left, right),
          None => AtOffset::Single(right),
        });
      }

      if range.contains(offset) {
        return Some(AtOffset::Single(child.into_red()));
      }

      if range.end() == offset {
        left = Some(child.into_red());
      }
    }

    left.map(AtOffset::Single)
  }

  /// Returns the first token in this subtree.
  ///
  /// If called on a token, returns `self`.
  pub fn first_token(&self) -> Self {
    let mut node = self.clone();
    while let Some(child) = node.first_child() {
      node = child;
    }
    node
  }

  /// Returns the last token in this subtree.
  ///
  /// If called on a token, returns `self`.
  pub fn last_token(&self) -> Self {
    let mut node = self.clone();
    while let Some(child) = node.last_child() {
      node = child;
    }
    node
  }

  /// Finds the token at an absolute offset.
  ///
  /// Boundary offsets return [`AtOffset::Between`] with the token on each side.
  pub fn token_at_offset(&self, offset: TextSize) -> Option<AtOffset<Self>> {
    if !self.range().contains_inclusive(offset) {
      return None;
    }

    let Some(child) = self.child_at_offset(offset) else {
      return Some(AtOffset::Single(self.clone()));
    };

    Some(match child {
      AtOffset::Single(child) => child.token_at_offset(offset)?,
      AtOffset::Between(left, right) => {
        AtOffset::Between(left.last_token(), right.first_token())
      }
    })
  }

  /// Iterates from this node to the root, including both endpoints.
  pub fn ancestors(&self) -> impl Iterator<Item = Self> {
    iter::successors(Some(self.clone()), |node| node.parent())
  }

  /// Returns this red tree's root node.
  pub fn root(&self) -> Self {
    self.ancestors().last().unwrap()
  }

  /// Returns this red tree's root green element.
  pub fn root_green(&self) -> Green<L> {
    self.root().green.clone()
  }

  /// Returns the smallest node that fully covers `range`.
  ///
  /// Returns `None` when `range` is outside this node.
  pub fn covering_node(&self, range: TextRange) -> Option<Self> {
    if !self.range().contains_range(range) {
      return None;
    }

    if range.is_empty() {
      return Some(self.covering_empty_range(range.start()));
    }

    Some(self.covering_nonempty_range(range))
  }

  fn covering_nonempty_range(&self, range: TextRange) -> Self {
    for child in self.lazy_children() {
      let r = child.range();
      if let Some(inter) = r.intersect(range)
        && !inter.is_empty()
      {
        if inter == range {
          return child.into_red().covering_nonempty_range(range);
        }
        break;
      }
    }

    self.clone()
  }

  fn covering_empty_range(&self, offset: TextSize) -> Self {
    let mut left = None;

    for child in self.lazy_children() {
      let r = child.range();

      if r.is_empty() {
        continue;
      }

      if r.start() == offset {
        if left.is_some() {
          return self.clone();
        }

        return child.into_red().covering_empty_range(offset);
      }

      if r.contains(offset) {
        return child.into_red().covering_empty_range(offset);
      }

      if r.end() == offset {
        left = Some(child);
      }
    }

    match left {
      Some(child) => child.into_red().covering_empty_range(offset),
      None => self.clone(),
    }
  }

  /// Returns this node's parent.
  pub fn parent(&self) -> Option<Self> {
    self.parent.as_ref().map(|p| p.node.clone())
  }

  /// Returns this node's index in its parent.
  pub fn index(&self) -> Option<usize> {
    self.parent.as_ref().map(|p| p.index)
  }

  /// Returns the previous sibling.
  pub fn prev_sibling(&self) -> Option<Self> {
    let parent = self.parent()?;
    let index = self.index()?;
    index.checked_sub(1).and_then(|index| parent.child(index))
  }

  /// Returns the next sibling.
  pub fn next_sibling(&self) -> Option<Self> {
    let parent = self.parent()?;
    let index = self.index()?;
    parent.child(index + 1)
  }

  /// Returns the previous token in source order.
  pub fn prev_token(&self) -> Option<Self> {
    if let Some(sibling) = self.prev_sibling() {
      return Some(sibling.last_token());
    }

    self.parent()?.prev_token()
  }

  /// Returns the next token in source order.
  pub fn next_token(&self) -> Option<Self> {
    if let Some(sibling) = self.next_sibling() {
      return Some(sibling.first_token());
    }

    self.parent()?.next_token()
  }

  /// Returns this node's siblings, including itself.
  ///
  /// Returns `None` for the root because it has no parent.
  pub fn siblings(&self) -> Option<Children<L>> {
    Some(self.parent()?.children())
  }

  /// Replaces this subtree and rebuilds a new green root.
  ///
  /// Calling this on the root returns `replacement`.
  pub fn replace_with(&self, replacement: Green<L>) -> Green<L> {
    let Some(Parent { node, index }) = &self.parent else {
      return replacement;
    };

    let replaced_parent =
      node.green.replace_child(*index, replacement).unwrap();

    node.replace_with(replaced_parent)
  }

  /// Splices this node's children and rebuilds a new green root.
  ///
  /// Returns `None` when this node is a token or the range is invalid.
  pub fn splice_children(
    &self,
    range: impl RangeBounds<usize>,
    replace_with: impl IntoIterator<Item = Green<L>>,
  ) -> Option<Green<L>> {
    let spliced = self.green.splice_children(range, replace_with)?;
    Some(self.replace_with(spliced))
  }

  /// Replaces one child and rebuilds a new green root.
  pub fn replace_child(
    &self,
    index: usize,
    new_child: Green<L>,
  ) -> Option<Green<L>> {
    let replaced = self.green.replace_child(index, new_child)?;
    Some(self.replace_with(replaced))
  }

  /// Inserts one child and rebuilds a new green root.
  pub fn insert_child(
    &self,
    index: usize,
    new_child: Green<L>,
  ) -> Option<Green<L>> {
    let inserted = self.green.insert_child(index, new_child)?;
    Some(self.replace_with(inserted))
  }

  /// Removes one child and rebuilds a new green root.
  pub fn remove_child(&self, index: usize) -> Option<Green<L>> {
    let removed = self.green.remove_child(index)?;
    Some(self.replace_with(removed))
  }

  /// Removes this node from its parent and rebuilds a new green root.
  ///
  /// Returns `None` for the root.
  pub fn remove_self(&self) -> Option<Green<L>> {
    let Some(Parent { node, index }) = &self.parent else {
      return None;
    };
    node.remove_child(*index)
  }

  /// Returns whether this node is the root red node.
  pub fn is_root(&self) -> bool {
    self.parent.is_none()
  }
}

impl<L: Language> Deref for Red<L> {
  type Target = RedNode<L>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<L: Language> Debug for Red<L> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let range = self.range();
    let start: u32 = range.start().into();
    let end: u32 = range.end().into();
    write!(f, "Red({start}..{end})@{}", Arc::as_ptr(&self.0).addr())
  }
}
