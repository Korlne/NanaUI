# CalendarHeatmap

`CalendarHeatmap` draws a heatmap of cells in week columns. You provide the dates, values, titles and meaning. Runtime handles the geometry and the level mapping.

There is no `<CalendarHeatmap>` in the control table.

## Basic usage

This type is behind the Cargo feature `calendar`.

`components` turns it on.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{CalendarHeatmap, CalendarHeatmapDatum};

view! {
    <Widget
        of={CalendarHeatmap::new([
            CalendarHeatmapDatum::new("2026-09-01", 1.0),
            CalendarHeatmapDatum::new("2026-09-02", 4.0),
        ])
        .week_starts_on(1)
        .label("Commits")}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{CalendarHeatmap, CalendarHeatmapDatum};

widget(
    CalendarHeatmap::new([
        CalendarHeatmapDatum::new("2026-09-01", 1.0),
        CalendarHeatmapDatum::new("2026-09-02", 4.0),
    ])
    .week_starts_on(1)
    .label("Commits"),
)
```

:::

## Data

`CalendarHeatmap::new(data)` takes `CalendarHeatmapDatum`s.

The date in `CalendarHeatmapDatum::new(date, value)` is a string, and `.data` can attach your own payload.

The control does not interpret the time zone of date strings, and does not fill in days that have no data for you.

## Weeks and names

`.week_starts_on(day)` sets which day a week starts on.

`.label` is the accessible name; an empty string falls back to the default "Calendar heatmap".

## Levels

`.level_strategy` swaps in your own bucketing.

Level 0 uses the theme's `subtle`; the other levels mix the accent color into subtle.

## Cells

The cell side length constant is `CELL_SIZE` 11 and the spacing is `CELL_GAP` 3.

The active cell is recorded in `active`.

On hover, read the date, value and title from `CellEnter` and `CellMove` and decide how to write the tooltip yourself.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `CalendarHeatmap::new(data)` | `CalendarHeatmapDatum` | Takes this data |
| `CalendarHeatmapDatum::new(date, value)` | — | The date is a string |
| `.data` | — | Can attach your own payload |
| `.week_starts_on(day)` | — | Sets which day a week starts on |
| `.label` | — | Accessible name. An empty string falls back to the default "Calendar heatmap" |
| `.level_strategy` | — | Swaps in your own bucketing |
| `active` | — | The active cell is recorded here |
| `CELL_SIZE` | — | The cell side length constant is 11 |
| `CELL_GAP` | — | The spacing constant is 3 |

## Events

The event is `CalendarHeatmapEvent`: `CellEnter` and `CellMove` carry a `CalendarHeatmapActiveCell`, and `CellLeave` means the pointer left.

| Event | Payload | Description |
| --- | --- | --- |
| `CalendarHeatmapEvent::CellEnter` | `CalendarHeatmapActiveCell` | On hover, read the date, value and title here and decide how to write the tooltip yourself |
| `CalendarHeatmapEvent::CellMove` | `CalendarHeatmapActiveCell` | On hover, read the date, value and title here and decide how to write the tooltip yourself |
| `CalendarHeatmapEvent::CellLeave` | — | Means the pointer left |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
