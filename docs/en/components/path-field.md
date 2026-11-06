# PathField

`PathField` is path text plus a browse button. It is a leaf compound: writing a property makes `assemble_path_field` build the `TextInput` and `IconButton` itself, with no separate assembly step to remember. The control cannot reach the parent window, so browsing does not open a system dialog.

The control table has no `<PathField>`.

## Basic usage

`PathField::new(value)` takes the current path.

The default accessible name is "Path"; replace it with `.label`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::PathField;

view! {
    <Widget
        of={PathField::new("/tmp/project")
            .placeholder("Choose a directory")
            .label("Project path")}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::PathField;

widget(
    PathField::new("/tmp/project")
        .placeholder("Choose a directory")
        .label("Project path"),
)
```

:::

After receiving `BrowseRequested`, opening the dialog and writing the chosen path back into this field are both yours.

The field only displays the path and updates `value` when the text changes.

## Browsing

When the button activates, and the field is not disabled, it emits `BrowseRequested`.

You then open the system dialog and write the resulting path back to `value`.

The browse button is an assembled `IconButton` with a folder icon and the accessible name "Browse".

Disabling the field disables the input and this button together, and the button no longer emits `BrowseRequested`.

## Synchronization

`.placeholder`, `.size`, `.disabled` and `.invalid` are written on the field and synchronized to the inner input during assembly; disabling also disables the browse button.

A `TextChanged` from the input is written into `value` and re-emitted unchanged.

When you change `value`, `placeholder` or `label`, that write rebuilds them itself.

The default accessible name is "Path".

## Invalid

When invalid, the border uses the danger color and the accessible state is marked `invalid`.

## Child nodes

The `input` and `browse` child nodes are filled in by assembly.

Do not put controls into these two slots yourself.

The `input` and `browse` child nodes are filled by `assemble_path_field`.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `value` | — | `PathField::new(value)` takes the current path. Updated when the text changes. The input's `TextChanged` is written in and re-emitted unchanged |
| `.label` | — | Replaces the default accessible name "Path" |
| `.placeholder` | — | Written on the field and synchronized to the inner input during assembly |
| `.size` | — | Written on the field and synchronized to the inner input during assembly |
| `.disabled` | — | Synchronized to the input and also disables the browse button. While disabled the button no longer emits `BrowseRequested` |
| `.invalid` | — | When invalid, the border uses the danger color and the accessible state is marked `invalid` |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `BrowseRequested` | — | Emitted when the button activates, as long as the field is not disabled. Opening the dialog and writing the path back to `value` are both yours |
| `TextChanged` | — | One from the input is written into `value` and re-emitted unchanged |

## Slots

| Slot | Description |
| --- | --- |
| `input` | Filled by `assemble_path_field`. Do not put controls into this slot yourself |
| `browse` | Filled by `assemble_path_field` as an `IconButton` with a folder icon and the accessible name "Browse". Do not put controls into it yourself |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
