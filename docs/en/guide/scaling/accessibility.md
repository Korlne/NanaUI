# Accessibility

Screen readers must be able to announce a control's name. An input's name comes from `label`; placeholder text does not count.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <TextInput label="Name" placeholder="What should we call you" v-model={name} />
}
```

```rust rust
use nana_ui::runtime::view::text_input;

text_input().label("Name").placeholder("What should we call you").model(name)
```

:::

When a template is compiled, a control that a screen reader cannot name gets a warning; compilation still succeeds. Two rules: `Button`, `Checkbox`, `Switch` and `ListItem` have no text; `TextInput`, `TextArea`, `NumberInput`, `Slider` and `Progress` have no `label`. `.vue` warnings come through `cargo:warning` with line and column. On stable, `view!` reports them at the macro call as "use of deprecated constant", with the explanation in the deprecation note.

## Row labels

A settings row builds its own label. When the control has no name, the screen reader reads the row's label. At assembly time, `MutationQueue::set_labelled_by` links the control to the label node. The link lives in `UiWorld`, so the control re-projecting its own state does not wipe it. When either end is destroyed, the link goes with it.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::settings_row;

settings_row("Mute").control(view! {
    <Switch></Switch>
})
```

```rust rust
use nana_ui::runtime::view::{settings_row, switch};

settings_row("Mute").control(switch(""))
```

:::

A control with its own non-empty name uses it, for example `switch("Mute")`. Only an empty name (`switch("")`, sliders, dropdown selects) falls back to the row label. When the row label changes, the control's accessibility node is re-projected. Once a `Select` is named this way, its accessible name is not the currently displayed option; the option is its value. When nobody names it, it is still named by the displayed text.

A `Switch` does not set `role_description` by default, so the platform accessibility layer announces "switch" (or its equivalent) in the system language (details in [Framework strings](../../reference/framework-strings.md#how-switches-are-announced)).

`settings_row` has no template tag. Which row in a group is first or last is computed by you from the data and then bound.

## Virtual children and region names

For self-drawn single-node controls such as `TreeView` and open menus, the framework projects each visible row as a virtual child in the accessibility tree (for example a `TreeItem` under `AccessibilityRole::Tree`, with `level`, `expanded`, `selected`, `disabled` and the row's rectangle bounds; collapsed child rows are removed from the accessibility tree). Screen reader `Click` / `Focus` / `Expand` / `Collapse` go through the same state update as pointer and keyboard (see [TreeView](../../components/tree-view.md#accessibility)). The menu opened by Select, Dropdown or SearchDropdown is a `ListBox` under the field (`ComboBox`, with `expanded` and an `active_descendant` pointing at the highlighted item), with one `ListBoxOption` per exposed row, carrying `selected`, `disabled`, `position_in_set` / `set_size` (see [Select screen reader support](../../components/select.md#screen-reader)).

Each area in `AreaLayout` is a `Region` in the accessibility tree. Its name is supplied by `.labels([(editor, name)])`, or by `AppContext::set_area_labels` when the language changes, and does not pollute the serializable layout model (see [AreaLayout](../../components/area-layout.md)).

## Announcing changes

A stack of toasts, or a progress bar that advances on its own, must be read out when it changes; this is `aria-live`. The composite control that holds them sets it: `MutationQueue::set_live(node, Some(AccessibilityLive::Polite))`. `Polite` waits until the user is idle, `Assertive` interrupts whatever is being read, `Off` does not announce; `None` removes the setting and returns to the platform's default for the role. Like row labels, the setting lives in `UiWorld`, not in the node's own `AccessibilityState`, so the node re-projecting its own state does not wipe it, and it is removed when the node is destroyed. A container with `visibility: hidden` is not a live region, as on the web; once shown, it is again. The AccessKit host projects it as the node's `live`.

## Is hidden content still in the tree

`UiWorld::project_accessibility` keeps the structural ancestors needed to connect visible controls. A container with `visibility: hidden` outputs a neutral Generic: its own label, value, description, original role and interaction state are not exposed. A child with explicit `visibility: visible` keeps a full parent chain and its semantics. Hidden leaves, and subtrees that produce no layout box, are not projected. Once visible again, the restored semantics come from the current application state.

A node's `bounds` are in its parent's space. A rotated or scaled node also carries `transform` (node-to-parent is `translate(bounds.x, bounds.y) · transform`; a positive scale is `diag(a, d)`; a page root, `fixed`, `sticky`, or escaping absolute position also carries it when rotated or scaled). `clip` is in viewport coordinates. For screen coordinates, use `AccessibilityPlacements` when you hold the relative tree, or `project_accessibility_in_viewport` straight from the world (see [Application API](../../reference/application-api.md#accessibility-node-coordinates)).

An embedded host with `accesskit-tree` enabled consumes the full tree or an `AccessibilityDelta` through `AccessTreeProjector`. A hidden structural container is a GenericContainer with no click or focus actions; its children keep their actions and focus. `AccessTreeProjector::new` only sets up the retained state. Call `full_update()` the first time you need the full tree.

The accessible content of a static `Text` maps to AccessKit's `Label.value`, so the text appears in the UIA Name. For ordinary controls, label and value stay separate. For editing controls, the name and the input value are not mixed.

## Window root and publish timing

The hosted adapter has a separate, stable Window accessibility root. Mounting, replacing or clearing the document, and focus entering an ordinary control, never replace this root's identity. Runtime nodes keep their own roles and stable ids. When no control has focus, native focus returns to the window root.

Changes are staged after layout converges and published after a successful present, before the application's `presented` and window binding callbacks. A batch of changes that has not been presented stays as a delta. When several batches have accumulated, recovery takes a full snapshot of the current document, so nodes removed and re-attached in between are not stitched together wrongly. An idle retry does not overwrite pending changes. Each window has its own queue; closing one window does not change the main window's queue.

A newly created main or auxiliary window starts with only the Window root. Content is published only after the first successful present. Calling `flush` before mount does not push controls that have not been presented into the native tree. The first partial delta at cold start is supplemented with one snapshot of the current document.

The projection contract is in [Application API](../../reference/application-api.md#accessibility-semantics-in-hidden-hierarchies), and frame ordering is in [How it works](../../reference/how-it-works.md).

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/scaling/fetch">
    <p class="next-step-link">Fetch</p>
    <p class="next-step-caption">The allowlist shared by page requests and remote images.</p>
  </a>
  <a class="next-step" href="/guide/components/props">
    <p class="next-step-link">Props</p>
    <p class="next-step-caption">How fields such as label are bound.</p>
  </a>
  <a class="next-step" href="/guide/scaling/performance">
    <p class="next-step-link">Performance</p>
    <p class="next-step-caption">Accessibility changes are published after present.</p>
  </a>
</div>
