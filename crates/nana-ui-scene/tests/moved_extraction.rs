//! A node layout only moved reuses the record the scene holds for it
//! (`UiWorld::extract_nodes_reusing`). Whatever else changed in the same
//! frame -- its colour, its text, the theme -- must still reach the scene:
//! after every frame, each record the scene holds is what a cold
//! extraction gives.

use std::sync::Arc;

use nana_ui_core::{FlexDirection, LayoutStyle, LengthSpec, SemanticColorRole, ThemeAppearance};
use nana_ui_runtime::{
    DocumentId, LayoutViewport, MeasureTextShaper, MutationQueue, NodeKind, NodeStyle,
    StableNodeId, TextContent,
};
use nana_ui_scene::{RuntimeDocument, UiScene};

const ROWS: usize = 8;
const ROOT: u64 = 1;
const COLUMN: u64 = 2;

fn id(raw: u64) -> StableNodeId {
    StableNodeId::new(raw).unwrap()
}

fn row_id(row: usize) -> StableNodeId {
    id(10 + row as u64 * 3)
}

fn label_id(row: usize) -> StableNodeId {
    id(11 + row as u64 * 3)
}

fn chip_id(row: usize) -> StableNodeId {
    id(12 + row as u64 * 3)
}

fn column_style(padding: f32) -> NodeStyle {
    NodeStyle {
        layout: Arc::new(LayoutStyle {
            width: Some(LengthSpec::Px(300.0)),
            height: Some(LengthSpec::Fill),
            direction: Some(FlexDirection::Column),
            padding_top: Some(LengthSpec::Px(padding)),
            ..LayoutStyle::default()
        }),
        ..NodeStyle::default()
    }
}

fn label_style(foreground: SemanticColorRole) -> NodeStyle {
    NodeStyle {
        foreground: Some(foreground),
        ..NodeStyle::default()
    }
}

fn chip_style(background: SemanticColorRole) -> NodeStyle {
    NodeStyle {
        layout: Arc::new(LayoutStyle {
            width: Some(LengthSpec::Px(12.0)),
            height: Some(LengthSpec::Px(12.0)),
            ..LayoutStyle::default()
        }),
        background: Some(background),
        ..NodeStyle::default()
    }
}

/// Root → column → rows × (row → label, chip). Moving the column's
/// padding moves every row, and every label and chip with its row.
fn document() -> RuntimeDocument {
    let document = DocumentId::new(1).unwrap();
    let mut runtime = RuntimeDocument::new(document);
    let mut queue = MutationQueue::new();
    queue.create(id(ROOT), document, NodeKind::Document);
    queue.create(
        id(COLUMN),
        document,
        NodeKind::Element { tag: "div".into() },
    );
    queue.insert(id(ROOT), id(COLUMN), None);
    queue.set_style(id(COLUMN), column_style(0.0));
    for row in 0..ROWS {
        queue.create(
            row_id(row),
            document,
            NodeKind::Element { tag: "div".into() },
        );
        queue.create(label_id(row), document, NodeKind::Text);
        queue.create(
            chip_id(row),
            document,
            NodeKind::Element { tag: "div".into() },
        );
        queue.insert(id(COLUMN), row_id(row), None);
        queue.insert(row_id(row), label_id(row), None);
        queue.insert(row_id(row), chip_id(row), None);
        queue.set_text(
            label_id(row),
            TextContent {
                value: format!("row {row}").into(),
            },
        );
        queue.set_style(
            row_id(row),
            NodeStyle {
                layout: Arc::new(LayoutStyle {
                    width: Some(LengthSpec::Px(300.0)),
                    height: Some(LengthSpec::Px(20.0)),
                    direction: Some(FlexDirection::Row),
                    ..LayoutStyle::default()
                }),
                ..NodeStyle::default()
            },
        );
        queue.set_style(label_id(row), label_style(SemanticColorRole::Text));
        queue.set_style(chip_id(row), chip_style(SemanticColorRole::Accent));
    }
    runtime.context_mut().commit_mutations(queue).unwrap();
    runtime
}

fn flush(runtime: &mut RuntimeDocument) {
    runtime
        .flush(LayoutViewport::new(300.0, 800.0), &mut MeasureTextShaper)
        .unwrap();
}

