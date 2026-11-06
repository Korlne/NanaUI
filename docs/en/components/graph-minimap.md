# GraphMinimap

`GraphMinimap` is an overview of a graph canvas. It scales uniformly to `GraphModel::bounds` and draws node rectangles and a viewport indicator. Clicks and drags emit a new viewport, which you write back with `GraphCanvas::set_viewport`.

There is no `<GraphMinimap>` in the control table.

## Basic usage

It does not change the canvas.

`GraphMinimap::new(model)` starts with an empty viewport and an empty canvas size.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{GraphMinimap, GraphModel, GraphSize};

view! {
    <Widget
        of={GraphMinimap::new(GraphModel::empty())
            .canvas_size(GraphSize::new(640.0, 480.0))
            .label("Overview")}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{GraphMinimap, GraphModel, GraphSize};

widget(
    GraphMinimap::new(GraphModel::empty())
        .canvas_size(GraphSize::new(640.0, 480.0))
        .label("Overview"),
)
```

:::

After receiving `ViewportRequested`, call `set_viewport` on that `GraphCanvas`, then write the same viewport back to the minimap so the indicator follows the pointer.

When the model changes, call `set_model` too; the minimap does not read canvas nodes on its own.

## Feature

Like `GraphCanvas`, this type is behind the Cargo feature `graph-canvas`.

`components` turns it on.

## Projection

The projection is centered in the minimap's own box and keeps the nodes' aspect ratio.

Position and size come from your layout; a common choice is to place it in a corner of the canvas with `PositionSpec::Absolute`.

## Dragging

Canceling a drag requests once more with the viewport recorded at press time.

## Properties

`.canvas_size` passes the graph canvas's visible size, used to compute the indicator and navigation offset; an invalid size leaves the previous value.

`.viewport` stays in sync with the canvas's current viewport.

`.node_fill` can be changed to another semantic color.

`.disabled` stops navigation.

`.label` is the accessible name; an empty string falls back to "Graph minimap".

`.set_model` and `.set_viewport` update an existing node.

| Property | Type | Description |
| --- | --- | --- |
| `model` | `GraphModel` | `GraphMinimap::new(model)` starts with an empty viewport and an empty canvas size |
| `.canvas_size` | — | The graph canvas's visible size, used to compute the indicator and navigation offset; an invalid size leaves the previous value |
| `.viewport` | — | Stays in sync with the canvas's current viewport |
| `.node_fill` | — | Can be changed to another semantic color |
| `.disabled` | — | Stops navigation |
| `.label` | — | Accessible name; an empty string falls back to "Graph minimap" |
| `.set_model` | — | Updates an existing node. Call it when the model changes too; the minimap does not read canvas nodes on its own |
| `.set_viewport` | — | Updates an existing node |

## Events

The only event is `GraphMinimapEvent::ViewportRequested(viewport)`.

| Event | Payload | Description |
| --- | --- | --- |
| `GraphMinimapEvent::ViewportRequested` | `viewport` | Clicks and drags emit a new viewport, which you write back with `GraphCanvas::set_viewport`. Canceling a drag requests once more with the viewport recorded at press time |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](index.md) · [Controls](../reference/components.md)
