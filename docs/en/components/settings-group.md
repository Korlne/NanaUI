# settings_group

`settings_group(title, rows)` is a group of settings: the title on top, a raised card below, and inset hairlines between rows. It is a view in `view::marks`; inside the card, Runtime's `SettingsGroup` manages the rows and dividers.

## Basic usage

```rust
use nana_ui::runtime::view::{group_row_on_line, settings_group, settings_row};

settings_group(Some("Input".into()), vec![
    group_row_on_line(settings_row("Send").control(send_chord)),
    group_row_on_line(settings_row("Mirror").control(mirror_switch)).visible(advanced),
])
```

- When `title` is `None`, no title is drawn (the page title already says what the group is).
- The card is drawn by `RaisedSurfacePainter`: a thin border, and a short shadow in light mode.

## Content line

Neither the card nor `group_row` **adds horizontal padding**; the hairlines between rows are inset by `CONTENT_INSET` (`SpacingStep::Xxxl`). A row's text and controls must sit on the same content line:

- `group_row_on_line(view)`: equivalent to `group_row(content_line(view))`, the common form;
- `group_row(view)`: use it when `view` already carries this padding (the application's own row constructor), and do not wrap it again.

`settings_row` has no horizontal padding of its own, so it also needs the content line. A row without it (settings rows, buttons, and description text alike) sits against the card edge and does not line up with the inset hairlines.

## Tinted groups

`.inset()` replaces the raised card with a borderless tinted block (`group_block`): the title and the row text stand on the same line, `SpacingRole::Attach` away from the block. The block's padding is the content line, so rows use `group_row(view)`, not `group_row_on_line`; the hairlines between rows span the block's content width.

```rust
use nana_ui::runtime::view::{group_row, settings_group, settings_row};

settings_group(Some("Connection".into()), vec![
    group_row(settings_row("Address").control(address)),
    group_row(settings_row("Port").control(port)),
])
.inset()
```

When you build the title yourself, or the block holds something other than `group_row`, use `group_block(children)` directly: it is the same tinted block, with an inner `SettingsGroup` managing the first and last members and the hairlines; `.quiet()` removes the background and padding, and `.gap(step)` sets the spacing between members.

## Hint placement

`.hint_placement(HintPlacement::Help)` puts the hint of every row in the group behind a help button after its name, so each row stays one line tall; use it where rows are adjusted at a glance, such as an inspector card or a form, and keep the default (the hint under the name) on a settings page. It is written on the group's `SettingsGroup` and reaches every settings row inside; see [SettingsRow's hint placement](https://github.com/sena-nana/NanaUI/blob/main/docs/components/settings-row.md#hint-placement). For a card holding several groups, write it once on the card (`SettingsCard`) or on a group around them, and the groups inside need none; a row's own placement wins.

```rust
use nana_ui::runtime::HintPlacement;

settings_group(Some("Transform".into()), rows).hint_placement(HintPlacement::Help)
```

For a group built by hand, write `SettingsGroup::new().hint_placement(HintPlacement::Help)`; `group_block(..)` has the same method.

## Actions of the group itself

`.footer(view)` goes below the card or block, as close to it as the title is (`Md` for a raised card, `Attach` for a tinted block), so it reads as belonging to this group rather than the next. Buttons that advance a task go here, not in the title tools.

## Hiding rows

`group_row(view).visible(cond)`: a hidden row takes its divider with it, the first row included; a divider shows only between two shown rows. Do not hide only the content inside a row; that leaves an empty row.

## When to use something else

| Content | Use |
| --- | --- |
| A group of related settings | `settings_group` |
| A block of free content (description, chart, row of buttons) | `Card`, keeping its padding |
| A `Card` as a group container, with rows putting themselves on the content line | `Card::new().kind(CardKind::Flat).padding(0.0)` |

The distance between groups comes from the `gap` of the column holding them; groups are distinguished only by title and card, with no extra decoration.

## API

| API | Description |
| --- | --- |
| `settings_group(title, rows)` | A group; accepts `.class` / `.css` / `.visible`, applied to the column holding the title and card |
| `group_row(view)` | A row, placed as-is |
| `group_row_on_line(view)` | A row, placed on the content line |
| `GroupRow::visible(cond)` | Shown conditionally; when hidden, takes its divider with it |
| `content_line(view)` | Puts any view on the content line (adds only horizontal padding) |
| `.inset()` | A tinted block instead of a raised card; title and row text on the same line; rows use `group_row` |
| `.footer(view)` | The group's own actions, directly below the card or block |
| `.hint_placement(placement)` | Where the hints of the group's settings rows go (on `SettingsGroupView`, `GroupBlock`, `SettingsGroup` and `SettingsCard`); see above |
| `group_block(children)` | A tinted block without a title; `.quiet()`, `.gap(step)` |
| `CONTENT_INSET` / `content_inset()` | The content line's step / the pixel value that holds **only under default metrics**: when a theme changes spacing, the content line follows the step and this number does not change. Wherever a step can be written, write `CONTENT_INSET` (`Divider::inset_step`, theme padding) |

## See also

[Layout and composition](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/composition.md) · [SettingsRow](https://github.com/sena-nana/NanaUI/blob/main/docs/components/settings-row.md) · [Card](https://github.com/sena-nana/NanaUI/blob/main/docs/components/card.md) · [Divider](https://github.com/sena-nana/NanaUI/blob/main/docs/components/divider.md)
