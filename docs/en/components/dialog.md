# Dialog

An overlay with a title, a body and a footer. When you only need confirm and cancel, use [ConfirmDialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/confirm-dialog.md).

::: info
The control table has no `<Dialog>` tag. Both authoring styles call `widget(Dialog::new(title))`, then add `.title_icon`, `.title_content`, `.body`, `.footer` and `.close_action` to a view that is already built.
:::

## Basic usage

`Dialog::new` takes the title. You can also chain `.description`, `.size` and `.close_policy`. `.initial_focus_on(entity_ref)` puts focus on a control already placed in a slot when the dialog opens.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::widget;
use nana_ui::runtime::Dialog;

widget(Dialog::new("Rename"))
    .body(view! {
        <Column gap=8>
            <TextInput placeholder="Name" v-model={name} />
        </Column>
    })
    .footer(view! {
        <Row gap=8>
            <Button @activate={cancel}>"Cancel"</Button>
            <Button @activate={save}>"Save"</Button>
        </Row>
    })
```

```rust rust
use nana_ui::runtime::view::{button, column, row, text_input, widget};
use nana_ui::runtime::Dialog;

widget(Dialog::new("Rename"))
    .body(
        column().gap(8).children((
            text_input().placeholder("Name").model(name),
        )),
    )
    .footer(
        row().gap(8).children((
            button("Cancel").on_activate(cancel),
            button("Save").on_activate(save),
        )),
    )
```

:::

Slots are built before the dialog. The dialog's own assembly places them below the header, in the footer, and in the close slot. Opening, closing, the Escape key and focus restore follow the overlay conventions; see the overlay section of [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md).

## Size

The dialog states its own width. `.size` takes `nana_ui::DialogSize`: five presets `Compact` 420, `Default` 520, `Medium` 600, `Wide` 680, `Workspace` 1080, or `DialogSize::Width(length)` for a width of your own. Lengths read as in CSS: `LengthSpec::Px(560.0)` is 560; `DialogSize::capped(560.0, 92.0)` is CSS's `min(560px, 92vw)`, yielding to the window when the window is narrow. `%` and `vw` / `vh` resolve against the scrim, and the scrim covers the window. Either way the card never exceeds the 16px margin the scrim leaves.

Where the card stands and how tall it may grow is the design system's business and belongs to the theme: `ThemeDefinition::with_dialog(DialogRecipe { top, max_height })`. The built-in theme is 90px from the top, at most 76% of the scrim's height. A design written as `margin-top: 12vh; max-height: 72vh` becomes `LengthSpec::Viewport { axis: ViewportAxis::Height, value: 12.0 }` and `value: 72.0`. When the card is too tall to fit, it moves up, but never past the margin. To center the card vertically in the scrim (the scrim is a column flex with `justify-content: center`), write `align: DialogAlign::Center`: the card sits as far from the scrim's top as from its bottom, grows up and down with its content, and `top` no longer applies; the built-in value is `DialogAlign::Top`. Installing a new recipe lays open dialogs out again, together with their slots.

```rust
use nana_ui::DialogSize;
use nana_ui::runtime::{Dialog, LengthSpec, ViewportAxis};
use nana_ui::theme::{DialogRecipe, ThemeDefinition};

