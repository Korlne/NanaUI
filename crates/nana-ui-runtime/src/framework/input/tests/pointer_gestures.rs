//! `PointerMoved` and `PointerDragged`: a view hears where the pointer is
//! and drags it, over children that are controls of their own.

use std::sync::{Arc, Mutex};

use super::*;
use crate::view::{El, entity_ref, widget, with_refs};
use crate::{
    Entity, LayoutViewport, LengthSpec, PointerDragPhase, PointerDragged, PointerMoved, Popover,
    PopoverToggled, Stack,
};

type Moves = Arc<Mutex<Vec<(&'static str, f32, f32)>>>;
type Drags = Arc<Mutex<Vec<PointerDragged>>>;

/// A rail (a row listening for both events) holding a popover whose trigger
/// the content draws and a painted marker that also hears moves; beside it a
/// popover in no listening node.
struct Fixture {
    cx: AppContext,
    input: TestInput,
    doc: DocumentId,
    rail: StableNodeId,
    marker: StableNodeId,
    chip: Entity<Popover>,
    plain: Entity<Popover>,
    moves: Moves,
    drags: Drags,
    toggles: Arc<Mutex<Vec<(&'static str, bool)>>>,
}

/// A box the size a painted piece would measure to; the pointer passes
/// through it to what holds it.
fn sized(width: f32, height: f32) -> El<Stack> {
    widget(Stack::row(0.0).with_layout(move |layout| {
        layout.width = Some(LengthSpec::Px(width));
        layout.height = Some(LengthSpec::Px(height));
    }))
}

/// A painted box the pointer lands on.
fn painted(width: f32, height: f32) -> El<Stack> {
    widget(Stack::row(0.0).hittable().with_layout(move |layout| {
        layout.width = Some(LengthSpec::Px(width));
        layout.height = Some(LengthSpec::Px(height));
    }))
}

impl Fixture {
    fn new() -> Self {
        let mut cx = AppContext::new();
        let doc = DocumentId::new(1).unwrap();
        let root = cx.create_component(doc, Stack::column(0.0)).unwrap();
        let moves = Moves::default();
        let drags = Drags::default();
        let (_, (rail, marker, chip, plain)) = cx
            .mount_view(root.stable_id(), || {
                let rail = entity_ref::<Stack>();
                let marker = entity_ref::<Stack>();
                let chip = entity_ref::<Popover>();
                let plain = entity_ref::<Popover>();
                let heard = Arc::clone(&moves);
                let on_marker = Arc::clone(&moves);
                let dragged = Arc::clone(&drags);
                let view = (
                    // Puts the rail below the top edge, so its own px differ
                    // from the window's.
                    sized(10.0, 30.0),
                    widget(Stack::row(12.0).hittable())
                        .entity_ref(rail)
                        .on(move |event: &PointerMoved| {
                            heard.lock().unwrap().push(("rail", event.x, event.y));
                        })
                        .on(move |event: &PointerDragged| {
                            dragged.lock().unwrap().push(*event);
                        })
                        .children((
                            widget(Popover::new().trigger("0.5"))
                                .entity_ref(chip)
                                .trigger(sized(48.0, 20.0))
                                .children(crate::view::button("Reset")),
                            painted(60.0, 20.0).entity_ref(marker).on(
                                move |event: &PointerMoved| {
                                    on_marker.lock().unwrap().push(("marker", event.x, event.y));
                                },
                            ),
                        )),
                    widget(Popover::new().trigger("1.0"))
                        .entity_ref(plain)
                        .trigger(sized(48.0, 20.0))
                        .children(crate::view::button("Reset")),
                );
                with_refs(view, (rail, marker, chip, plain))
            })
            .unwrap();
        let toggles = Arc::new(Mutex::new(Vec::new()));
        for (name, popover) in [("chip", chip), ("plain", plain)] {
            let heard = Arc::clone(&toggles);
            cx.on(popover, move |_, event: &PopoverToggled, _| {
                heard.lock().unwrap().push((name, event.open));
            })
            .unwrap();
        }
        let mut fixture = Self {
            cx,
            input: TestInput::default(),
            doc,
            rail: rail.stable_id(),
            marker: marker.stable_id(),
            chip,
            plain,
            moves,
            drags,
            toggles,
        };
        fixture.layout();
        fixture
    }

    fn layout(&mut self) {
        self.cx
            .layout_document(self.doc, LayoutViewport::new(800.0, 600.0))
            .unwrap();
        self.cx.rebuild_hit_test(self.doc);
    }

    fn center(&self, id: StableNodeId) -> (f32, f32) {
        let b = self.cx.world().layout_box(id).unwrap();
        (b.x + b.width / 2.0, b.y + b.height / 2.0)
    }

    /// `at` in `id`'s own px.
    fn local(&self, id: StableNodeId, at: (f32, f32)) -> (f32, f32) {
        let b = self.cx.world().layout_box(id).unwrap();
        (at.0 - b.x, at.1 - b.y)
    }

    fn open(&self, popover: Entity<Popover>) -> bool {
        self.cx.read(popover, |popover| popover.open).unwrap()
    }

    fn pointer(&mut self, phase: PointerPhase, at: (f32, f32), ms: u64) {
        let (x, y) = at;
        self.input
            .dispatch_at(
                &mut self.cx,
                self.doc,
                &pointer_fixture! {
                    phase,
                    pointer_id: 1,
                    pointer_type: PointerType::Mouse,
                    x,
                    y,
                    screen_x: x,
                    screen_y: y,
                    button: 0,
                    buttons: u16::from(phase != PointerPhase::Up),
                    pressure: 0.0,
                    tangential_pressure: 0.0,
                    tilt_x: 0,
                    tilt_y: 0,
                    twist: 0,
                    is_primary: true,
                    activation_click: false,
                    modifiers: Default::default(),
                },
                Duration::from_millis(ms),
            )
            .unwrap();
        self.layout();
    }

    fn drag_phases(&self) -> Vec<(PointerDragPhase, f32, f32)> {
        self.drags
            .lock()
            .unwrap()
            .iter()
            .map(|drag| (drag.phase, drag.dx, drag.dy))
            .collect()
    }
}

/// Moves reach the listening nodes at and above the one under the pointer,
/// innermost first and in each one's own px, over a control's trigger too;
/// a node that does not listen hears nothing, and leaving says nothing.
#[test]
fn pointer_moves_reach_listeners_in_their_own_px_innermost_first() {
    let mut f = Fixture::new();
    let on_marker = f.center(f.marker);
    f.pointer(PointerPhase::Move, on_marker, 10);
    let rail = f.local(f.rail, on_marker);
    let marker = f.local(f.marker, on_marker);
    assert!(rail.0 > marker.0 && rail.1 >= 0.0);
    assert_eq!(
        *f.moves.lock().unwrap(),
        [("marker", marker.0, marker.1), ("rail", rail.0, rail.1)]
    );

    // Over the chip's trigger: a control of its own inside the rail.
    f.moves.lock().unwrap().clear();
    let on_chip = f.center(f.chip.stable_id());
    f.pointer(PointerPhase::Move, on_chip, 20);
    let rail = f.local(f.rail, on_chip);
    assert_eq!(*f.moves.lock().unwrap(), [("rail", rail.0, rail.1)]);

    // Out of the rail: no listener is under the pointer.
    f.moves.lock().unwrap().clear();
    f.pointer(PointerPhase::Move, f.center(f.plain.stable_id()), 30);
    f.pointer(PointerPhase::Move, (700.0, 500.0), 40);
    assert!(f.moves.lock().unwrap().is_empty());
}

/// A press that does not travel is the ordinary press: the chip inside the
/// dragging rail opens, and no drag is raised.
#[test]
fn a_press_within_the_slop_still_opens_the_popover_and_raises_no_drag() {
    let mut f = Fixture::new();
    let (x, y) = f.center(f.chip.stable_id());
    f.pointer(PointerPhase::Down, (x, y), 100);
    f.pointer(PointerPhase::Move, (x + 2.0, y + 1.0), 110);
    f.pointer(PointerPhase::Up, (x + 2.0, y + 1.0), 120);
    assert!(f.open(f.chip));
    assert_eq!(*f.toggles.lock().unwrap(), [("chip", true)]);
    assert!(f.drags.lock().unwrap().is_empty());
}

/// Past the slop the rail drags: Started at the press point, a Moved per
/// sample with the travel from it, Ended on release. The chip the press
/// began on is no longer pressed and does not open.
#[test]
fn a_press_past_the_slop_drags_the_listener_and_does_not_open_the_popover() {
    let mut f = Fixture::new();
    let chip = f.chip.stable_id();
    let (x, y) = f.center(chip);
    f.pointer(PointerPhase::Down, (x, y), 100);
    assert_eq!(f.cx.world().pointer_press(f.doc, 1), Some(chip));
    f.pointer(PointerPhase::Move, (x + 3.0, y), 110);
    assert!(f.drags.lock().unwrap().is_empty(), "within the slop");
    f.pointer(PointerPhase::Move, (x + 6.0, y + 1.0), 120);
    assert_eq!(f.cx.world().pointer_press(f.doc, 1), None);
    assert_eq!(f.cx.world().pointer_capture(f.doc, 1), Some(f.rail));
    f.pointer(PointerPhase::Move, (x + 9.0, y - 2.0), 130);
    f.pointer(PointerPhase::Up, (x + 9.0, y - 2.0), 140);
    assert_eq!(
        f.drag_phases(),
        [
            (PointerDragPhase::Started, 0.0, 0.0),
            (PointerDragPhase::Moved, 6.0, 1.0),
            (PointerDragPhase::Moved, 9.0, -2.0),
            (PointerDragPhase::Ended, 9.0, -2.0),
        ]
    );
    let started = f.drags.lock().unwrap()[0];
    assert_eq!((started.x, started.y), f.local(f.rail, (x, y)));
    assert!(!f.open(f.chip), "the drag's release activates nothing");
    assert!(f.toggles.lock().unwrap().is_empty());
    assert_eq!(f.cx.world().pointer_capture(f.doc, 1), None);

    // The next still press is a press again.
    f.pointer(PointerPhase::Down, (x, y), 300);
    f.pointer(PointerPhase::Up, (x, y), 310);
    assert!(f.open(f.chip));
    assert_eq!(f.drags.lock().unwrap().len(), 4);
}

/// A cancelled pointer ends the drag as Cancelled and gives the capture back.
#[test]
fn a_cancelled_drag_reports_cancelled_and_releases_the_capture() {
    let mut f = Fixture::new();
    let (x, y) = f.center(f.marker);
    f.pointer(PointerPhase::Down, (x, y), 100);
    f.pointer(PointerPhase::Move, (x - 8.0, y), 110);
    f.pointer(PointerPhase::Cancel, (x - 8.0, y), 120);
    assert_eq!(
        f.drag_phases(),
        [
            (PointerDragPhase::Started, 0.0, 0.0),
            (PointerDragPhase::Moved, -8.0, 0.0),
            (PointerDragPhase::Cancelled, -8.0, 0.0),
        ]
    );
    assert_eq!(f.cx.world().pointer_capture(f.doc, 1), None);
}

/// Outside any listening node a travelling press is what it always was: the
/// popover it was released on opens, and nobody hears a drag.
#[test]
fn a_press_in_no_dragging_node_is_unchanged() {
    let mut f = Fixture::new();
    let (x, y) = f.center(f.plain.stable_id());
    f.pointer(PointerPhase::Down, (x, y), 100);
    f.pointer(PointerPhase::Move, (x + 6.0, y), 110);
    f.pointer(PointerPhase::Move, (x + 9.0, y), 120);
    assert_eq!(f.cx.world().pointer_capture(f.doc, 1), None);
    f.pointer(PointerPhase::Up, (x + 9.0, y), 130);
    assert!(f.open(f.plain));
    assert_eq!(*f.toggles.lock().unwrap(), [("plain", true)]);
    assert!(f.drags.lock().unwrap().is_empty());
}
