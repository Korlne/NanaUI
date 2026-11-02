//! A group composites over what it can change, not over the whole dest.

use super::*;

/// Two small nested opacity groups composite a few hundred pixels each, in
/// order, and nothing outside them changes.
#[test]
fn small_nested_opacity_groups_limit_composite_fill_and_keep_pixels() {
    let (device, queue) = test_device();
    let mut painter = SceneWgpuPainter::for_test(wgpu::TextureFormat::Rgba8Unorm);
    let mut scene = UiScene::new();
    scene.apply_delta(
        [
            translucent_parent(1, &[2, 3], 0.0, 0.0, 200.0, 100.0, 0.5),
            colored_quad_child(2, 1, 10.0, 10.0, 20.0, 20.0, [1.0, 0.0, 0.0, 1.0]),
            child_of(translucent_parent(3, &[4], 0.0, 0.0, 200.0, 100.0, 0.5), 1),
            colored_quad_child(4, 3, 50.0, 10.0, 20.0, 20.0, [0.0, 0.0, 1.0, 1.0]),
        ],
        [],
    );
    let pixels = paint_scene_rgba(
        &device,
        &queue,
        &mut painter,
        &scene,
        [200.0, 100.0],
        [200, 100],
        1.0,
    );
    let counts = painter.last_dest_pass_counts.expect("a painted frame");
    assert!(counts.group >= 2, "nested groups: {counts:?}");
    assert!(
        counts.composite_pixels > 0 && counts.composite_pixels < 4_000,
        "small groups composite small: {counts:?}"
    );
    assert!(pixel(&pixels, 200, 20, 20)[0].abs_diff(128) <= 2);
    assert!(pixel(&pixels, 200, 60, 20)[2].abs_diff(64) <= 2);
    assert_eq!(pixel(&pixels, 200, 40, 20), [0, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 200, 190, 90), [0, 0, 0, 255]);
}

/// A fractional edge keeps its antialiased fringe inside the scissor, and a
/// group reaching past the dest is clipped at its edge.
#[test]
fn normal_group_keeps_antialias_fringe_and_clips_at_target_edge() {
    use nana_ui_runtime::BlendMode;
    let pixels = paint_one_painter(
        PaintFn(|cx: &mut nana_ui_runtime::PaintContext<'_>| {
            cx.fill_path(&rect_path(0.0, 0.0, 80.0, 40.0), [0.0, 1.0, 0.0, 1.0]);
            cx.push_layer(0.5, BlendMode::Normal);
            let mut disc = nana_ui_runtime::PaintPath::new();
            disc.ellipse(LayoutBox {
                x: 10.3,
                y: 10.6,
                width: 19.0,
                height: 19.0,
            });
            cx.fill_path(&disc, [1.0, 0.0, 0.0, 1.0]);
            cx.push_layer(0.5, BlendMode::Normal);
            cx.fill_path(&rect_path(70.0, 10.0, 30.0, 20.0), [0.0, 0.0, 1.0, 1.0]);
            cx.pop_layer();
            cx.pop_layer();
        }),
        80,
        40,
    );
    let at = |x, y| pixel(&pixels, 80, x, y);
    let centre = at(20, 20);
    assert!(
        centre[0].abs_diff(128) <= 2 && centre[1].abs_diff(128) <= 2,
        "{centre:?}"
    );
    let rim = at(10, 20);
    assert!(rim[0] > 0 && rim[0] < centre[0], "antialiased rim: {rim:?}");
    for (x, y) in [(9, 20), (20, 8), (31, 20), (20, 31), (50, 20)] {
        assert_eq!(at(x, y), [0, 255, 0, 255], "outside the group at ({x},{y})");
    }
    let clipped = at(79, 20);
    assert!(
        clipped[2].abs_diff(64) <= 2 && clipped[1].abs_diff(191) <= 2,
        "at the dest edge: {clipped:?}"
    );
}