let theme = ThemeDefinition::NANA_DARK.with_dialog(DialogRecipe {
    top: LengthSpec::Viewport { axis: ViewportAxis::Height, value: 12.0 },
    max_height: LengthSpec::Viewport { axis: ViewportAxis::Height, value: 72.0 },
});
let export = Dialog::new("Export library").size(DialogSize::capped(560.0, 92.0));
```

## Tone and the title icon

`.danger(true)` makes the dialog speak in a danger tone: the title takes the danger semantic color of the theme's status recipe (the built-in theme uses `Danger`), overriding the node's own text color. Body and description are unchanged. `danger` on [ConfirmDialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/confirm-dialog.md) is the same tone, except that the confirm button takes the danger color and the title changes color too. For a design that expresses danger with a title icon and a confirm button, write `danger_title: false` in the recipe: the title keeps the text color and the confirm button still takes the danger color.

`.title_icon(view)` puts an icon before the title. The header lays out as CSS `display: flex; align-items: center` would: `[icon] title [close slot]`, each taking one cell and vertically centered in the row, with the title starting after the icon and its wrap width giving up the icon and the close slot. The row height is the taller of the title block and the icon; the close slot does not stretch the row, so hiding the close slot on a busy confirm dialog does not move the body.

Sizes inside the row belong to the theme's `DialogRecipe`: `icon_size` (the icon cell, built in 16), `close_size` (the close cell, built in 28), `header_gap` (the gap between the three, built in 12) and `header_min_height` (the row's minimum height, built in 0). A design written as `.dialog-card__header { align-items: center; gap: 8px }` plus a 24px close button becomes `close_size: 24.0, header_gap: 8.0, header_min_height: 24.0`.

The title's type belongs to the recipe too: `title: RecipeText { size, weight }`, built in 14 / 600, which is CSS's `font-size` and `font-weight`; `weight: None` keeps the node's own weight. Measurement and painting read the same declaration, so the measured title is the painted title. The drawer keeps its own title.

`title_max_width: Option<LengthSpec>` limits how far the title block (title and description) may run: `%` resolves against the card width, `vw` / `vh` against the scrim, and `None` (the built-in) runs all the way to the close slot. Use it when the header stands on a shape narrower than the card — a card surface that paints its own tab taller than the card body, with the title staying inside the tab, for example: write `title_max_width: Some(LengthSpec::Percent(50.0))` and the title wraps or truncates at half the card width, while the accessible name stays the full title. It constrains only the title block: the body of a confirm dialog wraps and measures against the body area's width, unaffected by this limit.

:::api

```rust view
use nana_ui::icons_tabler::ALERT_TRIANGLE;
use nana_ui::runtime::view;
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{Dialog, IconGlyph};

widget(Dialog::new("Delete library").danger(true))
    .title_icon(view! { <Widget of={IconGlyph::new(ALERT_TRIANGLE)} /> })
    .body(view! { <Text>"Files in the library will not be deleted."</Text> })
```

```rust rust
use nana_ui::icons_tabler::ALERT_TRIANGLE;
use nana_ui::runtime::view::{text, widget};
use nana_ui::runtime::{Dialog, IconGlyph};

widget(Dialog::new("Delete library").danger(true))
    .title_icon(widget(IconGlyph::new(ALERT_TRIANGLE)))
    .body(text("Files in the library will not be deleted."))
```

:::

## Title content

When the header has to hold more than one line of text — a status badge after the name, or something else — use `.title_content(view)`. That view is ordinary layout; what goes in it and how it is arranged are its own business. It stands where the title text would: `[icon] [title content] [close slot]`, vertically centered in the row, as wide as the title text may run (giving up the icon and the close slot, and subject to the recipe's `title_max_width`), and as tall as it lays itself out. The row height is still the tallest of the title block, the icon and `header_min_height`, and the close slot does not stretch the row. With `.description`, the description follows the title content.

With title content, the title text is no longer painted, but the title passed to `Dialog::new` is still the dialog's accessible name and screen readers still call the dialog by it; children inside the title content are ordinary children of the dialog, each with its own accessibility semantics. The recipe's `title` type and the `danger` title color apply only to the title text and do not reach the title content, whose type the view decides. When the title content produces no box (`display: none`) or is removed, the header falls back to the title text and lays out as if none had been given. `cx.modal_sections()`'s `title` is then the title content's box (together with the description below it).

`Drawer` has `.title_content` too, laid out the same way. `ConfirmDialog` does not.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{Dialog, StatusTone};

widget(Dialog::new("Library"))
    .title_content(view! {
        <Row gap=8>
            <Text>"Library"</Text>
            <StatusBadge tone={StatusTone::Success}>"Synced"</StatusBadge>
        </Row>
    })
    .body(view! { <Text>"Last synced 5 minutes ago."</Text> })
```

