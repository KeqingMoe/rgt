use crate::{green::Green, lang::Language};
use text_size::TextSize;

/// Errors reported while finishing an event-style tree build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
  /// `finish_node` was called without a matching `start_node`.
  NoOpenNode,
  /// `finish` was called while at least one node was still open.
  UnclosedNode,
  /// `finish` did not end with exactly one root element.
  ExpectedSingleRoot,
}

/// Event-style builder for green trees.
///
/// Parsers can push `start_node`, `token`, and `finish_node` events as they
/// recognize syntax. Finished elements are attached to the current open node, or
/// to the top-level result list when no node is open.
pub struct Builder<L: Language> {
  stack: Vec<Frame<L>>,
  done: Vec<Green<L>>,
}

struct Frame<L: Language> {
  kind: L::Kind,
  children: Vec<Green<L>>,
}

impl<L: Language> Builder<L> {
  /// Creates an empty builder.
  pub fn new() -> Self {
    Self {
      stack: Vec::new(),
      done: Vec::new(),
    }
  }

  /// Starts a new non-token node.
  ///
  /// The node is completed by the next matching [`finish_node`](Self::finish_node).
  pub fn start_node(&mut self, kind: L::Kind) {
    self.stack.push(Frame {
      kind,
      children: Vec::new(),
    });
  }

  /// Adds a token to the current open node or to the top level.
  pub fn token(&mut self, kind: L::Kind, width: TextSize, payload: L::Payload) {
    self.push_element(Green::token(kind, width, payload))
  }

  /// Finishes the most recently opened node.
  ///
  /// Returns [`BuildError::NoOpenNode`] when there is no open node to finish.
  pub fn finish_node(&mut self) -> Result<(), BuildError> {
    let Some(frame) = self.stack.pop() else {
      return Err(BuildError::NoOpenNode)?;
    };

    self
      .push_element(Green::node(frame.kind, frame.children.into_boxed_slice()));

    Ok(())
  }

  /// Finishes the builder and returns the single root green element.
  ///
  /// Returns [`BuildError::UnclosedNode`] if any node is still open, and
  /// [`BuildError::ExpectedSingleRoot`] if the top level does not contain
  /// exactly one element.
  pub fn finish(mut self) -> Result<Green<L>, BuildError> {
    if !self.stack.is_empty() {
      return Err(BuildError::UnclosedNode);
    }

    if self.done.len() != 1 {
      return Err(BuildError::ExpectedSingleRoot);
    }

    Ok(self.done.pop().unwrap())
  }

  fn push_element(&mut self, element: Green<L>) {
    match self.stack.last_mut() {
      Some(frame) => frame.children.push(element),
      None => self.done.push(element),
    }
  }
}

impl<L: Language> Default for Builder<L> {
  fn default() -> Self {
    Self::new()
  }
}