/// Every record the scene holds is a cold extraction of its node, and it
/// paints what a scene built from a cold extraction paints.
fn assert_scene_is_cold(runtime: &RuntimeDocument, frame: &str) {
    let world = runtime.context().world();
    let document = DocumentId::new(1).unwrap();
    let mut cold = UiScene::new();
    cold.apply_delta(world.extract_document(document), []);
    assert!(
        runtime.scene().primitives().eq(cold.primitives()),
        "{frame}: the scene paints other than a cold one"
    );
    for node in world.document_order(document) {
        let cold = world.extract_nodes(&[node]);
        assert_eq!(
            runtime.scene().extracted_node(node),
            cold.first(),
            "{frame}: the scene's record for {node:?} is not a cold extraction"
        );
    }
}

#[test]
fn a_moved_node_changed_in_the_same_frame_reaches_the_scene() {
    type Edit = (&'static str, fn(&mut MutationQueue, bool));
    let edits: [Edit; 5] = [
        ("move only", |_, _| {}),
        ("label colour", |queue, on| {
            let role = if on {
                SemanticColorRole::Accent
            } else {
                SemanticColorRole::Text
            };
            queue.set_style(label_id(2), label_style(role));
        }),
        ("chip fill", |queue, on| {
            let role = if on {
                SemanticColorRole::Danger
            } else {
                SemanticColorRole::Accent
            };
            queue.set_style(chip_id(5), chip_style(role));
        }),
        ("same-length text", |queue, on| {
            let value = if on { "wor 3" } else { "row 3" };
            queue.set_text(
                label_id(3),
                TextContent {
                    value: value.into(),
                },
            );
        }),
        ("theme", |queue, on| {
            queue.set_preset_theme(if on {
                ThemeAppearance::Dark
            } else {
                ThemeAppearance::Light
            });
        }),
    ];
    let mut runtime = document();
    flush(&mut runtime);
    assert_scene_is_cold(&runtime, "first frame");
    let mut padding = 0.0;
    for (name, edit) in edits {
        for on in [true, false] {
            padding += 3.0;
            let mut queue = MutationQueue::new();
            queue.set_style(id(COLUMN), column_style(padding));
            edit(&mut queue, on);
            runtime.context_mut().commit_mutations(queue).unwrap();
            flush(&mut runtime);
            assert_scene_is_cold(&runtime, name);
        }
    }
}

/// A button below a list moves when a row is inserted above it: its label
/// must move with its plate.
#[test]
fn a_button_moved_by_an_inserted_row_paints_like_a_cold_scene() {
    use nana_ui_runtime::Button;
    let document = DocumentId::new(1).unwrap();
    let mut runtime = self::document();
    let button = runtime
        .context_mut()
        .create_detached_component(document, Button::new("选择范围"))
        .unwrap()
        .stable_id();
    // `actions_side_by_side`: a wrapping row of filling width, inside another.
    let wrap_row = || NodeStyle {
        layout: Arc::new(LayoutStyle {
            width: Some(LengthSpec::Fill),
            direction: Some(FlexDirection::Row),
            flex_wrap: nana_ui_core::FlexWrap::Wrap,
            align_items: nana_ui_core::AlignSpec::Center,
            ..LayoutStyle::default()
        }),
        ..NodeStyle::default()
    };
    let mut queue = MutationQueue::new();
    for (bar, parent) in [(id(800), id(COLUMN)), (id(801), id(800))] {
        queue.create(bar, document, NodeKind::Element { tag: "div".into() });
        queue.insert(parent, bar, None);
        queue.set_style(bar, wrap_row());
    }
    queue.insert(id(801), button, None);
    runtime.context_mut().commit_mutations(queue).unwrap();
    flush(&mut runtime);
    assert_scene_is_cold(&runtime, "first frame");
    for extra in 0..3u64 {
        let row = id(900 + extra);
        let mut queue = MutationQueue::new();
        queue.create(row, document, NodeKind::Element { tag: "div".into() });
        queue.insert(id(COLUMN), row, Some(row_id(0)));
        queue.set_style(
            row,
            NodeStyle {
                layout: Arc::new(LayoutStyle {
                    width: Some(LengthSpec::Px(300.0)),
                    height: Some(LengthSpec::Px(20.0)),
                    ..LayoutStyle::default()
                }),
                background: Some(SemanticColorRole::Accent),
                ..NodeStyle::default()
            },
        );
        runtime.context_mut().commit_mutations(queue).unwrap();
        flush(&mut runtime);
        assert_scene_is_cold(&runtime, "row inserted");
    }
}
