# SearchDropdown

`SearchDropdown` is a single-value field with a query. The query uses committed `TextInput` state, not a separate input field. You still supply the options; the control matches them against the query by substring.

## Basic usage

There is no `<SearchDropdown>` in the control table.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{SearchDropdown, SearchDropdownOption};

view! {
    <Widget
        of={SearchDropdown::new(Some("main"))
            .placeholder("Search files")
            .options([
                SearchDropdownOption::new("main", "main.rs").hint("src"),
                SearchDropdownOption::new("lib", "lib.rs"),
            ])}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{SearchDropdown, SearchDropdownOption};

widget(
    SearchDropdown::new(Some("main"))
        .placeholder("Search files")
        .options([
            SearchDropdownOption::new("main", "main.rs").hint("src"),
            SearchDropdownOption::new("lib", "lib.rs"),
        ]),
)
```

:::

While the menu is open and the query is empty, the surface shows the placeholder; when the query is not empty, it shows the query itself. Once closed, a selected value with a match shows that option's label.

## Options

An option is `SearchDropdownOption::new(value, label)`; `.hint` adds a line of supplementary text: in the menu it is drawn at the end of the row in the secondary color, and in the collapsed field it runs on the same line as the label. The value in the select event is the option's `value`.

## Query

`.query(...)` initializes the query; `set_query` changes both the input state and highlights the first item. The query string follows the committed input; read the current value with `query_text()`.

Matching is case-insensitive and looks at the label, the value, and `hint`. With an empty query all options are visible. `visible_indices` are the indices still visible under the current query.

The filtering rule lives in the control. To use a different ordering or remote results, change `options`.

## Opening and closing

After a selection, the control writes `value` itself and closes the menu. While disabled or loading, `toggle_open` emits no event. While disabled or loading, the field is `inactive`, and the press that opens the menu does not emit `Opened`.

`close` emits `Closed` only when the menu is open, and clears the highlight.

## Screen reader

The open menu is a list box under the field, like [Select's screen reader support](https://github.com/sena-nana/NanaUI/blob/main/docs/components/select.md#读屏), but lists only options matched by the query: position and total count are computed over matching items, and options hidden by the query have no node. Options are identified by value, so option nodes that remain when the query changes stay the same. Activating an item from the screen reader selects the option that row shows.

## Properties

The `value` of `SearchDropdown::new(value)` is the currently selected value and can be `None`. Other configurable fields are `placeholder`, `size`, `disabled`, `loading`, `invalid`, `opened`, and `highlighted`; the query is set with `.query(...)` and read with `query_text()`.

| Property | Type | Description |
| --- | --- | --- |
| `value` | — | `SearchDropdown::new(value)`. Currently selected value, can be `None`. The control writes it after a selection |
| Options | `SearchDropdownOption` | `SearchDropdownOption::new(value, label)`; `.hint` adds a line of supplementary text |
| `.query(...)` | — | Initializes the query; `query_text()` reads the current value, which follows the committed `TextInput` |
| `placeholder` | — | Field. While the menu is open and the query is empty, the surface shows the placeholder |
| `size` | — | Field |
| `disabled` | — | Field. While disabled it is `inactive`, `toggle_open` emits no event, and opening does not emit `Opened` |
| `loading` | — | Field. While loading it is likewise `inactive` and `toggle_open` emits no event |
| `invalid` | — | Field |
| `opened` | — | Field |
| `highlighted` | — | Field. `set_query` moves the highlight to the first item; `close` clears the highlight when open |
| `menu_scroll` | — | Field. The menu's scroll position; `set_query` returns the menu to the first match. For menu placement, scrolling, and scrollbar, see [Select's menu](https://github.com/sena-nana/NanaUI/blob/main/docs/components/select.md#菜单) |

## Events

The event is `SearchDropdownEvent`: typing a query emits `Search`, selecting emits `Select`, and opening and closing emit `Opened` and `Closed`.

| Event | Payload | Description |
| --- | --- | --- |
| `Search` | — | Query typed |
| `Select` | The option's `value` | Selected. The control then writes `value` itself and closes the menu |
| `Opened` | — | Opened. While disabled or loading, the press that opens the menu does not emit it |
| `Closed` | — | Closed. `close` emits it only when open, and clears the highlight |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
