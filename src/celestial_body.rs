use bevy::{math::NormedVectorSpace, prelude::*};
use bevy_rapier2d::prelude::*;
use std::f32::consts::PI;

// TODO: This is probs going to break things.
use crate::MainCamera;

pub struct CelestialBodyPlugin;
impl Plugin for CelestialBodyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0)); //FIXME: This is broken

        #[cfg(debug_assertions)]
        app.add_plugins(RapierDebugRenderPlugin::default());

        app.add_systems(FixedFirst, reset_forces.in_set(PhysicsSet::SyncBackend))
            .add_systems(FixedUpdate, apply_gravity.in_set(PhysicsSet::SyncBackend))
            .add_systems(FixedUpdate, combine_bodies)
            .init_resource::<CelestialBodyAssets>();

        app.init_resource::<MouseDragState>();
        app.add_systems(Update, spawn_on_mouse_drag);

        #[cfg(debug_assertions)]
        app.add_systems(Update, debug_draw_two_body_connection);
    }
}

// https://bevy-cheatbook.github.io/programming/res.html
#[derive(Resource, Clone)]
pub struct CelestialBodyAssets {
    moon: Handle<Image>,
    earth: Handle<Image>,
    sun: Handle<Image>,
}

impl FromWorld for CelestialBodyAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self {
            moon: asset_server.load("sprites/moon.png"),
            earth: asset_server.load("sprites/earth.png"),
            sun: asset_server.load("sprites/sun.png"),
        }
    }
}

// TODO: Make this a setup system and change sprites depending on mass (ie asteriod, moon, planet, star, blackhole!)
pub fn add_sprite(
    commands: &mut Commands,
    entity: Entity,
    image_assets: &CelestialBodyAssets,
    mass: f32,
) {
    let image = if mass < 1.0 {
        image_assets.moon.clone()
    } else if mass < 7.0 {
        image_assets.earth.clone()
    } else {
        image_assets.sun.clone()
    };
    let radius = CelestialBody::radius_from_mass(mass);
    commands.entity(entity).insert(SpriteBundle {
        texture: image,
        sprite: Sprite {
            custom_size: Some(Vec2::new(2.0 * radius, 2.0 * radius)), // Set the desired size here
            ..Default::default()
        },
        ..Default::default()
    });
}

pub struct CelestialBody {
    pub position: Vec2,
    pub velocity: Vec2,
    pub mass: f32,
}

impl Default for CelestialBody {
    fn default() -> Self {
        CelestialBody {
            position: Vec2::default(),
            velocity: Vec2::default(),
            mass: 1.0,
        }
    }
}

impl CelestialBody {
    pub fn with_position(&self, position: Vec2) -> Self {
        CelestialBody { position, ..*self }
    }
    pub fn with_velocity(&self, velocity: Vec2) -> Self {
        CelestialBody { velocity, ..*self }
    }
    pub fn with_mass(&self, mass: f32) -> Self {
        assert!(mass > 0.0);
        CelestialBody { mass, ..*self }
    }

    /// Calculates the body's radius given mass.
    ///
    /// Assumes a constant density.
    pub fn radius_from_mass(mass: f32) -> f32 {
        5.0 * (mass / 2.0 * PI).sqrt()
    }
}

#[derive(Component, Default)]
struct TwoBodyProblem {
    entity: Option<Entity>,
    force: Option<f32>,
}

impl TwoBodyProblem {
    fn update(&mut self, entity: Entity, force: f32) {
        *self = Self {
            entity: Some(entity),
            force: Some(force),
        };
    }
}

/// Spawns a celesital body
///
// TODO: Should I use a Bundle here?
pub fn add_celestial_body(commands: &mut Commands, entity: Entity, body: CelestialBody) {
    let radius = CelestialBody::radius_from_mass(body.mass);
    commands
        .entity(entity)
        .insert(RigidBody::Dynamic)
        .insert(Collider::ball(radius))
        .insert(TransformBundle::from(Transform::from_xyz(
            body.position.x,
            body.position.y,
            0f32,
        )))
        .insert(ExternalForce {
            force: Vec2::new(0.0, 0.0),
            torque: 0.0,
        })
        .insert(AdditionalMassProperties::Mass(body.mass)) // Final mass of the body
        .insert(ColliderMassProperties::Density(0.0))
        .insert(ReadMassProperties::default())
        .insert(Velocity {
            linvel: body.velocity,
            angvel: 0.0,
        })
        .insert(ActiveEvents::COLLISION_EVENTS)
        .insert(TwoBodyProblem::default());
}

/// Zeros out external_forces and two body problem influence
fn reset_forces(mut query: Query<(&mut ExternalForce, &mut TwoBodyProblem)>) {
    for (mut external_forces, mut two_body_problem) in &mut query {
        external_forces.force = Vec2::default();
        *two_body_problem = TwoBodyProblem::default();
    }
}

