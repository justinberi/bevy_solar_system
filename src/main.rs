mod celestial_body;
mod stats;
mod trails;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use celestial_body::{
    add_celestial_body, add_sprite, CelestialBody, CelestialBodyAssets, CelestialBodyPlugin,
};
use trails::{Trail, TrailsPlugin};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use stats::StatsPlugin;
use std::ops::Range;

/// The main function of the game
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(CelestialBodyPlugin)
        .add_plugins(TrailsPlugin)
        .add_systems(Startup, setup);

    app.add_plugins(StatsPlugin);

    app.add_systems(Update, pan_camera);

    app.run();
}

#[derive(Component)]
struct MainCamera;

/// Sets up the N-body simulation
fn setup(
    mut commands: Commands,
    mut rapier_config: ResMut<RapierConfiguration>,
    rapier_context: Res<RapierContext>,
    celestial_body_assets: Res<CelestialBodyAssets>,
) {
    assert!(
        rapier_context.integration_parameters.length_unit >= 1.0,
        "pixels_per_meter must be >= 1.0"
    );

    // Set gravity to zero for a space-like environment
    rapier_config.gravity = Vec2::ZERO;

    // Camera
    commands
        .spawn(Camera2dBundle { ..default() })
        .insert(MainCamera);

    // Create bodies at know positions
    let entity = commands.spawn_empty().id();

    let mass = 10.0;
    add_sprite(&mut commands, entity, &celestial_body_assets, mass);
    add_celestial_body(
        &mut commands,
        entity,
        CelestialBody::default().with_mass(mass),
    );
    commands.entity(entity).insert(Trail::default());

    let entity = commands.spawn_empty().id();
    let mass = 5.0;
    add_sprite(&mut commands, entity, &celestial_body_assets, mass);
    add_celestial_body(
        &mut commands,
        entity,
        CelestialBody::default()
            .with_position(Vec2::new(-100f32, 0f32))
            .with_velocity(Vec2::new(60.0, 60.0))
            .with_mass(mass),
    );
    commands.entity(entity).insert(Trail::default());

    let entity = commands.spawn_empty().id();
    let mass = 5.0;
    add_sprite(&mut commands, entity, &celestial_body_assets, mass);
    add_celestial_body(
        &mut commands,
        entity,
        CelestialBody::default()
            .with_position(Vec2::new(100f32, 0f32))
            .with_velocity(Vec2::new(-100.0, -100.0))
            .with_mass(mass),
    );
    commands.entity(entity).insert(Trail::default());

    // FIXME: Why is this not determinstic?
    // Add some bodies at random positions
    let seed: [u8; 32] = [0; 32];
    let mut rng = StdRng::from_seed(seed);

    for _ in 0..10 {
        let mass = rng.gen_range(0.1..1.0) as f32;
        let entity = commands.spawn_empty().id();
        add_sprite(&mut commands, entity, &celestial_body_assets, mass);
        add_celestial_body(
            &mut commands,
            entity,
            CelestialBody {
                position: Vec2::gen_from_range(&mut rng, -400.0..400.0),
                velocity: Vec2::gen_from_range(&mut rng, -50.0..50.0),
                mass,
            },
        );
        commands.entity(entity).insert(Trail::default());
    }
}

/// Helper trait for generating random numbers for different types
trait MyRand {
    fn gen_from_range(g: &mut StdRng, range: Range<f32>) -> Self;
}

// FIXME: Should be a random direction
impl MyRand for Vec2 {
    fn gen_from_range(g: &mut StdRng, range: Range<f32>) -> Self {
        Vec2::new(g.gen_range(range.clone()), g.gen_range(range.clone()))
    }
}

const EDGE_THRESHOLD: f32 = 40.0;
const CAMERA_SPEED: f32 = 10.0;

fn pan_camera(
    mut camera_q: Query<(&Camera, &GlobalTransform, &mut Transform), With<MainCamera>>,
    windows_q: Query<&Window>,
) {
    let window = windows_q.single();
    let (camera, camera_global_transform, mut camera_transform) = camera_q.single_mut();

    if let Some(mouse_position) = windows_q.single().cursor_position() {
        let radius = (window.width() / 2.0).min(window.height() / 2.0) - EDGE_THRESHOLD;

        let window_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        let position_from_center = mouse_position - window_center;

        if position_from_center.length() < radius {
            return;
        }

        let t1 = camera
            .viewport_to_world_2d(camera_global_transform, mouse_position)
            .unwrap();
        let t2 = camera
            .viewport_to_world_2d(camera_global_transform, window_center)
            .unwrap();

        let dir = (t1 - t2).normalize();
        camera_transform.translation += CAMERA_SPEED * dir.extend(0.0);
    } // else outside window
}
