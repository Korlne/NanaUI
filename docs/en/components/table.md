# Table

`Table` is a table retained in the tree. It does not compare data itself. The sort indicator on a column header and the column order are recorded by the adjacent `VirtualTableLayout`; the row order is sorted by you, because only you know how two rows compare.

## Basic usage

The visible controls are three types. `Table::new()` has the accessible role table, and `.label` gives the whole table a name. `TableRow::new()` is one row, and `.selected` marks the current row. `TableCell::new(value)` is one cell, whose text is that value. `.column_header(true)` projects the cell as a column header, and otherwise it is a data cell. Cells are clickable and focusable by default. The table and its rows take no pointer input themselves.

The control table has no `<Table>`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{Table, TableCell, TableRow};

view! {
    <Widget of={Table::new().label("Files")}>
        <Widget of={TableRow::new()}>
            <Widget of={TableCell::new("Name").column_header(true)} />
            <Widget of={TableCell::new("Size").column_header(true)} />
        </Widget>
        <Widget of={TableRow::new()}>
            <Widget of={TableCell::new("main.rs")} />
            <Widget of={TableCell::new("12 KB")} />
        </Widget>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{Table, TableCell, TableRow};

widget(Table::new().label("Files")).children((
    widget(TableRow::new()).children((
        widget(TableCell::new("Name").column_header(true)),
        widget(TableCell::new("Size").column_header(true)),
    )),
    widget(TableRow::new()).children((
        widget(TableCell::new("main.rs")),
        widget(TableCell::new("12 KB")),
    )),
))
```

:::

## Geometry

Geometry lives in `nana_ui::runtime::VirtualTableLayout`. `VirtualTableLayout::new(row_extents, columns)` takes each row's height and a set of `nana_ui::TableColumn`. `TableColumn::new(key, extent)` gives a column key and width. `.sortable(true)` marks this column's header as a sort control. `.limits(min, max)` bounds the width, and `.resizable` allows resizing.

`move_column(from, to)` reorders columns, keeping widths and the total width, with `to` the target index after removal. `VirtualTableLayout` answers only which column shows the sort indicator, and the column geometry.

## Sorting

You do the sort comparison. A header activation goes through `toggle_sort(column)`: ascending, then descending, then off. The third activation clears the sort instead of returning to ascending. A column that is not sortable, or not in the table, is rejected. `set_sort` publishes the indicator directly; `None` means no sort.

When a header is activated, you call `toggle_sort`, read the current `TableSort`, sort your own data by that column key and direction, and then change the order of the table's child nodes.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `.label` | — | On `Table::new()`, gives the whole table a name. The accessible role is table |
| `.selected` | — | On one row of `TableRow::new()`, marks the current row |
| `value` | — | The text of `TableCell::new(value)` is that value |
| `.column_header` | — | `true` projects the cell as a column header, otherwise it is a data cell |
| `row_extents` | — | Each row's height taken by `VirtualTableLayout::new` |
| `columns` | A set of `nana_ui::TableColumn` | Same as above |
| `key` | — | The column key of `TableColumn::new(key, extent)` |
| `extent` | — | The width of `TableColumn::new(key, extent)` |
| `.sortable` | — | `true` marks this column's header as a sort control |
| `.limits(min, max)` | — | Bounds the width |
| `.resizable` | — | Allows resizing |

## Events

There is no separate event type. Sorting from a header is done by calling `toggle_sort` or `set_sort` yourself; see above.

## Slots

There are no named slots. Rows and cells are child nodes; see basic usage.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