```rust rust
use nana_ui::runtime::view::{row, status_badge, text, widget};
use nana_ui::runtime::{Dialog, StatusTone};

widget(Dialog::new("Library"))
    .title_content(row().gap(8).children((
        text("Library"),
        status_badge("Synced").tone(StatusTone::Success),
    )))
    .body(text("Last synced 5 minutes ago."))
```

:::

## Card sections

The card has three sections: header, body, footer. Each section's padding, the dividers below the header and above the footer, and the card's corner radius all belong to the theme's `DialogRecipe` and are not written on the dialog:

| Recipe field | Built-in value | CSS equivalent |
| --- | --- | --- |
| `header: DialogInsets { top, bottom, inline }` | 14 / 8 / 16 | The header's `padding` |
| `body: DialogInsets` | 8 / 10 / 16 | The body's `padding` |
| `body_bottom_alone: Option<f32>` | `Some(16.0)` | The body's bottom padding when there is no footer; `None` uses `body.bottom` as CSS would |
| `footer: DialogInsets` | 0 / 14 / 16 | The footer's `padding`; the footer's content box is one button tall |
| `action_gap` | 8 | The `gap` between footer buttons |
| `header_divider` / `footer_divider: Option<SemanticColorRole>` | `None` | The header's `border-bottom` and the footer's `border-top`; the thickness is the theme's `border.hairline` and counts toward the height of the section it belongs to |
| `divider_inset: DialogDividerInset` | `Edge` | The divider runs edge to edge (`Edge`), or insets into the horizontal padding of the section it belongs to (`Content`): the line below the header starts at the title text's start, like a hairline under the title |
| `radius: RadiusTier` | `Md` | The card's `border-radius` |

The `.footer` slot and a confirm dialog's buttons go into the footer's content box: inside the divider and the top and bottom padding, one button tall. The drawer does not read this recipe and keeps its own layout.

A design written as this CSS:

```css
.dialog-card__header { padding: 12px 14px; border-bottom: 1px solid var(--border-soft); }
.dialog-card__body { padding: 12px 14px; }
.dialog-card__actions { gap: 8px; padding: 10px 14px; border-top: 1px solid var(--border-soft); }
.modal-card { border-radius: var(--radius-xl); }
```

becomes this in the theme:

```rust
use nana_ui::runtime::SemanticColorRole;
use nana_ui::theme::{DialogInsets, DialogRecipe, RadiusTier, ThemeDefinition};

let theme = ThemeDefinition::NANA_DARK.with_dialog(DialogRecipe {
    radius: RadiusTier::Xl,
    header: DialogInsets::symmetric(12.0, 14.0),
    body: DialogInsets::symmetric(12.0, 14.0),
    body_bottom_alone: None,
    footer: DialogInsets::symmetric(10.0, 14.0),
    action_gap: 8.0,
    header_divider: Some(SemanticColorRole::BorderSoft),
    footer_divider: Some(SemanticColorRole::BorderSoft),
    ..DialogRecipe::DEFAULT
});
```

## Painting the card surface yourself

The card's position, sections and motion belong to the theme's recipe; its shape belongs to the theme's card (`Surface` background, `Overlay` shadow, recipe radius). When the application has its own card shape — a header standing on a tab taller than the card body, a slope down to the body, an accent-colored plate behind it — paint the card surface yourself with `.surface_painter(painter)`. Both `Dialog` and `ConfirmDialog` have it.

The painter is the same [`Painter`](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md) that attaches to nodes, except that its origin is the card's top-left corner and its size is the card's size. It paints above the scrim and below the card's title, body, dividers and slots, and fades and moves with the card. `cx.modal_sections()` gives each section's position in card-surface coordinates (`ModalSurfaceSections`):

