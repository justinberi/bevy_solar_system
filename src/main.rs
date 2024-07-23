use bevy::{math::NormedVectorSpace, prelude::*};
use bevy_rapier2d::prelude::*;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f32::consts::PI;
use std::ops::Range;

use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

/// The main function of the game
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .add_systems(Startup, setup)
        .add_systems(FixedFirst, reset_forces.in_set(PhysicsSet::SyncBackend)) // Zero forces before physics step
        .add_systems(FixedUpdate, apply_gravity.in_set(PhysicsSet::SyncBackend)) // Apply gravity forces before physics step;
        .add_systems(Update, draw_polyline)
        .add_systems(FixedUpdate, combine_bodies);

    #[cfg(debug_assertions)]
    app.add_plugins(RapierDebugRenderPlugin::default());

    app.run();
}

/// Sets up the N-body simulation
fn setup(
    mut commands: Commands,
    mut rapier_config: ResMut<RapierConfiguration>,
    rapier_context: Res<RapierContext>,
    asset_server: Res<AssetServer>,
) {
    assert!(
        rapier_context.integration_parameters.length_unit >= 1.0,
        "pixels_per_meter must be >= 1.0"
    );

    // Set gravity to zero for a space-like environment
    rapier_config.gravity = Vec2::ZERO;

    // Camera
    commands.spawn(Camera2dBundle { ..default() });

    // Load the texture
    let image_assets = CelestialBodyAssets {
        moon: asset_server.load("sprites/moon.png"),
    };
    commands.insert_resource(image_assets.clone());

    // Create bodies at know positions
    let entity = commands.spawn_empty().id();

    let mass = 10.0;
    let radius = CelestialBody::radius_from_mass(mass);
    // Spawn the sprite
    commands.entity(entity).insert(SpriteBundle {
        texture: image_assets.moon.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::new(2.0 * radius, 2.0 * radius)), // Set the desired size here
            ..Default::default()
        },
        ..Default::default()
    });

    add_celestial_body(
        &mut commands,
        entity,
        CelestialBody::default().with_mass(mass),
    );
    commands.entity(entity).insert(Trail::default());

    let entity = commands.spawn_empty().id();
    let mass = 5.0;
    let radius = CelestialBody::radius_from_mass(mass);
    commands.entity(entity).insert(SpriteBundle {
        texture: image_assets.moon.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::new(2.0 * radius, 2.0 * radius)), // Set the desired size here
            ..Default::default()
        },
        ..Default::default()
    });
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
    let radius = CelestialBody::radius_from_mass(mass);
    commands.entity(entity).insert(SpriteBundle {
        texture: image_assets.moon.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::new(2.0 * radius, 2.0 * radius)), // Set the desired size here
            ..Default::default()
        },
        ..Default::default()
    });
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
        let radius = CelestialBody::radius_from_mass(mass);
        commands.entity(entity).insert(SpriteBundle {
            texture: image_assets.moon.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(2.0 * radius, 2.0 * radius)), // Set the desired size here
                ..Default::default()
            },
            ..Default::default()
        });
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

// FIXME: Not required public??
#[derive(Resource, Clone)]
pub struct CelestialBodyAssets {
    moon: Handle<Image>,
}

struct CelestialBody {
    position: Vec2,
    velocity: Vec2,
    mass: f32,
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
    fn with_position(&self, position: Vec2) -> Self {
        CelestialBody { position, ..*self }
    }
    fn with_velocity(&self, velocity: Vec2) -> Self {
        CelestialBody { velocity, ..*self }
    }
    fn with_mass(&self, mass: f32) -> Self {
        assert!(mass > 0.0);
        CelestialBody { mass, ..*self }
    }

    /// Calculates the body's radius given mass.
    ///
    /// Assumes a constant density.
    fn radius_from_mass(mass: f32) -> f32 {
        5.0 * (mass / 2.0 * PI).sqrt()
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

/// Spawns a celesital body
///
// TODO: Should I use a Bundle here?
fn add_celestial_body(commands: &mut Commands, entity: Entity, body: CelestialBody) {
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
        .insert(ActiveEvents::COLLISION_EVENTS);
}

/// Zeros out external_forces
fn reset_forces(mut query: Query<&mut ExternalForce>) {
    for mut external_forces in &mut query {
        external_forces.force = Vec2::default();
    }
}

/// Applies gravitational attraction contributions from all bodies.
// FIXME: Should use a "CelestialBody" component to differentiate from other bodies
fn apply_gravity(
    rapier_context: Res<RapierContext>,
    mut query: Query<(&mut ExternalForce, &Transform, &ReadMassProperties)>,
) {
    // Scale to SI units for force calculations
    let pixels_per_meter = rapier_context.integration_parameters.length_unit;
    let gravitational_constant = 10.0;
    let mut bodies = query.iter_combinations_mut();

    while let Some([(mut force1, transform1, m1), (mut force2, transform2, m2)]) =
        bodies.fetch_next()
    {
        let direction = (transform2.translation - transform1.translation) / pixels_per_meter; //FIXME: Get this scale from the physics config
        let direction2 = Vec2::new(direction.x, direction.y);
        let r2 = direction2.norm_squared();
        let force = gravitational_constant * m1.mass * m2.mass / r2 * direction2.normalize();

        if force.is_finite() {
            force1.force += force;
            force2.force += -force;
        }
    }
}

const TRAIL_LENGTH: usize = 512;
#[derive(Component, Clone, Default, Debug)]
struct Trail {
    buffer: ConstGenericRingBuffer<Vec2, TRAIL_LENGTH>,
}

/// Draws a polyline for any entity that has a Trail component
// FIXME: The clone here is probs really bad performance due to .into() on the trail.
fn draw_polyline(mut gizmos: Gizmos, mut query: Query<(&mut Trail, &Transform)>) {
    for (mut trail, transform) in &mut query {
        let vert = Vec2::new(transform.translation.x, transform.translation.y);
        trail.buffer.push(vert);

        gizmos.linestrip_2d(trail.buffer.clone(), Color::WHITE);
    }
}

/// Combines the momentum of two bodies that collide
// TODO: Only do this when they have a stable collision
pub fn combine_bodies(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    asset_handle: Res<CelestialBodyAssets>,
    query: Query<(&Transform, &Velocity, &ReadMassProperties)>,
) {
    for collision_event in collision_events.read() {
        // Check for the correct collision event, otherwise skip
        let (e1, e2, _) = match collision_event {
            CollisionEvent::Started(e1, e2, flags) => (e1, e2, flags),
            CollisionEvent::Stopped(..) => continue,
        };

        let properties = (query.get(*e1), query.get(*e2));
        let (t1, v1, m1, t2, v2, m2) = match properties {
            (Ok((t1, v1, m1)), Ok((t2, v2, m2))) => (t1, v1, m1, t2, v2, m2),
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
        let radius = CelestialBody::radius_from_mass(combined_mass);
        // Spawn the sprite
        commands.entity(entity).insert(SpriteBundle {
            texture: asset_handle.moon.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(2.0 * radius, 2.0 * radius)), // Set the desired size here
                ..Default::default()
            },
            ..Default::default()
        });

        add_celestial_body(
            &mut commands,
            entity,
            CelestialBody::default()
                .with_mass(combined_mass)
                .with_position(combined_position)
                .with_velocity(combined_velocity),
        );
        commands.entity(entity).insert(Trail::default());

        // Despawn old entities
        commands.entity(*e1).despawn();
        commands.entity(*e2).despawn();
    }
}
