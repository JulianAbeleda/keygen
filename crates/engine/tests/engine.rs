use keygen_engine::{
    ease,
    model::{
        Anchor, Color, Easing, ImageLayerSpec, MenuEntrySpec, MenuSpec, SceneSpec, TextLayerSpec,
        SCHEMA,
    },
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
        borderless: false,
        window_width: None,
        window_height: None,
        fit_window_to_display: false,
        immersive_system_ui: false,
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
            source_rect: None,
            visible_when_focused: None,
            nine_slice: None,
        }],
        particle_insertions: vec![],
        menu_insertion: None,
        menu: Some(MenuSpec {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            row_height: 1.0,
            spacing: 0.0,
            font_size: 1.0,
            outline_width: 0,
            color: Color([255; 4]),
            focused_color: None,
            outline: Color([0, 0, 0, 255]),
            focused_outline: Color([0, 0, 0, 255]),
            entries: vec![MenuEntrySpec {
                id: "entry".into(),
                label: "Entry".into(),
                enabled: true,
            }],
        }),
        text_layers: vec![],
        particles: None,
        fade: None,
    }
}

#[test]
fn scenes_without_menus_are_valid() {
    let mut scene = minimal_scene();
    scene.menu = None;
    assert!(scene.validate().is_ok());
}

#[test]
fn invalid_text_timing_fails_closed() {
    let mut scene = minimal_scene();
    scene.text_layers.push(TextLayerSpec {
        id: "boot".into(),
        text: "MEMORY OK".into(),
        x: 0.0,
        y: 0.0,
        font_size: 16.0,
        color: Color([255; 4]),
        outline: Color([0, 0, 0, 255]),
        outline_width: 0,
        visible_at: -1.0,
        characters_per_second: None,
        system_clock_24h: false,
        font_path: None,
    });
    assert!(scene.validate().is_err());
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

#[test]
fn json_rejects_empty_atlas_rect() {
    let json = br#"{
        "schema":"keygen.scene.v1","title":"atlas","design_width":1,"design_height":1,
        "clear":[0,0,0,255],"font_path":"font.ttf",
        "layers":[{"id":"image","path":"image.png","x":0,"y":0,"scale":1,
        "anchor":"top_left","alpha":1,"entrance":null,"motion":null,"source_rect":[0,0,0,4]}],
        "particle_insertions":[],"menu_insertion":null,"menu":null,"text_layers":[],
        "particles":null,"fade":null
    }"#;
    assert!(SceneSpec::from_json(json).is_err());
}

#[test]
fn invalid_nine_slice_geometry_fails_closed() {
    let mut scene = minimal_scene();
    scene.layers[0].source_rect = Some([0, 0, 8, 8]);
    scene.layers[0].nine_slice = Some(keygen_engine::model::NineSliceSpec {
        left: 4,
        top: 1,
        right: 4,
        bottom: 1,
        width: 16.0,
        height: 16.0,
    });
    assert!(scene.validate().is_err());
}

#[test]
fn borderless_and_focused_fill_fields_are_backward_compatible() {
    let mut scene = minimal_scene();
    scene.borderless = true;
    scene.menu.as_mut().unwrap().focused_color = Some(Color([8, 7, 6, 255]));
    assert!(scene.validate().is_ok());
}

#[test]
fn display_fit_and_fixed_window_geometry_are_mutually_exclusive() {
    let mut scene = minimal_scene();
    scene.fit_window_to_display = true;
    scene.window_width = Some(1280);
    scene.window_height = Some(720);
    assert!(scene.validate().is_err());
}
