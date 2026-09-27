//! A whole Scan through the real wiring: a replayed camera, the frame pump
//! and production face reader, and Actions through the Flow.

use std::time::Duration;

use bevy::ecs::system::RunSystemOnce;
use rubic_core::{Facelets, Sequence};

use super::*;
use crate::flow::systems::apply_actions;
use crate::flow::{Flow, FlowKind};
use crate::scan::hud::hud_text;
use crate::scan::{CameraStatus, Scan};
use crate::solve::SolvePlayer;
use crate::types::{CubeRes, TurnQueue};
use crate::vision::capture::CAPTURE_ORDER;
use crate::vision::fixtures::face_frame;
use crate::vision::source::ReplaySource;

/// A Scan in progress, with a camera replaying `frames`.
fn scanning_app(frames: Vec<RgbImage>) -> App {
    let mut images = Assets::<Image>::default();
    let preview = images.add(Image::default());
    let mut app = App::new();
    app.insert_resource(images)
        .insert_resource(PreviewImage(preview))
        .insert_resource(Flow::Scanning(Box::default()))
        .insert_resource(CubeRes(Facelets::SOLVED))
        .init_resource::<TurnQueue>()
        .init_resource::<SolvePlayer>()
        .init_resource::<Time>()
        .add_event::<Action>()
        .insert_non_send_resource(CameraFeed(Some(Box::new(ReplaySource::new(frames)))))
        .add_systems(Update, (pump_camera, apply_actions).chain());
    app
}

fn act(app: &mut App, action: Action) {
    app.world_mut().send_event(action);
    app.world_mut().run_system_once(apply_actions).unwrap();
}

fn scan(app: &App) -> &Scan {
    app.world()
        .resource::<Flow>()
        .scan()
        .expect("still scanning")
}

#[test]
fn a_whole_scan_hands_the_cube_to_editing_and_releases_the_camera() {
    let cube = Facelets::SOLVED.apply_seq(&"R U R' U' F2 L D B'".parse::<Sequence>().unwrap());
    // The camera shows each face for one detection cycle.
    let frames = CAPTURE_ORDER
        .iter()
        .flat_map(|&face| (0..DETECT_INTERVAL).map(move |_| face_frame(&cube, face)))
        .collect();
    let mut app = scanning_app(frames);

    for (i, _) in CAPTURE_ORDER.iter().enumerate() {
        for _ in 0..DETECT_INTERVAL {
            app.update();
        }
        let scan = scan(&app);
        assert_eq!(scan.index(), i);
        assert_eq!(scan.camera_status(), CameraStatus::Live);
        assert!(scan.in_view(), "face {i} should be read from the frame");
        assert!(hud_text(scan, true).contains("In view"));
        act(&mut app, Action::Capture);
        act(&mut app, Action::NextFace);
    }

    let flow = app.world().resource::<Flow>();
    assert_eq!(flow.kind(), FlowKind::Editing);
    assert_eq!(flow.entry().unwrap().ready_cube(), Some(cube));
    assert!(
        app.world().non_send_resource::<CameraFeed>().0.is_none(),
        "the finished Scan releases the camera"
    );
}

#[test]
fn a_camera_that_sends_no_image_asks_for_access() {
    let mut app = scanning_app(Vec::new());
    app.update();
    assert_eq!(scan(&app).camera_status(), CameraStatus::Starting);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(5));
    app.update();
    assert_eq!(scan(&app).camera_status(), CameraStatus::Unavailable);
    assert!(hud_text(scan(&app), true).contains("Allow camera access"));
}
