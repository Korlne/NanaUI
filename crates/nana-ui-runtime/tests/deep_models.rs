//! Application models that nest are walked without running out of stack:
//! a panel layout as deep as `MAX_SPLIT_DEPTH` allows, on a 512 KiB thread
//! in any build. (A tree view's data has no depth bound at all; its walks
//! are tested in `tree_view`.) See `docs/reference/resource-budgets.md`.

use nana_ui_core::SplitAxis;
use nana_ui_runtime::{
    AreaAxis, AreaLayoutModel, DockAxis, DockNode, DockWorkspace, LayoutBox, MAX_SPLIT_DEPTH,
    PaneTreeNode,
};

fn on_a_small_stack(run: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(512 << 10)
        .spawn(run)
        .unwrap()
        .join()
        .expect("a walk of the model ran out of stack");
}

#[test]
fn a_dock_as_deep_as_allowed_clones_compares_persists_and_drops() {
    on_a_small_stack(|| {
        let mut node = DockNode::item("leaf", None);
        for n in 0..MAX_SPLIT_DEPTH {
            node = DockNode::split(
                DockAxis::Horizontal,
                0.5,
                DockNode::item(format!("beside-{n}"), None),
                node,
            );
        }
        assert_eq!(node.split_depth(), MAX_SPLIT_DEPTH);
        let copy = node.clone();
        assert!(copy == node);
        assert_eq!(node.flatten().len(), MAX_SPLIT_DEPTH + 1);
        let workspace = DockWorkspace::new(node);
        let json = workspace.layout_json().unwrap();
        let restored = DockWorkspace::from_layout_json(&json).unwrap();
        assert_eq!(restored.main.split_depth(), MAX_SPLIT_DEPTH);
        drop((workspace, restored, copy));
    });
}

#[test]
fn an_area_layout_as_deep_as_allowed_places_persists_and_drops() {
    on_a_small_stack(|| {
        let mut model = AreaLayoutModel::single("a0", "editor");
        let mut last = String::from("a0");
        for n in 1..=MAX_SPLIT_DEPTH {
            let next = format!("a{n}");
            assert!(model.split(&last, AreaAxis::Horizontal, 0.5, next.as_str()));
            last = next;
        }
        assert_eq!(model.leaves().len(), MAX_SPLIT_DEPTH + 1);
        let bounds = LayoutBox {
            x: 0.0,
            y: 0.0,
            width: 1000.0,
            height: 600.0,
        };
        let _ = model.geometry(bounds, 2.0, 0.0);
        let json = serde_json::to_string(&model).unwrap();
        let restored: AreaLayoutModel = serde_json::from_str(&json).unwrap();
        assert!(restored == model);
        drop((model, restored));
    });
}

#[test]
fn a_pane_tree_as_deep_as_allowed_visits_clones_and_drops() {
    on_a_small_stack(|| {
        let mut node = PaneTreeNode::leaf("leaf");
        for n in 0..MAX_SPLIT_DEPTH {
            node = PaneTreeNode::split(
                format!("split-{n}"),
                SplitAxis::Horizontal,
                0.5,
                PaneTreeNode::leaf(format!("beside-{n}")),
                node,
            );
        }
        assert_eq!(node.split_depth(), MAX_SPLIT_DEPTH);
        let mut leaves = 0;
        node.visit_leaves(|_| leaves += 1);
        assert_eq!(leaves, MAX_SPLIT_DEPTH + 1);
        let copy = node.clone();
        assert!(copy == node);
        drop((node, copy));
    });
}

#[test]
fn the_deepest_dock_layout_json_can_hold_still_restores() {
    // Hand-edited or foreign, a saved layout can nest past MAX_SPLIT_DEPTH,
    // up to what `serde_json` reads; restoring it must not run out of stack.
    on_a_small_stack(|| {
        let deepest = (MAX_SPLIT_DEPTH..)
            .take_while(|&depth| DockWorkspace::from_layout_json(&dock_json(depth)).is_ok())
            .last()
            .expect("a layout at the split depth restores");
        assert!(deepest > MAX_SPLIT_DEPTH);
        let restored = DockWorkspace::from_layout_json(&dock_json(deepest)).unwrap();
        assert_eq!(restored.main.split_depth(), deepest);
    });
}

/// A saved dock layout `depth` splits deep, written out by hand.
fn dock_json(depth: usize) -> String {
    let mut node = String::from(r#"{"kind":"item","id":"leaf"}"#);
    for n in 0..depth {
        node = format!(
            r#"{{"kind":"split","axis":"horizontal","ratio":0.5,"first":{{"kind":"item","id":"beside-{n}"}},"second":{node}}}"#
        );
    }
    format!(r#"{{"version":1,"main":{node}}}"#)
}
