use keygen_engine::{
    ease,
    model::{Anchor, Color, Easing, ImageLayerSpec, MenuEntrySpec, MenuSpec, SceneSpec, SCHEMA},
    Surface,
};

#[test]
fn easing_endpoints_are_exact() {
    for easing in [
        Easing::Linear,
        Easing::Ease,
        Easing::Cubic,
        Easing::Quint,
        Easing::Bounce,
    ] {
        assert_eq!(ease(0.0, easing), 0.0);
        assert_eq!(ease(1.0, easing), 1.0);
    }
}

#[test]
fn alpha_composition_is_deterministic() {
    let mut first = Surface::new(2, 1, [0, 0, 0, 255]);
    first.blend(0, 0, [200, 100, 50, 128], 1.0);
    let mut second = Surface::new(2, 1, [0, 0, 0, 255]);
    second.blend(0, 0, [200, 100, 50, 128], 1.0);
    assert_eq!(first, second);
    assert_eq!(&first.pixels[..4], &[100, 50, 25, 255]);
}

#[test]
fn png_encoding_is_deterministic() {
    let surface = Surface::new(2, 2, [12, 34, 56, 255]);
    assert_eq!(surface.encode_png().unwrap(), surface.encode_png().unwrap());
}

#[test]
fn offscreen_pixels_are_clipped() {
    let mut surface = Surface::new(1, 1, [1, 2, 3, 255]);
    surface.blend(-1, 0, [255, 0, 0, 255], 1.0);
    surface.blend(1, 0, [255, 0, 0, 255], 1.0);
    assert_eq!(surface.pixels, [1, 2, 3, 255]);
}

fn minimal_scene() -> SceneSpec {
    SceneSpec {
        schema: SCHEMA.into(),
        title: "test".into(),
        design_width: 1,
        design_height: 1,
        clear: Color([0, 0, 0, 255]),
        font_path: "font.ttf".into(),
        layers: vec![ImageLayerSpec {
            id: "image".into(),
            path: "image.png".into(),
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            anchor: Anchor::TopLeft,
            alpha: 1.0,
            entrance: None,
            motion: None,
        }],
        particle_insertions: vec![],
        menu_insertion: None,
        menu: MenuSpec {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            row_height: 1.0,
            spacing: 0.0,
            font_size: 1.0,
            outline_width: 0,
            color: Color([255; 4]),
            outline: Color([0, 0, 0, 255]),
            focused_outline: Color([0, 0, 0, 255]),
            entries: vec![MenuEntrySpec {
                id: "entry".into(),
                label: "Entry".into(),
                enabled: true,
            }],
        },
        particles: None,
        fade: None,
    }
}

#[test]
fn schema_rejection_fails_closed() {
    let mut scene = minimal_scene();
    scene.schema = "unknown".into();
    assert!(scene
        .validate()
        .unwrap_err()
        .contains("unsupported scene schema"));
}

#[test]
fn invalid_non_finite_geometry_fails_closed() {
    let mut scene = minimal_scene();
    scene.layers[0].x = f32::NAN;
    assert!(scene.validate().is_err());
}
