//! A tree as deep as the world allows goes through every per-frame pass on a
//! small stack: mount, layout, extraction, hit testing, accessibility, an
//! update at the bottom and teardown. See
//! `docs/reference/resource-budgets.md`.

use nana_ui_core::{LayoutStyle, LengthSpec, OverflowSpec};
use nana_ui_runtime::view::{IntoView, button, signal, text, widget};
use nana_ui_runtime::{
    DocumentId, LayoutViewport, MAX_TREE_DEPTH, StableNodeId, Stack, TextContent, TextMetrics,
    TextShaper,
};
use nana_ui_scene::RuntimeDocument;

/// The stack a whole frame gets, in either build: half a macOS or iOS
/// secondary thread's 512 KiB. A debug frame needs more than a release one
/// (unoptimized code keeps a slot for every local); it fits because the
/// per-frame `match`es run their large arms in frames of their own
/// (`own_frame`) and placement cuts a boundary every 8 levels there.
const FRAME_STACK: usize = 256 << 10;

struct TestShaper;

impl TextShaper for TestShaper {
    fn shape(
        &mut self,
        _id: StableNodeId,
        text: &TextContent,
        _style: &nana_ui_runtime::ComputedStyle,
        _constraints: nana_ui_runtime::TextShapeConstraints,
    ) -> TextMetrics {
        TextMetrics {
            width: text.value.len() as f32 * 8.0,
            height: 18.0,
            ascent: None,
        }
    }
}

/// One level of the tree: the containers a page nests, in turn.
fn level(depth: usize) -> Stack {
    let stack = Stack::column(0.0).with_layout(|layout| {
        *layout = LayoutStyle {
            padding: Some(LengthSpec::Px(0.25)),
            ..Default::default()
        }
    });
    match depth % 4 {
        0 => stack,
        1 => stack.with_layout(|layout| layout.opacity = Some(0.99)),
        2 => stack.with_layout(|layout| layout.overflow_y = OverflowSpec::Hidden),
        _ => stack.with_layout(|layout| layout.z_index = Some(1)),
    }
}

fn run(depth: usize) {
    let document = DocumentId::new(1).unwrap();
    let mut runtime = RuntimeDocument::new(document);
    let label = signal(String::from("deepest"));
    runtime
        .context_mut()
        .mount_view_root(document, move || {
            let mut view = (button(label), text("leaf")).into_any();
            for depth in 0..depth {
                view = widget(level(depth)).children((view,)).into_any();
            }
            view
        })
        .unwrap();
    let viewport = LayoutViewport::new(800.0, 600.0);
    runtime.flush(viewport, &mut TestShaper).unwrap();
    assert!(runtime.scene().primitives().next().is_some());
    let world = runtime.context().world();
    let inset = 2.0 + depth as f32 * 0.25;
    assert!(world.hit_test(document, inset, inset).is_some());
    assert!(world.project_accessibility(document).len() > 1);

    label.set(String::from("changed at the bottom"));
    runtime.flush(viewport, &mut TestShaper).unwrap();
    drop(runtime);
}

#[test]
fn a_tree_as_deep_as_allowed_runs_every_frame_pass_on_a_small_stack() {
    // The view adds the document root, the button and its label below the
    // levels built here.
    let depth = MAX_TREE_DEPTH - 4;
    std::thread::Builder::new()
        .stack_size(FRAME_STACK)
        .spawn(move || run(depth))
        .unwrap()
        .join()
        .expect("a frame pass ran out of stack");
}
