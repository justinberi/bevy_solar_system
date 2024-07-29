use bevy::{math::NormedVectorSpace, prelude::*};
use bevy_rapier2d::prelude::*;
use std::f32::consts::PI;

pub struct CelestialBodyPlugin;
impl Plugin for CelestialBodyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedFirst, reset_forces.in_set(PhysicsSet::SyncBackend))
            .add_systems(FixedUpdate, apply_gravity.in_set(PhysicsSet::SyncBackend))
            .add_systems(FixedUpdate, combine_bodies)
            .init_resource::<CelestialBodyAssets>();
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
        .insert(ActiveEvents::COLLISION_EVENTS);
}

/// Zeros out external_forces
pub fn reset_forces(mut query: Query<&mut ExternalForce>) {
    for mut external_forces in &mut query {
        external_forces.force = Vec2::default();
    }
}

/// Applies gravitational attraction contributions from all bodies.
// FIXME: Should use a "CelestialBody" component to differentiate from other bodies
pub fn apply_gravity(
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

use crate::Trail;

/// Combines the momentum of two bodies that collide
// TODO: Only do this when they have a stable collision
pub fn combine_bodies(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    image_assets: Res<CelestialBodyAssets>,
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
        add_sprite(&mut commands, entity, &image_assets, combined_mass);

        add_celestial_body(
            &mut commands,
            entity,
            CelestialBody::default()
                .with_mass(combined_mass)
                .with_position(combined_position)
                .with_velocity(combined_velocity),
        );

        // TODO: Maybe using <dyn Bundle> to insert this trail.
        commands.entity(entity).insert(Trail::default());

        // Despawn old entities
        commands.entity(*e1).despawn();
        commands.entity(*e2).despawn();
    }
}