| Field | What it is |
| --- | --- |
| `header` | The header section, the card's full width, from the top edge down to the divider (with a divider, the line sits directly below this section) |
| `title` | The title block (title and description), as wide as the recipe allows; with `.title_content`, the title content's box together with the description |
| `icon` / `close` | The title icon cell and the close slot, present only when used |
| `body` | The body's content box |
| `footer` | The footer section, the card's full width down to the bottom edge, present only with buttons or a footer slot |

Draw the shape against these boxes and it always stays under what they hold: when the title grows to two lines and the header gets taller, the tab grows with it.

Paint only the layer below the children: `paint_over_children` and the painter's own hit testing are not used, and the whole card takes presses that land on it under the modal card's rules. Call `cx.draw_default()` inside the painter and the theme's card is painted below what the painter draws. The recording is cached by the painter's `paint_key`, the card size, the section positions and the theme, and is not recorded again while all of them are unchanged. The drawer has no such entry.

```rust
use nana_ui::runtime::{ConfirmDialog, PaintContext, PaintPath, Painter, SemanticColorRole};
use nana_ui::theme::RadiusTier;

struct TabbedCard;

impl Painter for TabbedCard {
    fn paint(&self, cx: &mut PaintContext<'_>) {
        let [width, height] = cx.size();
        let rise = cx.modal_sections().map_or(0.0, |sections| sections.header.height);
        let radius = cx.radius(RadiusTier::Lg);
        let mut outline = PaintPath::new();
        // The tab takes half the card width and stands `rise` above the card body; the slope and the body follow the design.
        outline
            .move_to(0.0, height * 0.5)
            .arc_to(0.0, 0.0, width * 0.5, 0.0, radius)
            .arc_to(width * 0.5, 0.0, width * 0.5 + 30.0, rise, radius)
            .arc_to(width * 0.5 + 30.0, rise, width, rise, radius)
            .arc_to(width, rise, width, height, radius)
            .arc_to(width, height, 0.0, height, radius)
            .arc_to(0.0, height, 0.0, 0.0, radius)
            .close();
        cx.shadow(&outline, nana_ui::theme::ElevationRole::Overlay);
        cx.fill_path(&outline, SemanticColorRole::Surface);
    }

    fn paint_key(&self) -> u64 {
        0
    }
}

let dialog = ConfirmDialog::new("Delete scene", "Models in the scene are deleted too.")
    .danger(true)
    .surface_painter(TabbedCard);
```

## Scrim

The scrim behind a dialog or drawer belongs to the theme's effect tokens: `EffectTokens::modal_scrim` (color and opacity, built in black at 0.45) and `modal_scrim_blur` (how much the scrim blurs the window behind it, CSS `backdrop-filter: blur(r)`, where `r` is a Gaussian standard deviation in logical px; built in 0, no blur). The window behind is blurred first and the scrim color composited over it, the same order CSS applies an element's `backdrop-filter` and background color.

The painter composites in linear light: a black alpha leaves `1 - α` of the light behind it. A browser's `rgba(0, 0, 0, c)` blends encoded sRGB values, leaving `1 - c` of the encoded value, which is about `(1 - c)^2.2` of the light. So an opacity written for a CSS scrim has to be converted: `nana_ui::theme::linear_scrim_alpha(c)` is `1 - (1 - c)^2.2`, and CSS's 0.45 is about 0.73 here.

```rust
use nana_ui::theme::{EffectTokens, SemanticColor, ThemeDefinition, linear_scrim_alpha};

// .modal-overlay { background: rgba(0, 0, 0, .45); backdrop-filter: blur(2px) }
let mut effects = EffectTokens::DARK;
effects.modal_scrim = SemanticColor::rgba(0.0, 0.0, 0.0, linear_scrim_alpha(0.45));
effects.modal_scrim_blur = 2.0;
let theme = ThemeDefinition::NANA_DARK.with_effects(effects);
```

