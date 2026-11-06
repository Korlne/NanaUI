# Toast

A passive notification: a title, an optional description, and the tone it speaks in (`ToastTone`: Info / Success / Warning / Danger). `Toast::new(title, tone)` creates it. The control table has no `<Toast>` tag; both authoring styles call `widget(Toast::new(..))`. Appearance belongs to the theme's `ToastRecipe`; see [Theme](../reference/theme.md).

A toast comes in two forms. A bare toast is one painted node, with one line each for the title and description. [`AppContext::assemble_toast`](#composite-form) permanently replaces it with a composite form built from child nodes: the text wraps, and it gains a progress bar, an action button, a pressable close button, and its own icon. For a notification corner use [ToastStack](toast-stack.md), which assembles every toast pushed into it.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{Toast, ToastTone};

view! {
    <Widget of={Toast::new("Saved", ToastTone::Success).description("Project exported")} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{Toast, ToastTone};

widget(Toast::new("Saved", ToastTone::Success).description("Project exported"))
```

:::

`description("")` is the same as no description. `dismissible(true)` makes a bare toast a pressable, focusable close target. A bare toast does not read `progress`, `action`, `icon`, or `close_label`; these appear only after assembly.

## Placement and timeout

`Toast::place_in(viewport, align, max_width, insets)` uses the same reservation contract as `Panel::viewport` to pin the toast into the free area the application leaves, against the reserved bottom edge; see [Controls](../reference/components.md). A placed toast keeps the width given by `place_in`.

`Toast::timeout(Duration)` makes the toast emit `ToastDismissed` once after it has been on screen that long, the same event as pressing the close button. The timer does not advance per frame; it wakes the host only when it expires. The framework does not remove toasts itself; the application removes one after receiving the event.

## Composite form

```rust
use nana_ui::runtime::{Toast, ToastAction, ToastDismissed, ToastProgress, ToastTone};

let toast = cx.create_component(
    document,
    Toast::new("Optimizing model", ToastTone::Info)
        .description("2 / 5")
        .progress(ToastProgress::Determinate(0.4))
        .action("View")
        .dismissible(true),
)?;
cx.assemble_toast(toast)?;
cx.on(toast, |_, _: &ToastAction, _| { /* open details */ })?;
cx.on(toast, |_, _: &ToastDismissed, _| { /* remove this toast */ })?;
```

Assembly is permanent; calling it again does nothing (returns `false`). After that, every write to the toast is reconciled in place against its children by its assembler: when progress changes, the same progress bar moves, with no rebuild. A whole-value rewrite (`*toast = Toast::new(..)`) does not drop the assembly either.

An assembled toast lays out in one row:

- **Tone mark**: the recipe's dot or glyph, painted in a square reserved on the left, standing on the title's first line rather than in the middle of a multi-line toast. When the recipe marks tone with a glyph (`ToastIndicator::Glyph`), `Toast::icon` replaces the tone's glyph; when the recipe uses a dot or no mark, the recipe wins.
- **Text column**: the title wraps and truncates by the recipe's `title_max_lines`, the description by `description_max_lines`; `0` means no truncation.
- **Percentage**: with `ToastProgress::Determinate` and the recipe's `progress.show_percent` true, it is written next to the title using the framework string `toast.percent` (`{percent}%`), in tabular figures, with the width measured for `100%` so the title does not jump as the number grows.
- **Progress bar**: at the bottom of the text column; thickness, track, fill color, and top margin (`progress.top_gap`) come from the recipe. With `ToastProgress::Indeterminate` it sweeps, and does not sweep under reduced motion. A determinate value is clamped to `0.0..=1.0`.
- **Action button**: `Toast::action(label)` provides one; its kind and size tier come from the recipe's `action_kind` and `control_size`, and its corner radius from `control_radius` (default `RadiusTier::Sm`). It is vertically centered on the toast's full content height, which includes description wrapping and the progress bar; a single-line toast does not grow taller because of the button. Pressing it emits `ToastAction` on the toast.
- **Close button**: appears when `dismissible`; vertically centered like the action button, `control_size` tier, text kind, corner radius also from `control_radius`. Pressing it emits `ToastDismissed`. Its accessible name is `close_label`, or the framework string `toast.close` ("Dismiss notification") when not given.

Both buttons are vertically centered on the full content height and do not stretch a one-line toast. A theme can set `control_radius` to a tier such as `RadiusTier::Xl` so the text action becomes a pill and the square close button's feedback becomes a circle; the radius resolves against the current theme's metrics and updates in place when the theme switches. The toast's width follows its content (CSS `width: fit-content`), and the text wraps when it does not fit; when showing progress it is at least the recipe's `progress.min_width` (default 240), and the progress bar does not stretch with the text. A placed toast keeps the width given by `place_in`.

An assembled toast pauses its timer while the pointer hovers it or focus is inside it, and continues with the remaining time from the next frame after it leaves. Inside a [ToastStack](toast-stack.md), the stack decides when time counts.

## Layout

A bare toast's mark, title, description, and close target are content parts published by layout (#205), with no child nodes: the recipe's padding reserves a band inward from the border box, and the band holds a Flex row (mark `LeadingContent`, the text column, close target `TrailingContent`). The text column in turn lays out the title (`TextContent`, one line as tall as `title.size`) and the description (`Detail`) as a Flex column. With a description the text column sits at the top of the band; without one it is vertically centered in the band like the mark and close target. The toast's height comes from measuring this declaration (the larger of the band's block height and one title line), and the style no longer writes `min_height`; content-based widths such as `width: fit-content` include the mark, gap, and close target. The title is one line, shaped to the width between the mark and the close target, and truncated with an ellipsis when it does not fit, instead of wrapping across the whole content width and stretching the toast. Painting only reads the published parts.

An assembled toast publishes only the mark as a part: layout places the mark and the gap after it before the text children, inside the recipe's padding, and the mark stands on the center line of the title's first line; the padding is only the recipe's.

## Accessibility

A toast is a Status named by its title and described by its description. Bare and assembled toasts are the same. An assembled toast is also a live region: polite for ordinary tones, assertive for danger. Changing the title in place (for example when a task completes) is announced once by the platform's live region. Text in the text column is not read twice; the progress bar is named by the title and does not announce changes separately; both buttons are reachable with Tab.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `title` | `Arc<str>` | Constructor argument |
| `tone` | `ToastTone` | Constructor argument. Decides the tone mark's color; assertive for danger |
| `description` | `Option<Arc<str>>` | An empty string is the same as none |
| `dismissible` | `bool` | Bare toast: a pressable close target; assembled: a close button |
| `timeout` | `Option<Duration>` | Emits `ToastDismissed` once after being on screen this long. Without it the toast waits indefinitely |
| `progress` | `Option<ToastProgress>` | Assembled only. `Determinate(0.0..=1.0)` or `Indeterminate` |
| `action` | `Option<Arc<str>>` | Assembled only. Text for one action button; an empty string is the same as none |
| `icon` | `Option<Icon>` | Assembled only. Replaces the tone's glyph when the recipe marks tone with a glyph |
| `close_label` | `Option<Arc<str>>` | Assembled only. Accessible name of the close button; `toast.close` when not given |
| `style` | `NodeStyle` | Style of the outer frame; `place_in` writes only position and width in it |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `.on(\|_: &ToastDismissed\| …)` | `&ToastDismissed` | The close button was pressed, or `timeout` elapsed (fires once). The application does the removal |
| `.on(\|_: &ToastAction\| …)` | `&ToastAction` | The assembled action button was pressed |

## Slots

| Slot | Description |
| --- | --- |
| None | Parts are built by `assemble_toast`; application children are not accepted |

## See also

[Overview](index.md) · [ToastStack](toast-stack.md) · [Controls](../reference/components.md) · [Framework strings](../reference/framework-strings.md)
