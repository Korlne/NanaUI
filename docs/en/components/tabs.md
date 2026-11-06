# Tabs

`Tabs` is a standalone tab strip. Selection, reordering, close requests and dragging between strips are all reported from here. The tab values and their order still belong to you; when a reorder succeeds, the control changes the option order it keeps to match the new one.

## Basic usage

`Tabs::new(selected)` takes the currently selected value. Options are `TabOption::new(value, label)`, which also takes `.icon`, `.disabled`, `.draggable` and `.closable`. The strip itself also has `.label`, `.size`, `.fill`, `.strip_id` and `.accepts_external_drop`.

The control table has no `<Tabs>`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{TabOption, Tabs};

view! {
    <Widget
        of={Tabs::new("main")
            .strip_id("editor")
            .label("Files")
            .options([
                TabOption::new("main", "main.rs").closable(true),
                TabOption::new("lib", "lib.rs").closable(true),
            ])}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{TabOption, Tabs};

widget(
    Tabs::new("main")
        .strip_id("editor")
        .label("Files")
        .options([
            TabOption::new("main", "main.rs").closable(true),
            TabOption::new("lib", "lib.rs").closable(true),
        ]),
)
```

:::

## Closing

Closing is only a request; the control does not delete the tab. `.closable(true)` means only that you accept close requests, and the runtime does not paint a close button.

`request_close` returns `None` when the tab is disabled or not closable. You decide whether to close, and remove it from `options` if you do.

## Reordering and dragging

A tab with `.draggable(false)` can still be selected; it just cannot be a drag source or a "drop before it" target. By default a strip accepts tabs dragged in from other strips; `accepts_external_drop(false)` still allows dragging out and only refuses outside drops.

`reorder` describes the result as "the value moved plus the value after it". `before` in `Reorder` is the value immediately after it once it has moved. It is `None` when the tab lands at the end. `before` being `None` means it lands at the end. The control then arranges the `options` it keeps into this order.

`transfer_to` only reports and changes neither strip. A `Transfer` between strips reports only the source strip, the value and the target position, and you change the option lists on both strips yourself.

`strip_id` distinguishes several tab strips in one window. Dragging tabs across windows uses `TabDragGroup` / `TabDragSurface`, and the selection and order are still yours. When a tab is dragged out of a window, `TabDragGroup` and `TabDragSurface` hand this drag to the other side; which tab is selected and the final order are still written in your state. `Dock` does not add cross-window semantics for this strip.

## Focus

The arrow keys move focus only; they do not change the selection.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `selected` | — | The currently selected value taken by `Tabs::new(selected)` |
| `value` | — | The value of `TabOption::new(value, label)` |
| `label` | — | On an option it is the label of `TabOption::new`. `.label` on the strip belongs to the strip itself |
| `.icon` | — | On an option |
| `.disabled` | — | On an option. While disabled, `request_close` returns `None` |
| `.draggable` | — | `false` still allows selecting it, but not being a drag source or a "drop before it" target |
| `.closable` | — | `true` means only that close requests are accepted, and the runtime does not paint a close button |
| `.size` | — | Belongs to the strip itself |
| `.fill` | — | Belongs to the strip itself |
| `.strip_id` | — | Distinguishes several tab strips in one window |
| `.accepts_external_drop` | — | Accepts tabs dragged in from other strips by default. `false` still allows dragging out and only refuses outside drops |
| `.options` | — | When a reorder succeeds, the option order it keeps changes to the new one. You remove a tab from here after closing it |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `TabsEvent::Select` | — | Selection is reported from this strip |
| `TabsEvent::Reorder` | `{ value, before }` | `before` being `None` means it lands at the end, and it is also the value immediately after it once it has moved. `reorder` describes the result as "the value moved plus the value after it". The control then arranges the `options` it keeps into this order |
| `TabsEvent::Close` | — | Closing is only a request; the control does not delete the tab |
| `TabsEvent::Transfer` | `{ source_strip, value, target_strip, before }` | `transfer_to` only reports and changes neither strip. You change the option lists on both strips yourself |

## Slots

There are no named slots. Tabs come from the option data.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
