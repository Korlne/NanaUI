# TextInput

One line of text. The initial value goes through `value`, not a constructor argument. For multiple lines use [TextArea](https://github.com/sena-nana/NanaUI/blob/main/docs/components/text-area.md).

The template tag is `<TextInput>`.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <TextInput placeholder="New task" v-model={draft} />
}
```

```rust rust
use nana_ui::runtime::view::text_input;

text_input().placeholder("New task").model(draft)
```

:::

`v-model={draft}` expands to `.model(draft)`. When `draft` is a `Signal<String>`, typing writes back to that one signal, and text that reads it updates with it.

## Submit

Submit on Enter with `on:TextSubmitted={|event: &TextSubmitted| …}`. The function form is `.on_submit(|event: &TextSubmitted| …)`.

`on_input` has type `TextChanged`, and the handler receives a reference to the event. `model` binds `value`, and typing emits `TextChanged`.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `value` | `String` | The current text. `model` binds this field |
| `label` | `Option<Arc<str>>` | Accessible name |
| `placeholder` | `Arc<str>` | The hint shown while it is empty |
| `disabled` | `bool` | Cannot be edited |
| `invalid` | `bool` | The value is wrong and a Danger frame is drawn; on a settings row the reason is written in the row's `error` |
| `character_counter` | `Option<usize>` | Shows the committed character count and the limit inside the box on the right; off by default |

## Alignment

On a single line, text shorter than the box is placed per the style's `text-align`: start edge by default, `End` at the end edge, `Center` centered, `Left` / `Right` by physical direction; the caret, selection and clicks follow. Text longer than the box is unaffected and scrolls as before. The multi-line editor ignores it.

## Character count

The Rust component uses `TextInput::new("").character_counter(40)`; views use
`text_input().character_counter(Some(40))` or `<TextInput character_counter={Some(40)} />`.
The semantic attribute is `character-counter="40"`.

The count uses `chars().count()` (Unicode scalars): spaces are included and pre-edit text not yet committed by the input method is excluded.
An empty value shows `0/40`, and the count uses the danger colour once the limit is exceeded. The several scalars of a composed emoji each count separately.
The count area stays inside the input box on the right; scrolling, clipping and hit testing of the body, placeholder, caret, selection and pre-edit text share the remaining edit area.

This is a display capability. It still allows typing, pasting and submitting; gating business sends is the application's job.
`max_length` is a separate hard UTF-16 editing limit; the two can be used together.

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `on_input` | `&TextChanged` | While typing. `model` also writes back to `value` at this point |
| `on_submit`, template `on:TextSubmitted` | `&TextSubmitted` | Submit on Enter |

## Slots

There are no slots.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
