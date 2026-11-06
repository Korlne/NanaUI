# Spinner

`spinner()` creates an indeterminate spinner. The template tag is `<Spinner>`. It takes no constructor arguments; internally it is `Spinner::new("")`.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Spinner label="Loading" />
}
```

```rust rust
use nana_ui::runtime::view::spinner;

spinner().label("Loading")
```

:::

`label` can be a constant, a `Signal<Arc<str>>` or a closure. It is also the accessible name. The control is marked busy.

## Label

The only field is `label: Arc<str>`. There are no events and no `model`. An empty label is just a spinner. With a label, the spinner sits to the left of the text as the leading slot; the left inset is then the spinner's own slot and belongs to the framework, and only the other three sides are the caller's padding.

## Rotation

Once mounted, rotation is driven by an infinite Motion IR timeline. The host does not feed the phase frame by frame. Projection starts that timeline only when it is not already turning, and does not wind back the phase of one that is running.

Stopping it means unmounting or parking the node. Parking keeps the node identity and application state but takes it out of layout, painting, input, focus and accessibility until the subtree is inserted again. When hidden through layout (`hidden`), the timeline stops too, so the host is not kept awake for a spinner nobody sees. Do not pause it by writing the phase yourself. The phase belongs to animation scheduling and is not in the field table.

The timeline repeats linearly and has no finished frame. Reprojecting does not set the phase back to 0.

## Size

Size is on the `Spinner` component as `Spinner::size`, not a `<Spinner>` attribute. A non-finite or non-positive size falls back to the default.

## Accessibility

The accessibility role is a progress indicator, and it is always marked busy. An empty label keeps that role, with an empty string as the name.

## Covers

The loading spinner used on covers is drawn by `Thumbnail` itself, centered, with a side length twice that of the compact `Spinner`. That is not this page's control. A standalone `Spinner` with a label still sits to the left.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `label` | `Arc<str>` | The only field. Can be a constant, a `Signal<Arc<str>>` or a closure, and is also the accessible name. An empty label is just a spinner |
| `Spinner::size` | — | On the component, not a `<Spinner>` attribute. A non-finite or non-positive size falls back to the default. The phase is not in the field table |
| `Spinner::label_visible` | — | On the component. With `false` the label only serves as the accessible name and is not drawn next to the spinner, for compact inline status |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | There are no events and no `model` |

## Slots

| Slot | Description |
| --- | --- |
| Leading slot | With a label, the spinner sits to the left of the text. The left inset is the spinner's own slot and belongs to the framework. Only the other three sides are the caller's padding |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
