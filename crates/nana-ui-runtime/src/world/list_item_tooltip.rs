//! A ListItem with a tooltip hangs the tooltip under an anchor of its own:
//! an overlay host shows only its active child, so hosting it on the row
//! hid the row's slots, closed or open.
#![cfg(test)]

use std::sync::Arc;
use std::time::Duration;

use nana_ui_core::LengthSpec;

use super::reflow_oracle::{bundled_face_shaper, product_frame};
use crate::view::{column, widget};
use crate::{AppContext, DocumentId, Entity, LayoutViewport, ListItem, StableNodeId, Stack};

fn document() -> DocumentId {
    DocumentId::new(1).unwrap()
}

fn viewport() -> LayoutViewport {
    LayoutViewport::new(400.0, 300.0)
}

/// A 320px row with a detail and a 24px trailing slot, with or without a
/// tooltip, laid out once.
fn page(tooltip: bool) -> (AppContext, StableNodeId, StableNodeId) {
    let mut cx = AppContext::new();
    let root = cx
        .create_component(document(), Stack::column(0.0))
        .unwrap()
        .stable_id();
    let mut item = ListItem::new("Angle X").detail("0.5");
    if tooltip {
        item = item.tooltip(
            "Probably: head turn",
            nana_ui_core::TooltipConfig {
                delay_ms: 0,
                ..nana_ui_core::TooltipConfig::default()
            },
        );
    }
    Arc::make_mut(&mut item.style.layout).width = Some(LengthSpec::Px(320.0));
    let trailing = Stack::column(0.0).with_layout(|l| {
        l.width = Some(LengthSpec::Px(24.0));
        l.height = Some(LengthSpec::Px(24.0));
        l.flex_shrink = Some(0.0);
    });
    let view = cx
        .mount_view(root, || {
            column().children(
                widget(item)
                    .trailing(widget(trailing).key("trailing"))
                    .key("row"),
            )
        })
        .unwrap();
    let base = view.roots()[0];
    let row = cx.resolve_assembly_path(base, "row").unwrap();
    let slot = cx.resolve_assembly_path(base, "row/trailing").unwrap();
    product_frame(&mut cx, document(), viewport(), &mut bundled_face_shaper());
    (cx, row, slot)
}

#[test]
fn list_item_with_a_tooltip_keeps_its_trailing_slot() {
    let (plain, plain_row, plain_slot) = page(false);
    let (mut cx, row, slot) = page(true);
    let expected = (
        plain.world().layout_box(plain_row),
        plain.world().layout_box(plain_slot),
    );
    for open in [false, true] {
        let world = cx.world();
        assert!(world.is_overlay_reachable(slot), "open {open}: slot drawn");
        assert_eq!(
            (world.layout_box(row), world.layout_box(slot)),
            expected,
            "open {open}: laid out as without a tooltip"
        );
        cx.set_pointer_hover_at(document(), 1, Some(row), Duration::from_millis(10))
            .unwrap();
        cx.advance_animations(Duration::from_millis(20));
        let tooltip = cx
            .component_tooltip(Entity::<ListItem>::from_stable_id(row))
            .unwrap()
            .unwrap()
            .stable_id();
        let host = cx.world().parent_id(tooltip).unwrap();
        assert_eq!(cx.world().overlay_host(host).unwrap().active, Some(tooltip));
        product_frame(&mut cx, document(), viewport(), &mut bundled_face_shaper());
    }
}
