use crate::{green::Green, lang::Language};
use std::{
  iter,
  ops::{Deref, RangeBounds},
  sync::Arc,
};
use text_size::{TextRange, TextSize};

struct Parent<L: Language> {
  node: Red<L>,
  index: usize,
}

pub struct RedNode<L: Language> {
  green: Green<L>,
  parent: Option<Parent<L>>,
  offset: TextSize,
}

pub struct Red<L: Language>(Arc<RedNode<L>>);

impl<L: Language> Clone for Red<L> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

pub enum AtOffset<T> {
  Single(T),
  Between(T, T),
}

impl<T> AtOffset<T> {
  pub fn as_ref(&self) -> AtOffset<&T> {
    match self {
      Self::Single(node) => AtOffset::Single(node),
      Self::Between(left, right) => AtOffset::Between(left, right),
    }
  }

  pub fn is_between(&self) -> bool {
    matches!(self, Self::Between(_, _))
  }

  pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> AtOffset<U> {
    match self {
      Self::Single(node) => AtOffset::Single(f(node)),
      Self::Between(left, right) => AtOffset::Between(f(left), f(right)),
    }
  }

  pub fn left_biased(self) -> T {
    match self {
      Self::Single(node) | Self::Between(node, _) => node,
    }
  }

  pub fn right_biased(self) -> T {
    match self {
      Self::Single(node) | Self::Between(_, node) => node,
    }
  }
}

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

pub enum WalkEvent<T> {
  Enter(T),
  Leave(T),
}

pub struct LazyChildren<L: Language> {
  parent: Red<L>,
  front_index: usize,
  front_offset: TextSize,
  back_index: usize,
  back_offset: TextSize,
}

pub struct LazyChild<L: Language> {
  parent: Red<L>,
  index: usize,
  green: Green<L>,
  offset: TextSize,
}

impl<L: Language> LazyChild<L> {
  pub fn range(&self) -> TextRange {
    TextRange::at(self.offset, self.green.width())
  }

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

  pub fn green(&self) -> &Green<L> {
    &self.green
  }

  pub fn kind(&self) -> L::Kind {
    self.green.kind()
  }

  pub fn offset(&self) -> TextSize {
    self.offset
  }

  pub fn width(&self) -> TextSize {
    self.green.width()
  }

  pub fn range(&self) -> TextRange {
    TextRange::at(self.offset, self.width())
  }

  pub fn payload(&self) -> &L::Payload {
    self.green.payload()
  }

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

  pub fn children(&self) -> Children<L> {
    Children(self.lazy_children())
  }

  pub fn preorder(&self) -> Preorder<L> {
    Preorder {
      stack: vec![WalkEvent::Enter(self.clone())],
    }
  }

  pub fn descendants(&self) -> Descendants<L> {
    Descendants {
      preorder: self.preorder(),
    }
  }

  pub fn tokens(&self) -> impl Iterator<Item = Self> {
    self.descendants().filter(|node| node.is_token())
  }

  pub fn child_count(&self) -> Option<usize> {
    self.green.child_count()
  }

  pub fn child(&self, index: usize) -> Option<Self> {
    self.lazy_children().nth(index).map(LazyChild::into_red)
  }

  pub fn first_child(&self) -> Option<Self> {
    self.children().next()
  }

  pub fn last_child(&self) -> Option<Self> {
    self.children().next_back()
  }

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

  pub fn first_token(&self) -> Self {
    let mut node = self.clone();
    while let Some(child) = node.first_child() {
      node = child;
    }
    node
  }

  pub fn last_token(&self) -> Self {
    let mut node = self.clone();
    while let Some(child) = node.last_child() {
      node = child;
    }
    node
  }

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

  pub fn ancestors(&self) -> impl Iterator<Item = Self> {
    iter::successors(Some(self.clone()), |node| node.parent())
  }

  pub fn root(&self) -> Self {
    self.ancestors().last().unwrap()
  }

  pub fn root_green(&self) -> Green<L> {
    self.root().green.clone()
  }

  pub fn covering_node(&self, range: TextRange) -> Option<Self> {
    if !self.range().contains_range(range) {
      return None;
    }

    for child in self.lazy_children() {
      let r = child.range();
      if let Some(inter) = r.intersect(range)
        && !inter.is_empty()
      {
        if inter == range {
          return child.into_red().covering_node(range);
        }
        break;
      }
    }

    Some(self.clone())
  }

  pub fn parent(&self) -> Option<Self> {
    self.parent.as_ref().map(|p| p.node.clone())
  }

  pub fn index(&self) -> Option<usize> {
    self.parent.as_ref().map(|p| p.index)
  }

  pub fn prev_sibling(&self) -> Option<Self> {
    let parent = self.parent()?;
    let index = self.index()?;
    index.checked_sub(1).and_then(|index| parent.child(index))
  }

  pub fn next_sibling(&self) -> Option<Self> {
    let parent = self.parent()?;
    let index = self.index()?;
    parent.child(index + 1)
  }

  pub fn prev_token(&self) -> Option<Self> {
    if let Some(sibling) = self.prev_sibling() {
      return Some(sibling.last_token());
    }

    self.parent()?.prev_token()
  }

  pub fn next_token(&self) -> Option<Self> {
    if let Some(sibling) = self.next_sibling() {
      return Some(sibling.first_token());
    }

    self.parent()?.next_token()
  }

  pub fn siblings(&self) -> Option<Children<L>> {
    Some(self.parent()?.children())
  }

  pub fn replace_with(&self, replacement: Green<L>) -> Green<L> {
    let Some(Parent { node, index }) = &self.parent else {
      return replacement;
    };

    let replaced_parent =
      node.green.replace_child(*index, replacement).unwrap();

    node.replace_with(replaced_parent)
  }

  pub fn splice_children(
    &self,
    range: impl RangeBounds<usize>,
    replace_with: impl IntoIterator<Item = Green<L>>,
  ) -> Option<Green<L>> {
    let spliced = self.green.splice_children(range, replace_with)?;
    Some(self.replace_with(spliced))
  }

  pub fn replace_child(
    &self,
    index: usize,
    new_child: Green<L>,
  ) -> Option<Green<L>> {
    let replaced = self.green.replace_child(index, new_child)?;
    Some(self.replace_with(replaced))
  }

  pub fn insert_child(
    &self,
    index: usize,
    new_child: Green<L>,
  ) -> Option<Green<L>> {
    let inserted = self.green.insert_child(index, new_child)?;
    Some(self.replace_with(inserted))
  }

  pub fn remove_child(&self, index: usize) -> Option<Green<L>> {
    let removed = self.green.remove_child(index)?;
    Some(self.replace_with(removed))
  }

  pub fn remove_self(&self) -> Option<Green<L>> {
    let Some(Parent { node, index }) = &self.parent else {
      return None;
    };
    node.remove_child(*index)
  }

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
