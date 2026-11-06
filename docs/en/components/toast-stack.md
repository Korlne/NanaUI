# ToastStack

A column of notifications in one corner of the window: the newest sits on the anchored bottom edge and older ones stack upward. `ToastStack::new()` creates it; toasts go in with `AppContext::push_toast` and come out with `remove_toast`. The control table has no `<ToastStack>` tag; both authoring styles call `widget(ToastStack::new())`.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::ToastStack;

view! {
    <Widget of={ToastStack::new().max_visible(3).label("Notifications")} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::ToastStack;

widget(ToastStack::new().max_visible(3).label("Notifications"))
```

:::

Pushing toasts, listening for events, and removing them are all up to the application:

```rust
use nana_ui::runtime::{PopoverAlignment, Toast, ToastDismissed, ToastProgress, ToastStack, ToastTone};

let mut corner = ToastStack::new();
corner.place_in(window, PopoverAlignment::End, 360.0, insets, 420.0);
let stack = cx.create_component(document, corner)?;

let toast = cx.push_toast(
    stack,
    Toast::new("Optimizing model", ToastTone::Info)
        .progress(ToastProgress::Indeterminate)
        .dismissible(true),
)?;
cx.on(toast, |_, _: &ToastDismissed, _| { /* have the application call remove_toast */ })?;

// The task reports progress: update this toast in place
cx.update_component(toast, |toast, _| {
    toast.progress = Some(ToastProgress::Determinate(0.6));
})?;
cx.remove_toast(stack, toast)?;
```

`push_toast` first assembles the toast ([`assemble_toast`](toast.md#composite-form)), then puts it into the stack as the newest entry and returns the toast's own entity: listen on it and write updates to it. It never moves focus.

## Placement

`ToastStack::place_in(viewport, align, max_width, insets, max_height)` uses the same reservation contract as `Toast::place_in` and `Panel::viewport`: it pins the stack into the free area left by `insets`. `align` decides where the stack sits within the free width and also how toasts align inside the stack (`End` aligns right). `max_width` caps the width, `max_height` caps the height toasts may occupy, and the stack grows upward from the reserved bottom edge. `viewport` is the positioning host's own box. It only writes position and size. When the window size or the reserved area changes, call it again; the stack relays out against the new height.

## Measure first, then show; queueing

A newly pushed toast is first laid out once inside the stack, out of flow and invisible: it is not hit, the screen reader does not hear it, its timer does not run, and toasts already in the stack do not move. Once that layout pass has measured its height, the stack decides which toasts to show: counting from newest to oldest, it shows those while there are fewer than `max_visible` and their heights plus spacing (the recipe's `stack_gap`) do not exceed `max_height`. The newest toast is shown even if it alone is too tall. The rest are queued: not laid out, not heard by the screen reader, timers paused. When room frees up (a toast is removed, the stack is placed taller), queued toasts are measured again and come back.

`shown_toasts(stack)` lists the toasts on screen, newest first, excluding queued ones, ones still being measured, and ones leaving.

## Entering, leaving, and closing up

Motion reads the recipe's `motion`: entering goes from transparent, from `enter_offset_y` and `enter_scale`, back to the resting position, with duration and easing from `enter`. `remove_toast` makes an on-screen toast play its entrance backward in place (`exit`) and removes it only when that finishes; the others slide to their new positions using `stack_move`. Queued toasts and toasts still being measured are removed directly. `clear_toasts` removes every toast this way.

All scaling pivots on the recipe's `origin` (the `transform-origin` written on each toast's slot, center by default): when the stack sits in the bottom-right corner, set it to `100% 100%` so toasts grow out of and shrink back into that corner.

When a toast's text changes in place (a task notification becomes its result), `pulse_toast(stack, toast)` makes the on-screen toast bounce back from the recipe's `change_scale` to its normal size, with duration and easing from `change`, without fading. `NoticeCenter` calls it on a swap only when the title, description, tone, or icon changed; progress bar movement does not count. It does not play when the toast is not yet on screen, its entrance or slide has not finished, reduced motion is on, or `change_scale` is 1 (the built-in value).

The stack records a y coordinate for each toast in the column: how high its bottom edge is above the stack's bottom edge. It checks them after every layout, and every toast whose coordinate changed slides with `stack_move` from where it is currently seen to its new position. A new toast arriving below, a toast below leaving, a toast below growing or shrinking because its text changed (including rewrapping when the stack width changes): all follow this one rule. Because the bottom edge is what is measured, the toast that grew does not move itself; only the ones above it move up to make room. A toast already sliding that gets pushed again continues from wherever it has slid to, without jumping back first. A toast pushed while entering or bouncing slides from where it stands and ends at its resting position with no transform. With reduced motion nothing plays: toasts appear directly, disappear directly, and the rest go straight to their positions.

## Timing

Toasts in the stack only count time while shown. While the pointer is over any toast in the stack, or focus is in the stack, every timer in the stack pauses; after it leaves, each continues with its own remaining time from the next frame. When one expires, `ToastDismissed` fires on that toast, and the application decides whether to remove it. When no motion is playing, the stack wakes the host only for the timer that expires first.

## Pointer and modals

The stack itself does not receive pointer input; only the toasts do. Gaps between toasts and the empty space above them fall through to the stage below.

A modal dialog's card is drawn at `z_index` 1000. When the application places the stack above the modal layer (a `z_index` above 1000 in the same stacking context, or higher on a sibling after the modal host), pointer presses on toasts in the stack reach the toast while the modal is open: actions and close work as usual, they do not count as clicks outside the modal, and the modal does not close. Focus stays in the modal, and Tab still cycles only among the modal's controls. When the stack sits below the modal, the modal covers it as usual. Do not mount the stack under `OverlayHost` to activate it as an overlay; place it as an ordinary positioned node.

## Accessibility

The stack is a Region named by `label`, or by the framework string `toast.region` ("Notifications") when none is given. Each on-screen toast is a live Status (see [Toast](toast.md#accessibility)); queued toasts, ones still being measured, and ones leaving are not heard.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `max_visible` | `usize` | Maximum number on screen at once, not counting ones leaving. Default 3 |
| `label` | `Option<Arc<str>>` | Name of the Region; `toast.region` when not given |
| `style` | `NodeStyle` | `place_in` writes the position, width, max height, and alignment in it; `z_index` is set by the application |

## Methods

| Method | Description |
| --- | --- |
| `AppContext::push_toast(stack, toast)` | Assembles the toast and adds it as the newest; returns the toast's entity. Measures before showing and does not move focus |
| `AppContext::remove_toast(stack, toast)` | An on-screen toast finishes its exit before removal and the others slide to close the gap; queued toasts and ones still being measured are removed directly. Returns `false` when the stack does not hold this toast |
| `AppContext::clear_toasts(stack)` | Removes all toasts |
| `AppContext::shown_toasts(stack)` | Toasts on screen, newest first |
| `AppContext::pulse_toast(stack, toast)` | Bounces back from `change_scale` to normal size after the text changed in place; returns `false` when it did not play |
| `ToastStack::place_in(..)` | See [Placement](#placement) |

## Events

The stack emits no events of its own. `ToastDismissed` and `ToastAction` fire on individual toasts; see [Toast](toast.md#events).

## Slots

| Slot | Description |
| --- | --- |
| None | Toasts go in with `push_toast`, not mounted as children |

## See also

[Overview](index.md) · [Toast](toast.md) · [Controls](../reference/components.md) (including the multi-window notification center `nana_ui::notices`)
