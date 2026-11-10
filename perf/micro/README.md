Not Issue #8 DoD and not a gate. See [`perf/README.md`](../README.md).

- [`stack-depth/`](stack-depth/): the least thread stack a layout, or a whole
  NanaUI frame, needs at a tree depth, and cold layout time and held heap per
  box on a page-shaped tree, beside [Taffy](https://github.com/DioxusLabs/taffy)
  laying out the same boxes on the same machine. Its own workspace and
  lockfile; `cargo run --release -- report` and `cargo run --release -- wide`.
  The numbers and what they are compared against are in
  [`docs/reference/resource-budgets.md`](../../docs/reference/resource-budgets.md).
- [`taffy-reference/`](taffy-reference/): NanaUI beside its performance
  reference, [Taffy](https://github.com/DioxusLabs/taffy) 0.11. Block, flex
  and grid pages with the same CSS in both engines (`check` confirms every
  box matches): cold layout, incremental edits, held heap and instructions
  per box; and, NanaUI alone, the capabilities Taffy has no counterpart for
  (inline text, float, sticky and fixed, moves under relative geometry, hit
  testing and accessibility, deep trees). Its own workspace and lockfile;
  Taffy is a dependency of this benchmark only. `cargo run --release --
  compare`, `-- features`, `-- check`. The numbers and what the extra cost
  buys are in
  [`docs/reference/taffy-reference.md`](../../docs/reference/taffy-reference.md).