## Enter and exit

How a dialog comes and goes belongs to the theme's `DialogRecipe::motion` (`DialogMotion`): the scrim fades in (`scrim`), the card fades in (`card_fade`), and the card moves back into place from `enter_offset_y` (px, positive downward) and `enter_scale` (`card_move`). Each is a `DialogTransition { millis, easing }`, which is CSS's `transition: <property> <duration> <easing>`. Translation and scale pivot on the card's own center, as CSS `transform` on the card would, and slot content fades and moves with the card. The scrim does not follow the card's fade: it fades in over its own duration, and its blur fades in with it, the way CSS `opacity` also applies to `backdrop-filter`. Exit plays the same transitions in reverse.

The built-in is a 140ms ease-out fade with a card that does not move. A design written as this CSS:

```css
.modal-fade-enter-active { transition: opacity .16s ease }
.modal-fade-enter-active .modal-card {
  transition: transform .18s cubic-bezier(.2, .8, .2, 1), opacity .16s ease;
}
.modal-fade-enter-from .modal-card { transform: translateY(-8px) scale(.98) }
```

becomes:

```rust
use nana_ui::runtime::Easing;
use nana_ui::theme::{DialogMotion, DialogRecipe, DialogTransition, ThemeDefinition};

let ease = Easing::CubicBezier([0.25, 0.1, 0.25, 1.0]); // CSS's `ease`
let theme = ThemeDefinition::NANA_DARK.with_dialog(DialogRecipe {
    motion: DialogMotion {
        scrim: DialogTransition::new(160, ease),
        card_fade: DialogTransition::new(160, ease),
        card_move: DialogTransition::new(180, Easing::MENU_POP),
        enter_offset_y: -8.0,
        enter_scale: 0.98,
    },
    ..DialogRecipe::DEFAULT
});
```

A theme is rejected when a duration is 0, an offset or scale is not finite, or a scale is not positive. When the application or the system asks for reduced motion (`AppContext::set_reduced_motion`, or the system preference the host reports), dialogs, drawers and menus play no transition: they appear and disappear at once, and open and close events are still emitted. The drawer does not read this recipe and keeps the built-in fade.

## Declarative open and close

When the dialog is under an `OverlayHost`, `.open(..)` is the declarative form of `activate_overlay` / `dismiss_overlay`. Write `true` and it opens once it, its host and its slots are in the tree (the slots are placed by assembly, so this waits for assembly); write `false` and it closes, exit animation included. When the block it lives in is parked (a `keep_alive` branch switched away), the host releases it as usual and `open` becomes `false` with it; when the block comes back, the dialog stays closed and needs `true` written again to open. `ConfirmDialog` and `Drawer` behave the same way.

When the host opens or closes it, `open` follows and `DialogToggled { open }` is emitted on the dialog itself: a close gesture the policy allows, `dismiss_overlay`, `activate_overlay`, another overlay on the same host taking its place, or the dialog being parked. Writing `open` yourself emits nothing. `.model(signal)` is two-way: the signal opens and closes it, and the host's open and close write back to the signal. With `.open(signal)` alone, after the framework closes the dialog you have to write your own state back to `false` in `DialogToggled`, otherwise the signal stays `true`, the next write of `true` is not a change, and the dialog will not open.

```rust
use nana_ui::runtime::view::{signal, text, widget};
use nana_ui::runtime::{Dialog, OverlayHost};

let exporting = signal(false);
widget(OverlayHost::new()).children((
    widget(Dialog::new("Export")).body(text("…")).model(exporting),
))
// exporting.set(true) opens; after the user closes with Escape, exporting turns itself back to false.
```

Keep the dialog under the host and drive it with `open`, so the exit animation can finish. Put it in `when(..)` and remove the node on close and there is no exit.

## Letting the application decide open state