/// Applies gravitational attraction contributions from all bodies.
// FIXME: Should use a "CelestialBody" component to differentiate from other bodies
fn apply_gravity(
    rapier_context: Res<RapierContext>,
    mut query: Query<(
        Entity,
        &mut ExternalForce,
        &Transform,
        &ReadMassProperties,
        &mut TwoBodyProblem,
    )>,
) {
    // Scale to SI units for force calculations
    let pixels_per_meter = rapier_context.integration_parameters.length_unit;
    let gravitational_constant = 10.0;
    let mut bodies = query.iter_combinations_mut();

    while let Some(
        [(entity1, mut force1, transform1, m1, mut two_body1), (entity2, mut force2, transform2, m2, mut two_body2)],
    ) = bodies.fetch_next()
    {
        let direction = (transform2.translation - transform1.translation) / pixels_per_meter; //FIXME: Get this scale from the physics config
        let direction2 = Vec2::new(direction.x, direction.y);
        let r2 = direction2.norm_squared();
        let force = gravitational_constant * m1.mass * m2.mass / r2 * direction2.normalize();

        if force.is_finite() {
            force1.force += force;
            force2.force += -force;

            // If the force is most influential, store the two body problem
            if two_body1.force.is_none() || two_body1.force.unwrap() < force.length() {
                two_body1.update(entity2, force.length())
            }
            if two_body2.force.is_none() || two_body2.force.unwrap() < force.length() {
                two_body2.update(entity1, force.length())
            }
        }
    }
}

use crate::Trail;

/// Combines the momentum of two bodies that collide
// TODO: Only do this when they have a stable collision
// TODO: Optional trail with Option<&Trail>
pub fn combine_bodies(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    image_assets: Res<CelestialBodyAssets>,
    query: Query<(&Transform, &Velocity, &ReadMassProperties, &Trail)>,
) {
    for collision_event in collision_events.read() {
        // Check for the correct collision event, otherwise skip
        let (e1, e2, _) = match collision_event {
            CollisionEvent::Started(e1, e2, flags) => (e1, e2, flags),
            CollisionEvent::Stopped(..) => continue,
        };

        let properties = (query.get(*e1), query.get(*e2));
        let (t1, v1, m1, trail1, t2, v2, m2, trail2) = match properties {
            (Ok((t1, v1, m1, trail1)), Ok((t2, v2, m2, trail2))) => {
                (t1, v1, m1, trail1, t2, v2, m2, trail2)
            }
            _ => continue,
        };

        // Calculate combined mass and velocity
        let mass1 = m1.mass;
        let mass2 = m2.mass;
        let combined_mass = mass1 + mass2;

        let combined_velocity = (v1.linvel * mass1 + v2.linvel * mass2) / combined_mass;
        if !combined_velocity.is_finite() {
            continue;
        }

        // Calculate new center of mass
        let combined_position =
            (t1.translation.truncate() * mass1 + t2.translation.truncate() * mass2) / combined_mass;

        if !combined_position.is_finite() {
            continue;
        }

        // Spawn new combined rigid body
        let entity = commands.spawn_empty().id();
        add_sprite(&mut commands, entity, &image_assets, combined_mass);

        add_celestial_body(
            &mut commands,
            entity,
            CelestialBody::default()
                .with_mass(combined_mass)
                .with_position(combined_position)
                .with_velocity(combined_velocity),
        );
        commands.entity(entity).insert(Trail::default());

        // Add a fading trail
        let fadout_time = 2f32;
        let mut trail1 = trail1.with_fadeout(fadout_time);
        let mut trail2 = trail2.with_fadeout(fadout_time);

        if mass1 > mass2 {
            trail1.add_vertex(combined_position);
        } else {
            trail2.add_vertex(combined_position);
        }
        commands.spawn(trail1);
        commands.spawn(trail2);

        // Despawn old entities
        commands.entity(*e1).despawn();
        commands.entity(*e2).despawn();
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

// Note this is inefficient as it will double up lines
// TODO: Could potentially add the TwoBodyProblem as another entity ID but meh
fn debug_draw_two_body_connection(
    world: &World,
    mut gizmos: Gizmos,
    query: Query<(&Transform, &TwoBodyProblem)>,
) {
    for (transform, two_body_problem) in &query {
        if let Some(other_entity) = two_body_problem.entity {
            if let Some(other_transform) = world.get::<Transform>(other_entity) {
                let color = Color::BLACK;
                let start = transform.translation.truncate();
                let end = other_transform.translation.truncate();
                gizmos.line_2d(start, end, color);
            }
        }
    }
}
