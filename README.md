# rgt

`rgt` is a small immutable red/green tree library for syntax trees.

It is syntax tree infrastructure, not a parser framework. It does not store
source text, it does not require syntax kinds to round-trip through `u16`, and
it does not do interning or deduplication. The tree stores widths and user
payloads; the language layer decides what those payloads mean.

The main design pressure is diagnostic-friendly syntax trees. A parser can put
diagnostics on error tokens, aggregate `has_diag` flags into parent nodes, keep
token text outside the tree, or store token text in payloads when that is more
convenient. `rgt` provides the tree shape and navigation API, while the payload
model stays under the user's control.

## Model

`rgt` uses the usual green/red split:

- `Green<L>` is immutable, parentless tree data. It stores a syntax kind, a
  width, a payload, and either children or token-ness.
- `Red<L>` is a positioned view over a green tree. It adds parent links, child
  indexes, and text offsets for navigation.
- `Language` defines the syntax kind and payload types for a tree family.
- `Builder` builds a green tree from parser-style events.

Green equality and hashing use `Arc` pointer identity. This keeps identity
simple for immutable sharing, without requiring a global interner.

## Why not rowan?

`rowan` is a good fit when you want its full model: green nodes with text stored
in the tree, language kinds convertible through `u16`, and a mature ecosystem
around typed AST wrappers. `rgt` is for a different set of constraints.

The main difference is payload ownership. In `rgt`, every green element carries
a language-defined payload, and node payloads are composed from the full child
slice. This makes diagnostic summaries, custom token storage, cache markers, or
other per-subtree data part of the immutable tree itself without forcing them
into side tables.

`rgt` also keeps source text outside the tree by default. Tokens have widths and
payloads; callers decide whether token text belongs in the payload, in a source
database, or somewhere else. Syntax kinds are just the language's associated
type, not a fixed integer representation.

Finally, `rgt` does not try to be an interned or deduplicated tree store. Green
nodes are ordinary `Arc` values with pointer identity. That keeps the MVP small
and makes sharing explicit.

## Payloads

Payloads are deliberately general. A payload can be a token's text, a diagnostic,
a folded summary of child diagnostics, a cache key, or a custom mix of those.

When a node is built or its children are edited, `Language::compose_node` gets
the node kind and the full `&[Green<L>]` child slice. This is intentionally not
a binary merge API: payload composition can inspect child kinds, widths,
payloads, and tree shape.

## Example

```rust
use rgt::{green::Green, lang::Language};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
  Root,
  ItemList,
  Function,
  ParamList,
  Block,
  Ident,
  LParen,
  RParen,
  Error,
}

#[derive(Clone, Debug)]
struct Diag {
  // ...
}

#[derive(Clone, Debug)]
enum DiagPayload {
  Diag,
  HasDiag,
}

type Payload = Option<DiagPayload>;

struct Lang;

impl Language for Lang {
  type Kind = Kind;
  type Payload = Payload;

  fn compose_node(_kind: Kind, children: &[Green<Self>]) -> Payload {
    children
      .iter()
      .any(|child| child.payload().is_some())
      .then_some(DiagPayload::HasDiag)
  }
}
```

## Navigation

Use `Red::new_root` to create a positioned view from a green root. From there,
the red API provides parent/child/sibling navigation, preorder traversal,
token traversal, offset queries, covering-node queries, and replacement helpers
that rebuild a new green root.

Offset queries return `AtOffset<T>`. Most offsets point at one element, but a
boundary between two adjacent elements returns `AtOffset::Between(left, right)`;
call `left_biased` or `right_biased` when a caller wants a deterministic side.
