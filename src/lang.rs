use crate::green::Green;

/// Defines the syntax vocabulary and payload model for a tree family.
///
/// A language chooses its own syntax kind type and payload type. `rgt` never
/// requires kinds to convert through a fixed integer representation, and it
/// does not prescribe what a payload contains.
pub trait Language: Sized {
  /// Syntax kind stored on every green element.
  type Kind: Clone + Copy + 'static;

  /// User-defined data stored on every green element.
  ///
  /// Tokens receive their payload directly from [`Green::token`]. Nodes receive
  /// their payload from [`compose_node`](Self::compose_node).
  type Payload: Clone + 'static;

  /// Composes the payload for a non-token node from its kind and children.
  ///
  /// The full child slice is passed so composition can inspect child kinds,
  /// widths, payloads, or tree shape. This is deliberately more general than a
  /// binary merge operation.
  fn compose_node(kind: Self::Kind, children: &[Green<Self>]) -> Self::Payload;
}
