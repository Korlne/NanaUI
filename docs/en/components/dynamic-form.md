# DynamicForm

`DynamicForm` assembles settings pages, extension configuration, and request forms from stable field descriptions (`DynamicFormField`). Each field's `id` is a stable key owned by the application (non-empty, no `/`). When a field with the same key updates, its underlying control, input focus, and edit undo history are kept; adding or removing neighboring fields does not rebuild unchanged fields.

There is no `<DynamicForm>` template tag in the control table; both authoring styles mount it with `widget(DynamicForm::new(...))`. For the dense two-column property panel of a professional tool inspector (shared, draggable label column width, mixed values), use [PropertyGrid](property-grid.md).

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{DynamicForm, DynamicFormEvent, DynamicFormField, DynamicFormOption};

widget(DynamicForm::new([
    DynamicFormField::section("general", "General"),
    DynamicFormField::field("name", "Project name", "NanaApp")
        .binding_identity("project-name")
        .hint("Shown in the title bar"),
    DynamicFormField::toggle("enabled", "Enable sync", true),
    DynamicFormField::choice(
        "mode",
        "Run mode",
        "safe",
        [
            DynamicFormOption::new("safe", "Safe"),
            DynamicFormOption::new("fast", "Fast"),
        ],
    ),
    DynamicFormField::action("reset", "Restore defaults").danger(true),
]))
.on(|event: &DynamicFormEvent| {
    // Update application state and validation results by field id
})
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{DynamicForm, DynamicFormEvent, DynamicFormField, DynamicFormOption};

widget(DynamicForm::new([
    DynamicFormField::section("general", "General"),
    DynamicFormField::field("name", "Project name", "NanaApp")
        .binding_identity("project-name")
        .hint("Shown in the title bar"),
    DynamicFormField::toggle("enabled", "Enable sync", true),
    DynamicFormField::choice(
        "mode",
        "Run mode",
        "safe",
        [
            DynamicFormOption::new("safe", "Safe"),
            DynamicFormOption::new("fast", "Fast"),
        ],
    ),
    DynamicFormField::action("reset", "Restore defaults").danger(true),
]))
.on(|event: &DynamicFormEvent| {
    // Update application state and validation results by field id
})
```

:::

When you create it by hand through `create_component`, call `cx.assemble_dynamic_form(form)?`; in a declarative view the framework assembles it automatically.

## Field types and edit history

`DynamicFormField` provides seven row descriptions:

| Constructor | Generated control | Optional modifiers |
| --- | --- | --- |
| `DynamicFormField::section(id, label)` | `SettingsCard` group card (following fields belong to this group until the next `section`) | — |
| `DynamicFormField::toggle(id, label, checked)` | `Switch` | `.disabled(bool)` |
| `DynamicFormField::choice(id, label, selected, options)` | `SettingsRow` + single-select `Dropdown` | `.disabled(bool)`, `.hint(text)`; options use `DynamicFormOption::new(value, label).disabled(bool)` |
| `DynamicFormField::field(id, label, value)` | `SettingsRow` + `TextInput` (or `TextArea` with `.multiline(true)`) | `.multiline(bool)`, `.secure(bool)`, `.disabled(bool)`, `.hint(text)`, `.binding_identity(id)` |
| `DynamicFormField::text(id, value)` | Static `Text` | — |
| `DynamicFormField::error(id, value)` | Danger-state `ValidationMessage` | — |
| `DynamicFormField::action(id, label)` | `Button` (`Subtle` by default, `Danger` with `.danger(true)`) | `.enabled(bool)`, `.danger(bool)` |

**Edit identity (`binding_identity`)**: when the same field `id` switches to a different business object (for example, the list selection moves to another record), pass the new object's unique identifier to `.binding_identity(...)`. The form clears that input's undo/redo history automatically while keeping the node and focus.

## Properties

| Property / method | Type | Description |
| --- | --- | --- |
| `DynamicForm::new(fields)` | `impl IntoIterator<Item = DynamicFormField>` | Creates the form from a list of field descriptions |
| `.fields(fields)` | `impl IntoIterator<Item = DynamicFormField>` | Replaces the list of field descriptions |
| `.label(text)` | `impl Into<Arc<str>>` | Accessible name of the form (role `Form`) |
| `.style(style)` | `NodeStyle` | Root node style |
| `field_nodes()` | `&[(Arc<str>, StableNodeId)]` | After assembly, returns the root node ID of each field (and group card) in description order |

## Events

`DynamicFormEvent` reports only the field key (`field: Arc<str>`) and the new value; it does not modify business state for the application:

| Event variant | Payload | Description |
| --- | --- | --- |
| `DynamicFormEvent::Activate` | `{ field }` | An `Action` button was clicked |
| `DynamicFormEvent::Toggle` | `{ field, checked }` | A `Toggle` switch was flipped |
| `DynamicFormEvent::Select` | `{ field, value }` | An item was selected in a `Choice` dropdown |
| `DynamicFormEvent::Input` | `{ field, value }` | Text was entered in a single-line or multi-line `Field` |

## Slots

| Slot | Description |
| --- | --- |
| None | Group cards, settings rows, and inner controls are created from field descriptions and kept by key |

## See also

[PropertyGrid](property-grid.md) · [FormField](form-field.md) · [SettingsRow](settings-row.md) · [InteractionRequestCard](interaction-request-card.md) · [Overview](index.md)
