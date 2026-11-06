# Extension controls

Most UIs are fine built from existing controls. Add a new kind only when it takes part in layout, clicks, and painting, and existing controls cannot express it. Built-in controls and yours go through the same `ComponentRegistry`.

## Registration

The stable identity is `ComponentTypeId`, for example `nana.button` or `app.preview-card`. The code below registers a card; `AppContext::install` is called before tree building.

```rust
use nana_ui::runtime::{
    AppContext, ComponentView, ExtensionRegistrar, FrameworkError, MutationQueue, NodeKind,
    RegisterableComponent, SemanticSpec, StableNodeId, TextContent, UiExtension, UiWorld,
};

#[derive(Clone, PartialEq)]
struct PreviewCard {
    title: String,
}

impl ComponentView for PreviewCard {
    fn node_kind(&self) -> NodeKind {
        NodeKind::Element {
            tag: "preview-card".into(),
        }
    }

    fn project(&self, id: StableNodeId, world: &UiWorld, mutations: &mut MutationQueue) {
        if world.text(id) != Some(self.title.as_str()) {
            mutations.set_text(id, TextContent::new(self.title.clone()));
        }
    }
}

impl RegisterableComponent for PreviewCard {
    const TYPE_ID: &'static str = "app.preview-card";
    const TAGS: &'static [&'static str] = &["preview-card"];

    fn from_semantic(spec: &SemanticSpec<'_>) -> Self {
        Self {
            title: spec.display_label().to_owned(),
        }
    }
}

struct PreviewCards;

impl UiExtension for PreviewCards {
    fn name(&self) -> &'static str {
        "preview-cards"
    }

    fn install(&self, registrar: &mut ExtensionRegistrar) -> Result<(), FrameworkError> {
        registrar.register_component::<PreviewCard>()
    }
}

fn install(cx: &mut AppContext) -> Result<(), FrameworkError> {
    cx.install(&PreviewCards)
}
```

`ComponentView` requires `Clone + PartialEq + Send`. Every field `project` reads must take part in the equality comparison: equal writes are skipped. If `project` reads shared interior-mutable state, or patches another component's nodes, add `const ALWAYS_REPROJECT: bool = true;`.

`RegisterableComponent` provides the type id, the tags, and `from_semantic`, which constructs from a `SemanticSpec`. Call `ExtensionRegistrar::register_component` in `install`. Use `register_component_alias` for extra tags. Use `register_activation` for pointer and keyboard activation; do not modify `activate_node`.

Rust builds nodes by type with `create_component`. Construction by tag resolves against the same table. The tag equals the `ComponentTypeId` with the `nana.` prefix removed: `nana.preview-card` corresponds to `preview-card`. When the semantics match HTML, use the native tag (`button`, `table` / `tr` / `td`). When the semantics differ, use another name: `search-dropdown`, not HTML `<search>`. A tag that is neither registered nor known HTML is an error; it is not treated as a layout box.

## Event list and GPU slots

The authority for which names are listened to is the `EventListeners` on `UiWorld`; no separate copy is kept outside the control. Write it during projection:

```rust
mutations.set_event_listener(id, "activate", true);
```

A GPU slot is a `CustomRenderNode` on the node, written in the same commit. `renderer` and `resource` are opaque keys; the fields hold no GPU backend objects:

```rust
mutations.set_custom_render(
    id,
    Some(nana_ui::runtime::CustomRenderNode::new(renderer, resource, revision)),
);
```

Ordinary live GPU content still uses `GpuTextureView` with the same string slot. Slots on that path also land on Runtime's `CustomRenderNode`. See [Live GPU content](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/gpu.md).

## Limitations

The `dist` profile used for release is `panic = "abort"`. A panic in a factory terminates the process outright; do not treat it as a recoverable error. Building controls as dynamically loaded dylibs is not supported either.

## Binders you do not use need not be linked

`AppContext::new()` registers the by-tag constructors of all built-in controls. When you only build trees from Rust types, `view!`, or `.vue`, use the typed mode:

```rust
impl nana_ui::ApplicationState for App {
    const BUILTINS: nana_ui::runtime::BuiltinComponents =
        nana_ui::runtime::BuiltinComponents::Typed;
}
```

Without a host, use `AppContext::typed()` and `RuntimeDocument::typed(document)`. The mode must be a compile-time constant; choosing at runtime links both sets in. In typed mode, constructing a built-in control by tag returns `FrameworkError::InvalidComponentType`.

After release and strip: the minimal headless program goes from 5.40 MB to 2.28 MB; the hosted window `examples/reactive-sfc` goes from 35.48 MB to 33.42 MB. Most of the hosted size is GPU, text, and built-in fonts.

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/components/props">
    <p class="next-step-link">Props</p>
    <p class="next-step-caption">Three kinds of value: constants, signals, and closures.</p>
  </a>
  <a class="next-step" href="/guide/components/events">
    <p class="next-step-link">Events</p>
    <p class="next-step-caption">What controls emit, and how to catch it on the view.</p>
  </a>
  <a class="next-step" href="/reference/components">
    <p class="next-step-link">Controls reference</p>
    <p class="next-step-caption">The registration contract and the semantics of existing controls.</p>
  </a>
</div>