/// A drop shadow paints past its source: its group composites everywhere.
#[test]
fn spreading_filter_keeps_pixels_outside_source_bounds() {
    let (device, queue) = test_device();
    let mut painter = SceneWgpuPainter::for_test(wgpu::TextureFormat::Rgba8Unorm);
    let mut scene = UiScene::new();
    scene.apply_delta(
        [extracted_div(
            1,
            &[],
            8.0,
            8.0,
            16.0,
            16.0,
            nana_ui_core::LayoutStyle {
                background: Some([1.0, 0.0, 0.0, 1.0]),
                paint: nana_ui_core::PaintStyle {
                    filter: Some(nana_ui_core::ColorFilter {
                        drop_shadow: Some(nana_ui_core::FilterDropShadow {
                            paint_color: None,
                            offset_x: 16.0,
                            offset_y: 0.0,
                            blur_radius: 0.0,
                            color: [0.0, 0.0, 1.0, 1.0],
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            },
            Some([1.0, 0.0, 0.0, 1.0]),
        )],
        [],
    );
    let pixels = paint_scene_rgba(
        &device,
        &queue,
        &mut painter,
        &scene,
        [64.0, 64.0],
        [64, 64],
        1.0,
    );
    let counts = painter.last_dest_pass_counts.expect("a painted frame");
    assert_eq!(
        counts.composite_pixels,
        64 * 64,
        "spreading filter: {counts:?}"
    );
    assert!(pixel(&pixels, 64, 12, 16)[0] > 250);
    assert!(
        pixel(&pixels, 64, 32, 16)[2] > 250,
        "the shadow past the source"
    );
    assert_eq!(pixel(&pixels, 64, 4, 16), [0, 0, 0, 255]);
}

/// A quad in an opacity group sliding on the GPU paints where the motion has
/// it, past the bounds it was encoded with: while anything moves on the GPU
/// the group composites over the whole dest.
#[test]
fn a_group_moving_on_the_gpu_is_not_clipped_to_its_encoded_bounds() {
    use nana_ui_core::{
        PaintTransform,
        motion::{AnimatableProperty, Easing, MotionTo, MotionValue},
    };
    use nana_ui_runtime::{AnimationId, AnimationSpec, MutationQueue, UiWorld};
    use std::time::Duration;

    let node = StableNodeId::new(2).unwrap();
    // From 32px right to rest: halfway through, 16px right.
    let spec = AnimationSpec::new(
        AnimationId::new(1).unwrap(),
        node,
        Duration::ZERO,
        Duration::from_millis(400),
        Duration::from_millis(16),
        Easing::Linear,
    )
    .with_property(AnimatableProperty::Transform)
    .with_range(
        MotionValue::Transform(PaintTransform {
            e: 32.0,
            ..PaintTransform::default()
        }),
        MotionTo::Value(MotionValue::Transform(PaintTransform::default())),
    );
    let mut world = UiWorld::new();
    let mut queue = MutationQueue::new();
    queue.create(node, DocumentId::new(1).unwrap(), NodeKind::Document);
    queue.start_animation(spec);
    world.commit(queue).unwrap();
    world.advance_animations(Duration::ZERO);
    let quad = colored_quad_child(2, 1, 0.0, 0.0, 16.0, 16.0, [1.0, 0.0, 0.0, 1.0]);
    let mut moving = world.extract_nodes(&[node]).remove(0);
    moving.parent = quad.parent;
    moving.layout = quad.layout;
    moving.source_style = quad.source_style;
    moving.style = quad.style;
    let mut scene = UiScene::new();
    scene.apply_delta(
        [
            translucent_parent(1, &[2], 0.0, 0.0, 64.0, 64.0, 0.5),
            moving,
        ],
        [],
    );
    scene.apply_presentation(
        world.presentation_store(),
        Duration::from_millis(200),
        Some(world.motion_descriptors()),
    );
    assert_ne!(
        scene.compositor_gpu_motion_ids(node).0,
        0,
        "the quad must move on the GPU"
    );

    let (device, queue) = test_device();
    let mut painter = SceneWgpuPainter::for_test(wgpu::TextureFormat::Rgba8Unorm);
    let pixels = paint_scene_rgba(
        &device,
        &queue,
        &mut painter,
        &scene,
        [64.0, 64.0],
        [64, 64],
        1.0,
    );
    assert!(
        pixel(&pixels, 64, 24, 8)[0].abs_diff(128) <= 2,
        "the slid quad: {:?}",
        pixel(&pixels, 64, 24, 8)
    );
    assert_eq!(
        pixel(&pixels, 64, 8, 8),
        [0, 0, 0, 255],
        "where it was encoded"
    );
}
