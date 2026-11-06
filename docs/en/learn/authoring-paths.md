# Authoring Paths

NanaUI offers two ways to author UI: Rust (the `nana_ui::runtime` builders and the `view!` macro) and the `.vue` dialect (single-file components whose script is Rust). Both become ordinary Rust view functions at build time, and underneath there is always **one retained tree (`UiWorld`)** and **one painter (`SceneWgpuPainter`)**. This chapter takes apart each path's pipeline, shows where they converge, and gives source-level evidence for why a second tree never appears.

## Convergence overview

```text
Path one (Rust)        view! / css! / stylesheet! and function-style builders
                        │ nana-ui-view-macros → view-codegen (calls the CSS engine at build time)
Path two (.vue dialect) <script setup lang="rust"> single-file components
                        │ nana-ui-sfc (application build.rs) → view-codegen (calls the CSS engine at build time)
                        ▼
        Runtime view functions + StylePatch data
                        │ MutationQueue (nana-ui-runtime's UiMutation vocabulary)
                        │ UiWorld::commit (whole-batch validation + atomic apply)
                        ▼
        UiWorld (nana-ui-runtime, the single authoritative retained tree)
                        ▼
        UiScene → SceneWgpuPainter (nana-ui, the single painter)
```

The two paths are **two written forms**, not two independent runtimes: their styles all flow into the same Style Model (Tokens + Semantics + Layout), and all their nodes are written into the same `UiWorld`.

## Path one: Rust (the `view!` macro chain and function-style views)

On the Rust path you can use either the `<Tag>` template syntax or the equivalent function-style builders (such as `column().gap(8).children(...)`):

```rust
use nana_ui::runtime::view;

view! {
    <Column gap=8>
        <Button @activate={save}>"OK"</Button>
    </Column>
}
```

The build-time compilation pipeline:

| Stage | Crate | Responsibility |
| --- | --- | --- |
| 1. Proc-macro entry | `nana-ui-view-macros` | Parses the `view!`, `css!` and `stylesheet!` syntax trees |
| 2. Control schema | `nana-ui-view-schema` | Provides the single intermediate representation of built-in control tags, fields and events (`for_each_control!`) |
| 3. Code generation | `nana-ui-view-codegen` | Expands into Runtime view function calls; CSS in `<style>` and `css!` is parsed by `nana-ui-css` **at build time**, producing Style Model patch data directly |

Source evidence:

- `crates/nana-ui-runtime/src/view/mod.rs:151-162`: the `stylesheet!` macro is defined in Runtime and forwards to `nana_ui_view_macros`:
  ```rust
  macro_rules! stylesheet { ... }
  pub use nana_ui_view_macros::stylesheet as __stylesheet;
  ```
- What the runtime receives is **pure-data compiler output**: the module docs at `crates/nana-ui-runtime/src/view/style.rs:3-8` state that the `.vue` compiler and `css!` parse and match view stylesheets at build time; the runtime does not parse CSS or match selectors, and this crate contains no CSS engine.
- Dependency graph evidence: neither `nana-ui-runtime` nor `nana-ui` has `nana-ui-css` among its normal dependencies, only `view-macros` and `view-schema` (see [Modules and layers](/learn/modules-and-layers)).

## Path two: the `.vue` dialect (compiled to Rust at build time)

The script of a `.vue` single-file component is Rust (`<script setup lang="rust">`). `nana-ui-sfc` compiles it into a standard Rust view function in the application's `build.rs` (for example, `TodoList.vue` produces `pub fn todo_list(...) -> impl IntoView`). The template and `<style>` are handed to the same `nana-ui-view-codegen` for expansion, and the output is written to `$OUT_DIR/nana_views.rs` (see [Authoring toolchain](/learn/toolchain)):

- The generated code refers to `nana_ui::runtime` through a path string and lands on the same set of view functions as hand-written `view!`;
- CSS in `<style>` is likewise parsed and matched by `nana-ui-css` **at build time**, and the runtime receives `StylePatch` data (`crates/nana-ui-runtime/src/view/style.rs:3-9`).

## Convergence point: `UiMutation` and `UiWorld::commit`

The `UiMutation` vocabulary is defined in **`nana-ui-runtime`** (`crates/nana-ui-runtime/src/mutation.rs:15`, with `MutationQueue` at `:306`): `Create`, `Insert`, `Detach`, `SetStyle`, `StartAnimation`, `RequestFocus`, and so on. Every write from view functions and component assemblers is a standard mutation already defined by Runtime, validated as a whole batch and applied atomically by `UiWorld::commit` (see [The retained tree](/learn/runtime-tree)). Neither path has a second set of tree-operation primitives of its own.

## "Only one tree" audit

| Potential violation | Measured result in the current workspace |
| --- | --- |
| A path maintains its own retained tree structure | **None.** Both paths become Runtime view functions at build time; writes enter `UiWorld` only through `MutationQueue`. |
| A path maintains its own render pipeline | **None.** Drawing always goes through the Scene host in `nana-ui` (`UiWorld → UiScene → SceneWgpuPainter`). |
| A second component instantiation ABI exists | **None.** Built-in and extension components all go through `ComponentRegistry`. |

## Two things for learners to notice

1. **Finding the convergence point matters more than finding the fork**: the two paths fully converge at `UiMutation` / `UiWorld`, so once you learn the retained tree API, the internal behavior of both paths follows.
2. **CSS is parsed only at build time**: one engine (`nana-ui-css`), called by `view-codegen` at build time for `css!`, `stylesheet!` and `.vue` `<style>`; the runtime only receives Style Model data.

## Read next

- [Retained tree: UiWorld](/learn/runtime-tree): the node table and commit mechanism after the two paths converge
- [CSS engine](/learn/css-engine): the internal pipeline called at build time
- [End-to-end data flow](/learn/flows): the full journey of one state update from template to screen pixels
- [.vue dialect (guide)](/guide/scaling/sfc): the `.vue` dialect guide for application developers
