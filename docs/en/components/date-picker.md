# DatePicker

`DatePicker` is a month grid. It is assembled from existing controls: two icon buttons and a title in the header, then 6×7 date buttons below. You can put it in a form, or in a `Popover` of your own.

The control table has no `<DatePicker>`.

## Basic usage

`assemble_date_picker` builds these buttons and reuses them; changing month only swaps labels, with no rebuild.

`DatePicker::new(cursor)` shows the month `cursor` falls in.

`DatePicker::selected(value)` also marks that day as selected and opens its month.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::DatePicker;
use nana_ui_core::CivilDate;

let cursor = CivilDate::new(2026, 9, 1).expect("valid date");
view! {
    <Widget
        of={DatePicker::new(cursor)
            .month_label("September 2026")
            .range(Some(cursor), CivilDate::new(2026, 9, 30))}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::DatePicker;
use nana_ui_core::CivilDate;

let cursor = CivilDate::new(2026, 9, 1).expect("valid date");
widget(
    DatePicker::new(cursor)
        .month_label("September 2026")
        .range(Some(cursor), CivilDate::new(2026, 9, 30)),
)
```

:::

## Dates

The date type is `nana_ui_core::CivilDate`, which has only a year, a month and a day, and `CivilDate::new(year, month, day)` gives an `Option`.

It is not a date-time library.

`CivilDate::new` returns `None` for a month and day that do not exist.

## Selection range

`.value` is the selected day, `.week_start` decides which day the week starts on, and `.range(minimum, maximum)` is a closed interval; days outside it and days not in this month cannot be selected.

`.disabled` makes the whole day unselectable.

`.size` is passed to the header icons and the date buttons.

Both ends of `range` are `Option`; a missing side sets no bound.

Cells outside the range and outside this month cannot be selected, and changing month only swaps labels.

The selected day is stored in `value`; whether to treat it as a business date is still your decision.

## Month title

You give the month title through `month_label`.

Month names depend on the locale, and the framework ships no locale data.

You change `month_label` in `DateCursorMoved`.

`shift_month` only moves `cursor`.

## Assembly

`slot_assembler` calls `assemble_date_picker` when the view is committed.

`assemble_date_picker` is the `slot_assembler`: it runs when the view is committed and after a reactive patch.

After placing it by hand with `create_component`, call it once yourself.

Changing `cursor` or `month_label` does not re-assemble these 6×7 buttons on every field write.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `DatePicker::new(cursor)` | `nana_ui_core::CivilDate` | Shows the month `cursor` falls in |
| `DatePicker::selected(value)` | `nana_ui_core::CivilDate` | Also marks that day as selected and opens its month |
| `.value` | — | The selected day. Whether to treat it as a business date is still your decision |
| `.week_start` | — | Decides which day the week starts on |
| `.range(minimum, maximum)` | `Option` | A closed interval. Both ends are `Option`; a missing side sets no bound. Days outside it and outside this month cannot be selected |
| `.disabled` | — | Makes the whole day unselectable |
| `.size` | — | Passed to the header icons and the date buttons |
| `month_label` | — | You give the month title. Month names depend on the locale, and the framework ships no locale data |
| `shift_month` | — | Only moves `cursor` |

## Events

A selection emits `DateChanged { date }`; changing month emits `DateCursorMoved { year, month }` and leaves the selected value unchanged.

| Event | Payload | Description |
| --- | --- | --- |
| `DateChanged` | `{ date }` | A selection emits `DateChanged { date }` |
| `DateCursorMoved` | `{ year, month }` | Changing month emits `DateCursorMoved { year, month }` and leaves the selected value unchanged. You change `month_label` in `DateCursorMoved` |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
