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
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .add_plugins(CelestialBodyPlugin)
        .add_plugins(TrailsPlugin)
        .add_systems(Startup, setup);

    app.init_resource::<MouseDragState>();
    app.add_systems(Update, spawn_on_mouse_drag);
    // .add_systems(Update, spawn_entity_on_click);

    #[cfg(debug_assertions)]
    app.add_plugins(RapierDebugRenderPlugin::default());

    app.add_plugins(StatsPlugin);

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

#[derive(Default, Resource)]
struct MouseDragState {
    dragging: bool,
    initial_position: Option<Vec2>,
    current_position: Option<Vec2>,
    entity: Option<Entity>,
}

impl MouseDragState {
    fn reset(&mut self) {
        *self = Self::default();
    }
}

// TODO: Split this into generating a non interacting sprite and then another system to convert it to a body
fn spawn_on_mouse_drag(
    mut commands: Commands,
    mut drag_state: ResMut<MouseDragState>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    celestial_body_assets: Res<CelestialBodyAssets>,
    mut gizmos: Gizmos,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    // Get the primary window
    let window = windows.single();

    // Get the camera
    let (camera, camera_transform) = camera_q.single();

    let mass = 1.0;

    // Check if the left mouse button is pressed
    if mouse_button_input.just_pressed(MouseButton::Left) {
        drag_state.dragging = true;
        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
        {
            let entity = commands.spawn_empty().id();
            add_sprite(&mut commands, entity, &celestial_body_assets, mass);
            commands
                .entity(entity)
                .insert(Transform::from_translation(world_position.extend(0.0)));

            // Store the data
            drag_state.initial_position = Some(world_position);
            drag_state.entity = Some(entity);
        }
    }

    // Get the current position based on mouse motion
    if drag_state.dragging {
        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
        {
            drag_state.current_position = Some(world_position);
        }
    }

    // Draw a vector using gizmos if we have valid positions
    if let (Some(initial_position), Some(current_position)) =
        (drag_state.initial_position, drag_state.current_position)
    {
        let red = Color::srgb(1.0, 0.0, 0.0);
        gizmos.linestrip_2d(vec![initial_position, current_position], red);
    }

    if mouse_button_input.just_released(MouseButton::Left) {
        // Spawn the entity
        // Spawn a new entity at the cursor's world position
        // let mass = rng.gen_range(0.1..1.0) as f32;
        let entity = commands.spawn_empty().id();

        if let (Some(inital_position), Some(current_position)) =
            (drag_state.initial_position, drag_state.current_position)
        {
            let velocity_scaled = inital_position - current_position;

            add_sprite(&mut commands, entity, &celestial_body_assets, mass);
            add_celestial_body(
                &mut commands,
                entity,
                CelestialBody {
                    position: inital_position,
                    velocity: velocity_scaled,
                    mass,
                },
            );
            commands.entity(entity).insert(Trail::default());
        }

        // Remove the ghost
        if let Some(entity) = drag_state.entity {
            commands.entity(entity).despawn();
        }

        // Clear it
        drag_state.reset();
    }
}
