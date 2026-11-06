# InteractionRequestCard

`InteractionRequestCard` is a neutral request-and-approval card surface: the title comes from `title`, the prompt and body are passed through the `prompt` and `body` slots, key-value form items are described by `fields`, and the action buttons at the bottom are passed through `actions`. It only composes and retains this subtree. It does not store business state or emit workflow or permission events for the application; the host listens directly to the input controls and buttons it put in.

There is no `<InteractionRequestCard>` template tag in the control table. In a declarative view, mount it with `widget(InteractionRequestCard::new(...))` together with `.child_slot` / `.children_slot`, or assemble it with the imperative API.

## Basic usage

Each `InteractionRequestField` has a stable `key` (non-empty, no `/`) and the node ID of a control created by the host. The component creates a `FormField` wrapper for each field automatically and keeps it by `key`: when the label, hint, or error text for the same `key` updates, the identity of the wrapper and inner control stays the same, and input focus and edit history are not lost when neighboring fields change.

```rust
use nana_ui::runtime::{
    Button, FormField, InteractionRequestCard, InteractionRequestField, Text, TextInput,
};

let input = cx.create_detached_component(document, TextInput::new(""))?;
let prompt = cx.create_detached_component(document, Text::new("Please provide connection details"))?;
let action = cx.create_detached_component(document, Button::new("Continue"))?;

let card = cx.create_component(
    document,
    InteractionRequestCard::new("Input required")
        .prompt(prompt.stable_id())
        .actions([action.stable_id()])
        .field(
            InteractionRequestField::new("host", "Host address", input.stable_id())
                .hint("For example 127.0.0.1:8080"),
        ),
)?;
cx.assemble_interaction_request_card(card)?;
```

The application writes field values, validation, and actions back to the `TextInput`, `Select`, or `Button` it owns. In the accessibility tree the card is projected as a `Region` named by `title` (with `busy = true` while `loading`).

## Properties

| Property / method | Type | Description |
| --- | --- | --- |
| `InteractionRequestCard::new(title)` | `impl Into<Arc<str>>` | Card title, also the accessible name of the `Region` |
| `.kind(kind)` | `CardKind` | Card surface style. Defaults to `CardKind::Surface` |
| `.loading(bool)` | `bool` | Loading state (the accessibility `busy` flag). Defaults to `false` |
| `.prompt(node)` | `StableNodeId` | Prompt node slot |
| `.body(node)` | `StableNodeId` | Body node slot |
| `.actions(nodes)` | `impl IntoIterator<Item = StableNodeId>` | List of action button nodes |
| `.field(field)` / `.fields(fields)` | `InteractionRequestField` | A single key-value field description or a whole batch |
| `.style(style)` | `NodeStyle` | Root node style |
| `field_wrappers()` | `Iterator<Item = (&str, StableNodeId)>` | After assembly, queries the `FormField` wrapper node ID for each field `key` |

Constructors and methods of `InteractionRequestField`:

| Method | Type | Description |
| --- | --- | --- |
| `InteractionRequestField::new(key, label, control)` | `(Arc<str>, Arc<str>, StableNodeId)` | The field's stable key, label, and the control node created by the host |
| `.hint(text)` | `impl Into<Arc<str>>` | Hint text passed to the outer `FormField` |
| `.error(text)` | `impl Into<Arc<str>>` | Validation error text passed to the outer `FormField` |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | The card itself emits no business events; listen directly to the events of the controls passed in `fields` and `actions` |

## Slots and child order

After assembly, child nodes are in a fixed order:

1. The `FormField` wrapper node for each of `fields` (with the host's `control` mounted inside);
2. `prompt` (if provided);
3. `body` (if provided);
4. Each action node in `actions`.

Old child nodes that are removed are parked automatically, so no ghost nodes are left behind.

## See also

[Card](https://github.com/sena-nana/NanaUI/blob/main/docs/components/card.md) · [FormField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/form-field.md) · [DynamicForm](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dynamic-form.md) · [Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md)
