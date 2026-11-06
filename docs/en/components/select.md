# Select

A single-value dropdown. Option identity belongs to the application; the framework does not invent a separate id. For multiple selection, or to share the field surface with a menu, use [Dropdown](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dropdown.md). To search while typing, use [SearchDropdown](https://github.com/sena-nana/NanaUI/blob/main/docs/components/search-dropdown.md).

Its template tag is `<Select>`. It has no constructor arguments. Options are given with `.options(…)`.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Select placeholder="Language" options={options} v-model={language} />
}
```

```rust rust
use nana_ui::runtime::view::select;

select().placeholder("Language").options(options).model(language)
```

:::

`v-model={language}` expands to `.model(language)`. `language` is a `Signal<Option<Arc<str>>>`. After the user selects an item, the control writes the selected value itself and closes the menu. The application does not need to write it again to see it. The value in the event is an `Arc<str>`, and the model always wraps it in `Some`.

## Options

An option is `SelectOption::new(value, label)`; both arguments go into `Arc<str>`. To disable an item, use `SelectOption::new("rust", "Rust").disabled(true)`. Disabled options still appear in the open menu.

`.hint(text)` gives an option a piece of secondary text, such as the original name after a translated one: `SelectOption::new("x", "Head turn left/right").hint("FaceAngleX")`. In the menu it sits at the end of the row, one size smaller than the label, in the secondary color; in the collapsed field it follows the selected item's label. Width goes to the label first; the hint is elided when it does not fit, and is not drawn at all when not even a short piece fits. The screen reader reads it after the label, even when it is not drawn. An empty string means no hint.

`.image(slot)` gives an option a round image, drawing host texture slot `slot`, the same as an [Avatar](https://github.com/sena-nana/NanaUI/blob/main/docs/components/avatar.md) on the same slot (Cover, cropped to a circle; in the scene it is the same `nana.host-texture` resource, named by the slot name). In the menu it sits before the row's label; in the collapsed field, when the selected item has an image, it is drawn before the label. Its side length is the line height of text at this size, one step larger than an icon, and the row does not grow taller. Options without an image lay out as before with no empty space reserved. An empty slot draws a Subtle placeholder circle and the label does not move; the convention for missing slots rejecting a frame is the same as Avatar. The image is decorative only; the screen reader reads only the label.

`options` can be a `Vec<SelectOption>` or a signal of that type. This is a single-value field, not multiple selection.

## Loading, disabled, and name

`loading` and `disabled` can be constants, signals, or closures.

A dropdown draws its value, not its name. `label` is its name: the screen reader says what it is for, and it is not drawn (`aria-label`). With nothing given, the screen reader can only use the displayed option as the name, reading "Ascending" instead of "Sort direction". The displayed option is always its value.

:::api

```rust view
view! {
    <Select label="Sort direction" options={directions} v-model={direction} />
}
```

```rust rust
select().label("Sort direction").options(directions).model(direction)
```

:::

When a visible field caption is already nearby, you do not need to write the name again: `labelled_by` points to that caption (`aria-labelledby`), the screen reader uses the caption's text, and follows it when the caption changes. The caption can come before or after the dropdown; in the template, use the caption's `ref`:

:::api

```rust view
view! {
    <Text ref={caption}>"Sort direction"</Text>
    <Select labelled_by={caption} options={directions} v-model={direction} />
}
```

```rust rust
let caption = node_ref();
(
    text("Sort direction").node_ref(caption),
    select().labelled_by(caption).options(directions).model(direction),
)
```

:::

When placed in a settings row, the row label names it as well. When its own `label` is not empty, its own label is used.

Without `placeholder` it is `None`. The type is `Option<Arc<str>>`.

## Sizing to the options

By default the dropdown fills the row. To size it to the longest option like a native browser `<select>`, turn on `fit_options`:

:::api

```rust view
view! {
    <Select fit_options={true} options={repos} v-model={repo} />
}
```

```rust rust
select().fit_options(true).options(repos).model(repo)
```

:::

The field's content width is then the widest of all options (and, when nothing is selected and the placeholder shows, the placeholder too), plus room for the arrow; the field width is that plus padding and border. Options with a hint are measured together with the hint, at the label's font size, erring on the wide side. Choosing a different option does not change the field width; adding or removing options does.

`fit_options` only determines the dropdown's own content width, and matters only when width is determined by content: `width: max-content` in the style (function API `.css(css! { width: max-content; })`), or `LengthSpec::Shrink` when changing the `Select` style directly. The default width still fills the row, and a definite width still uses that width; `min_width` and `max_width` apply as usual, for example adding `min_width` to keep a minimum width. Without `fit_options` it behaves as before: the content width considers only the item currently shown and reserves no room for the arrow.

When the field is narrower than the longest option (a definite width is set, or it is capped by `max_width`), the open menu still expands to the longest item, so options are not truncated. The menu starts at the field's left edge; when its right side goes past the area where the menu can be seen (see "Menu" below), it shifts left to sit exactly at the right boundary; when it is wider than the whole area, it is cut to the area's width. The menu is never narrower than the field. Without `fit_options`, the menu is as wide as the field.

```rust
let mut select = Select::new(Some(current)).options(options).fit_options(true);
let layout = Arc::make_mut(&mut select.style.layout);
layout.width = Some(LengthSpec::Shrink);
layout.min_width = Some(LengthSpec::Px(122.0));
```

## Menu

The menu opens below the field. When it does not fit below but fits above, it flips above; when it fits on neither side, it opens on the side with more space and its height is cut to that side. Space means where the menu can be seen: the window, minus every container outside the field that clips it (scroll views, boxes clipped by `overflow`, dialog cards), at their current scroll positions. Which containers clip it follows the clip chain, the same as drawing and clicking the field: an `absolute` field is not clipped by containers it escapes, and a `fixed` field (or one with a `fixed` box outside it) only counts up to there, so the menu of a `fixed` field can use the whole window. So a dropdown inside a card does not open its menu where the card clips, and after the page scrolls it does not open the menu outside the window based on the unscrolled position. When the page scrolls while the menu is open, the menu moves with the field; it is placed by the new position at the menu's next change (scrolling, moving the highlight) or the next layout pass. When the window resizes or layout moves the field or an outer container, an open menu flips and cuts its height again for the new space, without waiting for the field to project again; what is drawn, what the pointer hits, and what hit testing uses are always the same placement.

The menu is drawn on the top layer of the page, like a Popover panel: blocks behind the field and settings rows later in a card never cover it, and when the pointer is over the menu it belongs to the menu. Only clipping outside the field still applies; the menu is never drawn where that clipping cuts.

The menu has exactly one highlight: on open it is on the currently selected item; it follows the pointer to whichever item it is over (except disabled ones), arrow keys move it too, and Enter selects it. When the pointer is over no item, the highlight stays where it is, and the menu does not scroll to follow the highlight under the pointer.

When there are more options than fit, they scroll inside the menu and are clipped within the menu's border, never drawn outside it:

- The wheel over the menu scrolls the options and does not pass to the page below even at the end. When the menu fits all options, the wheel scrolls the page as usual. While Ctrl is held, the wheel does not belong to the menu.
- When arrow keys move the highlight, the highlighted item is scrolled into view by only the necessary distance. When the menu opens, the currently selected item is placed as close to the middle as possible.
- A scrollbar on the right has the same size and colors as [ScrollView](https://github.com/sena-nana/NanaUI/blob/main/docs/components/scroll-view.md)'s, taken from the theme's `ScrollbarMetrics` and `ScrollbarRecipe`. While the menu is open and does not fit, it is always shown; when the theme's visibility policy is `Always` it has a persistent track, and when it is `Hidden` it is not drawn, while wheel and keyboard work as usual. When the pointer moves onto the scrollbar, the thumb thickens and switches to the hover color; dragging the thumb scrolls the options; pressing the empty track first aligns the thumb to the press point and then continues as a drag; a canceled drag returns to the position before the press. Options leave room for the scrollbar strip, so the thumb never covers text or the highlight.
- What you click is the item drawn under the pointer: only the part visible in the menu accepts clicks, and the scrollbar strip is not an option. This holds after the page scrolls as well.

The scroll position, like the highlight, is interaction state (`Select::menu_scroll`, type `SelectMenuScroll`): it starts over each time the menu opens, and is kept across application refreshes when the value and options do not change. [Dropdown](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dropdown.md) and [SearchDropdown](https://github.com/sena-nana/NanaUI/blob/main/docs/components/search-dropdown.md) use the same menu and behave the same; when SearchDropdown's query changes, it returns to the first match.

## Screen reader

The field is a combo box (`ComboBox`), and the screen reader says whether it is expanded or collapsed. When the menu opens, a list box (`ListBox`) appears under the field, and each row in the menu is an option in it (`ListBoxOption`), the same as a `role="combobox"` controlling a `role="listbox"` on the web:

- An option reads its label, followed by the hint (`description`). The currently selected item is "selected" and the rest are "not selected"; disabled options read as disabled. Each option carries its position in the whole list and the total count (`aria-posinset` / `aria-setsize`), so with only a few rows visible the screen reader still says "item 12 of 40".
- Focus stays on the field. The highlighted item is the field's active descendant (`aria-activedescendant`), and the screen reader follows as arrow keys move the highlight.
- An option's bounds are the area where the menu draws it and the pointer hits it: the same menu geometry, mapped into the window through the field's scrolling and transforms. Rows scrolled out of the menu, and rows entirely clipped by clipping outside the field, have no node; a partly clipped row's bounds are the visible part. When the menu flips, cuts its height, or scrolls, the options follow.
- Options are identified by value; the node stays the same as long as the value does. When the menu closes, the list box and its options are removed together.
- Activating an option from the screen reader (AccessKit `Click`, Runtime `ActivateMenuItem`) is the same as pressing the pointer on that row: it selects it, closes the menu, and emits `SelectChanged`. Disabled options do not respond.

## Vetoing a selection

`SelectChanged` is a selection committed by pointer, keyboard, or accessibility. When `value` is `None` nothing is selected yet. To veto the user's selection, write the value you want back in the handler.

`.on_change(|event: &SelectChanged| …)` listens to commits itself. In a template it is `on:SelectChanged={|event: &SelectChanged| …}`.

`@change={save}` passes `save` through unchanged when it is a function, and its parameter must be `&SelectChanged`. Other expressions expand to `move |_| { … }` and the event is ignored.

## Menu open and close

Each time the menu opens and each time it closes, `SelectToggled { open }` is emitted once. It is emitted however the menu closes: clicking the field, selecting an item (including selecting the item already shown again), Enter, Escape, clicking elsewhere, focus leaving (Tab or the application moving focus), or an application refresh that removes options, disables the control, or sets it to loading. It is not emitted when the written state equals the existing one, such as asking an open menu to open again, or a refresh where the value and options do not change. Within one operation, `SelectChanged` is emitted first, then `SelectToggled`.

`open` says whether the menu is shown: while the control is disabled or loading the menu is not shown, which counts as closed. When the application writes `disabled` or `loading` back to `false` while `opened` is still `true`, the menu shows again and `open: true` is emitted once.

When focus leaves, the menu closes with it, like a native browser `<select>`.

`.on_toggle(|event: &SelectToggled| …)` listens to open and close itself; in a template it is `@toggle` or `on:SelectToggled`.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `value` | `Option<Arc<str>>` | Current selection. `model` writes `Some(event.value)` after a selection |
| `options` | `Vec<SelectOption>` | Can also be a signal of this type. Options can carry `.hint` and `.image`; see above |
| `label` | `Option<Arc<str>>` | Screen reader name, not drawn. When unset, it is named by `labelled_by` or the settings row, and failing those reads the displayed option |
| `placeholder` | `Option<Arc<str>>` | `None` when unset |
| `disabled` | `bool` | Constant, signal, or closure |
| `loading` | `bool` | Constant, signal, or closure |
| `fit_options` | `bool` | When `true`, the content width is the longest option plus room for the arrow, like a native `<select>`. Use it with a content-determined width (`Shrink`). When the field is narrower, the menu still expands to the longest option, kept within the visible area. Default `false` |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `on_change`, template `on:SelectChanged` or `@change` | `&SelectChanged` | Committed by pointer, keyboard, or accessibility. A function must take this reference; other expressions ignore the event |
| `on_toggle`, template `on:SelectToggled` or `@toggle` | `&SelectToggled` | The menu opened or closed; `open` is the resulting state. Emitted once per change; see "Menu open and close" above |

## Slots

No slots. Options are data in `options`, not children.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
