# FindReplaceBar

`FindReplaceBar` (alias `TextSearchBar`) provides a collapsible find-and-replace toolbar. It only keeps the query and replacement drafts, the expanded state, the read-only gate, and the feedback text; searching the document, locating matches, and writing changes back are up to the application. The query and replacement boxes are retained `TextInput` child nodes, so IME, selection, and accessibility all go through the standard input path.

There is no `<FindReplaceBar>` template tag in the control table; both authoring styles mount it with `widget(FindReplaceBar::new())`.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{FindReplaceBar, FindReplaceEvent};

widget(
    FindReplaceBar::new()
        .query("nana")
        .expanded(true)
        .feedback("1 / 3"),
)
.on(|event: &FindReplaceEvent| {
    // Handle intents such as QueryChanged / Next / Replace
})
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{FindReplaceBar, FindReplaceEvent};

widget(
    FindReplaceBar::new()
        .query("nana")
        .expanded(true)
        .feedback("1 / 3"),
)
.on(|event: &FindReplaceEvent| {
    // Handle intents such as QueryChanged / Next / Replace
})
```

:::

When you create it by hand through `create_component`, call `cx.assemble_find_replace_bar(bar)?` to assemble the internal retained child nodes; inside a declarative view (`widget(...)`) the framework assembles it automatically.

## Behavior and the read-only gate

- **Collapse and expand**: collapsed by default (only the toggle button shows). Clicking the toggle button or setting `.expanded(true)` expands the query row and replacement row and emits `FindReplaceEvent::Expanded(bool)`.
- **Read-only mode (`read_only`)**: keeps the query input and "Previous / Next" navigation available, and disables the replacement input and the "Replace / Replace all" buttons.
- **Fully disabled (`disabled`)**: disables every input and button; activation no longer emits events.
- **Default strings**: placeholders, button text, and accessible names read the `find.*` entries of `FrameworkStrings` by default (see [Framework strings](../reference/framework-strings.md)). You can override them individually with `.query_placeholder(...)`, `.replacement_placeholder(...)`, and `.label(...)`.

## Properties

| Property / method | Type | Description |
| --- | --- | --- |
| `FindReplaceBar::new()` | — | Creates a find-and-replace bar that is collapsed and writable by default |
| `.query(text)` | `impl Into<String>` | Current query draft |
| `.replacement(text)` | `impl Into<String>` | Current replacement draft |
| `.expanded(bool)` | `bool` | Whether the panel is expanded. Defaults to `false` |
| `.read_only(bool)` | `bool` | Read-only document gate: keeps find and navigation, disables replace. Defaults to `false` |
| `.disabled(bool)` | `bool` | Disables the whole toolbar. Defaults to `false` |
| `.feedback(text)` | `impl Into<Arc<str>>` | Result feedback text (for example `"2 / 5"` or `"No matches"`) |
| `.query_placeholder(text)` | `impl Into<Arc<str>>` | Query box placeholder. Defaults to `FrameworkStrings::find_query` |
| `.replacement_placeholder(text)` | `impl Into<Arc<str>>` | Replacement box placeholder. Defaults to `FrameworkStrings::find_replacement` |
| `.label(text)` | `impl Into<Arc<str>>` | Accessible name of the toolbar (role `Toolbar`). Defaults to `find.title` |
| `.style(style)` | `NodeStyle` | Root node style |
| `toggle_id()` / `query_input_id()` | `Option<StableNodeId>` | After assembly, reads the node ID of the toggle button or query input (for example, to focus it programmatically) |

## Events

The component reports user intent through `FindReplaceEvent`:

| Event variant | Payload | Description |
| --- | --- | --- |
| `FindReplaceEvent::QueryChanged(String)` | Current query | The query input changed |
| `FindReplaceEvent::ReplacementChanged(String)` | Current replacement | The replacement input changed |
| `FindReplaceEvent::Previous` | None | "Previous" was clicked |
| `FindReplaceEvent::Next` | None | "Next" was clicked |
| `FindReplaceEvent::Replace` | None | "Replace" was clicked (not emitted when `read_only`) |
| `FindReplaceEvent::ReplaceAll` | None | "Replace all" was clicked (not emitted when `read_only`) |
| `FindReplaceEvent::Expanded(bool)` | Whether expanded | The user clicked the toggle button to expand or collapse the panel |

## Slots

| Slot | Description |
| --- | --- |
| None | The toggle button, query/replacement inputs, feedback text, and action buttons are all retained and assembled inside the component |

## See also

[TextInput](text-input.md) · [SearchDropdown](search-dropdown.md) · [Overview](index.md) · [Controls](../reference/components.md)