There are three gestures for closing a dialog: pressing Escape, pressing and releasing outside the dialog, and activating the close slot (`.close_action`). Each first emits `DialogCloseRequested` on the dialog itself, with `trigger` naming which one. Only then does `.close_policy` apply: if the gesture is allowed, the framework closes the dialog and the host then emits `OverlayClosing`; if not, it stays. Escape and outside presses reach only dialogs that are under an `OverlayHost` and opened with `activate_overlay` (the overlay conventions); the close slot emits the request whether or not it is placed.

`DialogClosePolicy::requests_only()` lets the framework close on no gesture at all. The application decides everything: to close in the handler, change the application's own open state (the dialog unmounts with the view), or call `dismiss_overlay(host)` to play the exit animation; to keep it open, do nothing — ask about unsaved changes first, for example.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{Dialog, DialogCloseRequested};
use nana_ui::DialogClosePolicy;

widget(Dialog::new("Rename").close_policy(DialogClosePolicy::requests_only()))
    .on(move |_: &DialogCloseRequested| {
        if !dirty.get() {
            open.set(false);
        }
    })
    .body(view! {
        <TextInput placeholder="Name" v-model={name} />
    })
```

```rust rust
use nana_ui::runtime::view::{text_input, widget};
use nana_ui::runtime::{Dialog, DialogCloseRequested};
use nana_ui::DialogClosePolicy;

widget(Dialog::new("Rename").close_policy(DialogClosePolicy::requests_only()))
    .on(move |_: &DialogCloseRequested| {
        if !dirty.get() {
            open.set(false);
        }
    })
    .body(text_input().placeholder("Name").model(name))
```

:::

Gestures the policy did not allow used to be swallowed silently: Escape no longer reached `on_key` and an outside press produced no event, so the application could only check afterwards whether the dialog was still there. Now there is no need to poll; listen for this request.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| Title | Argument to `Dialog::new` | Given at construction |
| `.description` | Text | Description below the title |
| `.size` | `nana_ui::DialogSize` | Card width: five presets, or `Width(length)` / `capped(px, vw)`. Top offset and maximum height belong to the theme's `DialogRecipe` |
| `.danger` | `bool` | Danger tone: the title takes the theme's danger semantic color |
| `.close_policy` | Close policy | Which gestures the framework closes on. `DialogClosePolicy::requests_only()` closes on none and leaves everything to the application |
| `.initial_focus_on` | `entity_ref` | Puts focus on a control already placed in a slot when the dialog opens |
| `.open` | `bool` or a signal | Opens or closes it while it is under an `OverlayHost`; see [declarative open and close](#declarative-open-and-close) |
| `.model` | `Signal<bool>` | Two-way `open`: the host's open and close write back to the signal |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `.on(\|e: &DialogCloseRequested\| …)`, template `on:DialogCloseRequested` | `&DialogCloseRequested` | The user pressed Escape, pressed outside, or activated the close slot; `trigger` is `Escape` / `Outside` / `CloseButton`. Emitted on the dialog itself, before `close_policy` |
| `.on(\|e: &DialogToggled\| …)` | `&DialogToggled` | The host opened or closed it; `open` is the new state. Not emitted when the application writes `open` itself |

Opening, closing and focus restore follow the overlay conventions. When the framework closes the dialog, the host emits `OverlayClosing { root }`.

## Slots

| Slot | Description |
| --- | --- |
| `.title_icon` | Icon before the title, vertically centered in the header with the title and the close slot |
| `.title_content` | A view that stands in the header in place of the title text; see [title content](#title-content). The title is still the accessible name |
| `.body` | Content below the header. Build it before the call |
| `.footer` | Footer, usually a row of buttons |
| `.close_action` | The close slot |

The confirm dialog is `ConfirmDialog`. Its slots are `.title_icon`, `.body`, `.footer`, `.close_action`, `.cancel`, `.secondary` and `.confirm`, and `.footer` stands at the left end of the action row. Buttons you do not provide are generated from `cancel_label` or `confirm_label`.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [ConfirmDialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/confirm-dialog.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
