# GraphCanvas

`GraphCanvas` is a controlled graph canvas. You keep the model and its persistence; the control holds the viewport, the selection, and the current pointer gesture, and reports results as events. By default it only draws the grid, node boxes, and edges.

There is no `<GraphCanvas>` in the control table.

## Basic usage

The model in `GraphCanvas::new(canvas_id, model)` is a `GraphModel`.

Use `GraphModel::empty()` for an empty graph.

`GraphModel::new(nodes, edges)` validates its input and returns `GraphModelError` on failure.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{GraphCanvas, GraphModel};

view! {
    <Widget of={GraphCanvas::new("board", GraphModel::empty()).label("Node graph")} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{GraphCanvas, GraphModel};

widget(GraphCanvas::new("board", GraphModel::empty()).label("Node graph"))
```

:::

## Feature

This type is behind the Cargo feature `graph-canvas`.

`components` turns it on.

## Custom painting

The `"graph-canvas"` custom GPU renderer is not attached automatically.

The constant is `GRAPH_CANVAS_RENDERER`.

The default scene painter rejects this key when it is not registered.

To write a pass directly, the host registers it itself and then calls `set_custom_render`.

`custom_render()` only builds that `CustomRenderNode`; the projection does not attach it.

## Assembly

`assemble_graph_canvas_contents` is slot assembly; it is responsible for the child nodes inside graph nodes.

It is not a GPU renderer.

It runs when the view commits; after building the inner slots by hand, call it once yourself.

## Hit testing

A right click still only emits `SecondaryPress`, in window coordinates.

`AppContext::graph_canvas_hit_at` converts it into a canvas-local point and `GraphCanvasHit { local, selection }`.

This is a pure query; it does not change selection, focus, or hover.

Whether a menu opens, and what it contains, is up to you.

## Writing back

While disabled, geometry queries still answer from the boxes.

You write the viewport from the event back with `set_viewport`, and you also commit whether selections and connections enter the model.

## Properties

`.viewport`, `.selection`, `.label`, and `.disabled` are set on the canvas.

`.grid_spacing` only accepts a finite spacing no less than 8.

| Property | Type | Description |
| --- | --- | --- |
| `canvas_id` | — | The `canvas_id` in `GraphCanvas::new(canvas_id, model)` |
| `model` | `GraphModel` | The model of `GraphCanvas::new`. Use `GraphModel::empty()` for an empty graph. `GraphModel::new(nodes, edges)` validates and returns `GraphModelError` on failure |
| `.viewport` | — | Set on the canvas |
| `.selection` | — | Set on the canvas |
| `.label` | — | Set on the canvas |
| `.disabled` | — | Set on the canvas. While disabled, geometry queries still answer from the boxes |
| `.grid_spacing` | — | Only accepts a finite spacing no less than 8 |
| `.node_content` | — | `.node_content(node, content)` binds an inner region or host texture to a node; unknown ids are kept until `set_model` prunes them |

## Events

The event type is `GraphCanvasEvent`: `SelectionChanged`, `NodePositionInput` during a drag, the committed `NodePositionChanged`, `ConnectionRequested`, `ViewportInput` during a drag, and `ViewportChanged` produced by release, wheel, or keyboard.

| Event | Payload | Description |
| --- | --- | --- |
| `GraphCanvasEvent::SelectionChanged` | — | `GraphCanvasEvent` |
| `GraphCanvasEvent::NodePositionInput` | — | During a drag |
| `GraphCanvasEvent::NodePositionChanged` | — | Committed |
| `GraphCanvasEvent::ConnectionRequested` | — | `GraphCanvasEvent` |
| `GraphCanvasEvent::ViewportInput` | — | During a drag |
| `GraphCanvasEvent::ViewportChanged` | — | Produced by release, wheel, or keyboard |
| `SecondaryPress` | — | A right click still only emits this, in window coordinates |

## Slots

You place node interiors in child nodes.

`.node_content(node, content)` binds an inner region or host texture to a node; unknown ids are kept until `set_model` prunes them.

| Slot | Description |
| --- | --- |
| `.node_content` | A node's inner region or host texture. `assemble_graph_canvas_contents` is responsible for these child nodes |

## See also

[Overview](index.md) · [Controls](../reference/components.md)
