use bevy::core::FrameCount;
use bevy::{math::NormedVectorSpace, prelude::*};
use bevy_rapier2d::prelude::*;
use std::f32::consts::PI;
use std::ops::Range;

// In an external project ..
// f64 and semi-implicit euler, dt=1e-6 conserves energy
// f32 and semi-implicit euler, dt=1e-6 does not conserve enery
// f64 and euler, dt=1e-6 does not conserve enery energy
// A configurable integrator would be best like in DRAKE MIT ...
// Also look at https://github.com/aevyrie/big_space
// Alternately mujuco might be a better way to go ...

// TODO: This is probs going to break things.
use crate::MainCamera;

pub struct CelestialBodyPlugin;
impl Plugin for CelestialBodyPlugin {
    fn build(&self, app: &mut App) {
        let pixels_per_meter = 10.0;

        // Add custom systems to physics engine
        // https://github.com/dimforge/bevy_rapier/blob/master/bevy_rapier2d/examples/custom_system_setup2.rs
        app.add_plugins(
            RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(pixels_per_meter)
                .with_default_system_setup(false),
        );

        // after the update step
        let stage = PostUpdate;
        app.configure_sets(
            stage.clone(),
            (
                PhysicsSet::SyncBackend,
                PhysicsSet::StepSimulation,
                PhysicsSet::Writeback,
            )
                .chain()
                .before(TransformSystem::TransformPropagate),
        );

        // TODO: Compare with Particular on github when it is back up
        app.add_systems(
            stage,
            (
                (
                    reset_forces,
                    apply_gravity,
                    get_orbital_elements,
                    RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::SyncBackend),
                )
                    .chain()
                    .in_set(PhysicsSet::SyncBackend),
                (RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::StepSimulation))
                    .in_set(PhysicsSet::StepSimulation),
                RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::Writeback)
                    .in_set(PhysicsSet::Writeback),
            ),
        );

        // Custom config
        let mut config = RapierConfiguration::new(pixels_per_meter);
        config.timestep_mode = TimestepMode::Fixed {
            dt: 1e-4,
            substeps: 1,
        };
        app.insert_resource(config);

        #[cfg(debug_assertions)]
        app.add_plugins(RapierDebugRenderPlugin::default());

        app.init_resource::<CelestialBodyAssets>();

        app.init_resource::<MouseDragState>();
        app.add_systems(Update, spawn_on_mouse_drag);

        #[cfg(debug_assertions)]
        app.add_systems(Update, debug_draw_two_body_connection);

        // app.add_systems(FixedUpdate, get_orbital_elements);
    }
}

const GRAVITATIONAL_CONSTANT: f32 = 10.0; //FIXME: A resource ...

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
        .insert(TwoBodyProblem::default())
        .insert(GravityScale(0.0));
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
    time: Res<Time<Fixed>>,
    frame_count: Res<FrameCount>,
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
    let mut bodies = query.iter_combinations_mut();

    let mut count = 0;

    while let Some(
        [(entity1, mut force1, transform1, m1, mut two_body1), (entity2, mut force2, transform2, m2, mut two_body2)],
    ) = bodies.fetch_next()
    {
        let r = (transform2.translation - transform1.translation).truncate() / pixels_per_meter; //FIXME: Get this scale from the physics config
        let r2 = r.norm_squared();
        let force = GRAVITATIONAL_CONSTANT * m1.mass * m2.mass * r.normalize() / r2;

        if force.is_finite() {
            force1.force = force;
            force2.force = -force;

            // If the force is most influential, store the two body problem
            if two_body1.force.is_none() || two_body1.force.unwrap() < force.length() {
                two_body1.update(entity2, force.length())
            }
            if two_body2.force.is_none() || two_body2.force.unwrap() < force.length() {
                two_body2.update(entity1, force.length())
            }
        }

        count += 1;
    }
    println!(
        "count: {count} fixed time: {} frame count: {} physics time: {}",
        time.elapsed_seconds(),
        frame_count.0,
        rapier_context.integration_parameters.dt
    );
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

#[derive(Component)]
pub struct TrajectoryPredictor;

// https://en.wikipedia.org/wiki/Kepler_orbit
fn get_orbital_elements(
    world: &World,
    mut gizmos: Gizmos,
    rapier_context: Res<RapierContext>,
    query: Query<
        (&Transform, &Velocity, &ReadMassProperties, &TwoBodyProblem),
        With<TrajectoryPredictor>,
    >,
) {
    for (transform, velocity, mass_properties, two_body_problem) in &query {
        let other_entity = match two_body_problem.entity {
            Some(v) => v,
            None => continue,
        };

        let other_transform = match world.get::<Transform>(other_entity) {
            Some(tf) => tf,
            None => continue,
        };

        let other_velocity = match world.get::<Velocity>(other_entity) {
            Some(v) => v,
            None => continue,
        };

        let other_mass_properties = match world.get::<ReadMassProperties>(other_entity) {
            Some(v) => v,
            None => continue,
        };

        let pixels_per_meter = rapier_context.integration_parameters.length_unit;

        // Polar orbital elements
        let r = (transform.translation - other_transform.translation) / pixels_per_meter;
        println!("{}", transform.translation / pixels_per_meter);
        let v = (velocity.linvel - other_velocity.linvel).extend(0.0) / pixels_per_meter;
        let mu = GRAVITATIONAL_CONSTANT * (mass_properties.mass + other_mass_properties.mass);
        let h = r.cross(v);
        let e_vector = (v.cross(h) / mu) - (r / r.length());
        let e = e_vector.length();
        let h2 = h.length_squared();
        let p = h2 / mu;

        println!(
            "e: {e}\tp: {p} dt: {}",
            rapier_context.integration_parameters.dt
        );

        let energy = 0.5 * v.length_squared() - mu / r.length();

        println!("{energy}");

        // let current_r = r.length();

        // TODO: Get location of the periapsis alternatley offset by current theta as determined from r.length().
        // let current_theta = ((p / r.length() - 1.0) / e).acos();

        // let current_normalized_position = Vec2::new(
        //     current_r * current_theta.cos(),
        //     current_r * current_theta.sin(),
        // );

        // let delta = r.truncate() - current_normalized_position;

        // r = p/(1 + e cos(theta))
        let mut draw = |kilo: i32| {
            let points: Vec<Vec2> = (-kilo..(kilo + 1))
                .map(|i| {
                    let phi = (i as f32) * 0.001 * PI;
                    let radius = p / (1.0 + e * phi.cos());
                    let orbital_position = Vec2::new(radius * phi.cos(), radius * phi.sin())
                        + other_transform.translation.truncate() / pixels_per_meter;

                    let screen_position = orbital_position * pixels_per_meter;
                    screen_position
                })
                // Rotate the orbit
                .collect();
            gizmos.linestrip_2d(points, Color::BLACK);
        };

        if (0.0..1.0).contains(&e) {
            // circle & ellipse
            // Draw -+pi
            draw(1000);
        } else if e >= 1.0 {
            // parabola, hyperbola
            // Draw -+0.75*pi
            draw(750);
        }
    }
}
