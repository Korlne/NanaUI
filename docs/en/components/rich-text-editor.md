# RichTextEditor

`RichTextEditor` edits a `RichText` in place: what you see is what you get. It is the same kind of text node as [RichTextView](rich-text-view.md), goes through the same `SetRichText` path, and uses the same retained layout. For the same document at the same width, both break lines identically. The layouts do not merely happen to agree; they are the same layout.

The document belongs to the application. Every change emits `RichTextEditorEvent::Changed(value)`; the application stores that value and hands it back unchanged on the next rebuild. Handing back an equal value commits nothing. The control holds only editing state the application should not manage: the selection, the IME preedit, the style for the next character, and the undo history.

There is no `<RichTextEditor>` in the control table. The tag `rich-text-editor` only creates a plain-text initial value.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::RichTextEditor;
use nana_ui::runtime::rich::RichText;

view! {
    <Widget of={RichTextEditor::new(RichText::new("Say something")).font_size(20.0)} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::RichTextEditor;
use nana_ui::runtime::rich::RichText;

widget(RichTextEditor::new(RichText::new("Say something")).font_size(20.0))
```

:::

Give the editor the same font size, font family, and width as the view, and both lay out text the same way.

## Toolbar

A toolbar does not change the document directly. It sends commands to the editor: `cx.rich_edit(entity, command)`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{RichEditCommand, RichTextEditor};
use nana_ui::runtime::rich::{RichObject, RichSpanStyle, RichText};

let editor = cx.create_component(document, RichTextEditor::new(RichText::new("")))?;
cx.rich_edit(editor, RichEditCommand::SetAttrs(RichSpanStyle::new().bold()))?;
cx.rich_edit(editor, RichEditCommand::InsertObject(RichObject::chip(7, "Wait 500ms", 1)))?;

view! {
    <Widget of={RichTextEditor::new(RichText::new(""))} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{RichEditCommand, RichTextEditor};
use nana_ui::runtime::rich::{RichObject, RichSpanStyle, RichText};

let editor = cx.create_component(document, RichTextEditor::new(RichText::new("")))?;
cx.rich_edit(editor, RichEditCommand::SetAttrs(RichSpanStyle::new().bold()))?;
cx.rich_edit(editor, RichEditCommand::InsertObject(RichObject::chip(7, "Wait 500ms", 1)))?;

widget(RichTextEditor::new(RichText::new("")))
```

:::

| `RichEditCommand` | What it does |
| --- | --- |
| `SetAttrs(style)` | Applies `style` field by field over the selection. With no selection, it becomes the style of the next typed character |
| `ClearFormatting` | Removes all spans over the selection; with no selection, the next character has no style |
| `InsertText(text)` | Replaces the selection with `text`, styled the same as typing |
| `InsertObject(object)` | Replaces the selection with an inline object: a sticker, an image, or a zero-width marker chip |
| `SetObject { offset, object }` | Replaces the object at `offset` |
| `SetRuby(Some(text))` / `SetRuby(None)` | Adds ruby to the selection (replacing any ruby it touches) / removes the ruby the selection touches. Horizontal text only |
| `Select(range)` / `SelectAll` | Changes the selection (snapped to character boundaries) |
| `Undo` / `Redo` | Undo, redo |

Toolbar pressed states read `SelectionChanged { selection, attrs }`. `attrs` is tri-state: a value in `attrs.style` with no matching bit in `attrs.mixed` means the whole selection has that value; no value and not mixed means the whole selection leaves it unset; a set bit in `mixed` means the selection is inconsistent. `RichAttrMask` has one bit per field; font width is `STRETCH` and background color is `BACKGROUND_COLOR`. `ClearAttrs(mask)` removes only those fields.

## Input

- Typing inherits the style of the character before the caret; a style given to the caret by `SetAttrs` takes precedence. Consecutive typing merges into one undo step. `typing_style_policy(RichTypingStylePolicy::Plain)` stops typing, preedit, and plain-text paste from inheriting neighboring characters and uses only the node style; a caret style from `SetAttrs` still applies, and the toolbar at the caret shows the style that will be typed. The default is `Inherit`. Rich-text paste keeps the copied content's own styles under both policies.
- The IME preedit is drawn in the style it will be committed with, underlined. It is not part of the document until it is committed. The candidate window anchors to the editor's caret, even in an empty document.
- While focused, the caret blinks (500 ms shown, 500 ms hidden); every change to the document, preedit, or caret position shows it immediately and restarts the timer (including a backward delete that does not move the caret), matching `TextInput`. When the window loses focus the caret stays solid.
- Arrow keys move by grapheme, with Ctrl by word; Up and Down move by line; Home / End go to the start and end of the line, with Ctrl to the start and end of the document; Shift extends the selection.
- Ctrl+C / X write plain text to the system clipboard and also remember, in process, the most recently copied rich text along with the clipboard change number after the write. On paste, styles and objects come back only if the clipboard still holds that write (same change number, same text); the same text written by another program or another input field pastes as plain text. When the host has no change number (Linux), matching falls back to the text. When the clipboard holds no text, `PasteRequested` is emitted, and the application can insert an image from its own clipboard format.
- Edit menus and context menus use `cx.rich_clipboard(entity, RichClipboardCommand, services)`: `Copy`, `Cut`, and `Paste` match the shortcuts, and `PastePlainText` pastes as plain text even when the clipboard holds the editor's own copy. A read-only editor can only `Copy`; nothing runs during preedit; copying with no selection leaves the clipboard unchanged, and cut does not delete text when the write fails.
- Enter inserts a line break. For Enter-to-send, intercept it first in the application's key policy (which sees keys before the editor).

## Inline objects

Stickers and images wrap together with the text. A marker chip (`RichObject::chip`) has zero width and is drawn as a thin marker only in the editor; the view does not draw it, so both break lines the same way. `RichObject::label(id, "@name", RichLabelStyle { fill, text, scale, icon, .. })` is a label colored by the application (for example a person's name with an avatar). The editor and the view draw it identically at the same width, and selection and deletion treat it as one character. Clicking an object emits `ObjectActivated { id, offset }`; an edit that deletes an object emits `ObjectRemoved { id, offset }`.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `value` | `RichText` | The document. Every change is reported by `Changed` |
| `read_only` | `bool` | Can be selected and copied, not changed |
| `disabled` | `bool` | Cannot take focus |
| `typing_style_policy` | `RichTypingStylePolicy` | Whether typing inherits the style of neighboring text; default `Inherit` |
| `font_size` / `font_family` / `line_height` / `width` | | Same as [RichTextView](rich-text-view.md) |

## Events

| `RichTextEditorEvent` | When |
| --- | --- |
| `Changed(RichText)` | The document changed |
| `SelectionChanged { selection, attrs }` | The selection or the style at the selection changed |
| `ObjectActivated { id, offset }` | An inline object was clicked |
| `ObjectRemoved { id, offset }` | An edit deleted an inline object |
| `PasteRequested` | Paste found no text on the clipboard |

## See also

[RichTextView](rich-text-view.md) · [Overview](index.md) · [Text engine](../reference/text-engine.md#rich-text-editor)
