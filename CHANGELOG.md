# Changelog

## 0.1.1

- Added `Debug` implementations for `Green` and `Red`.
- Added `Eq` and `Hash` implementations for `Red` based on its underlying green node identity.
- Fixed `Red::covering_node` for empty ranges. Empty ranges now descend only when they are strictly inside a non-empty child, and stay on the parent when they sit between adjacent children.

## 0.1.0

- Initial release.
