use crate::{green::Green, lang::Language};
use text_size::TextSize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
  NoOpenNode,
  UnclosedNode,
  ExpectedSingleRoot,
}

pub struct Builder<L: Language> {
  stack: Vec<Frame<L>>,
  done: Vec<Green<L>>,
}

struct Frame<L: Language> {
  kind: L::Kind,
  base: Option<L::Payload>,
  children: Vec<Green<L>>,
}

impl<L: Language> Builder<L> {
  pub fn new() -> Self {
    Self {
      stack: Vec::new(),
      done: Vec::new(),
    }
  }

  pub fn start_node(&mut self, kind: L::Kind, base: Option<L::Payload>) {
    self.stack.push(Frame {
      kind,
      base,
      children: Vec::new(),
    });
  }

  pub fn token(&mut self, kind: L::Kind, width: TextSize, payload: L::Payload) {
    self.push_element(Green::token(kind, width, payload))
  }

  pub fn finish_node(&mut self) -> Result<(), BuildError> {
    let Some(frame) = self.stack.pop() else {
      return Err(BuildError::NoOpenNode)?;
    };

    self.push_element(Green::node(
      frame.kind,
      frame.base,
      frame.children.into_boxed_slice(),
    ));

    Ok(())
  }

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
