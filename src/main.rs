use bevy::prelude::*;
use bevy_rapier2d::{
    prelude::*,
    rapier::dynamics::{RigidBodyForces, RigidBodyVelocity},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, apply_gravity)
        .run();
}

fn setup(mut commands: Commands, mut rapier_config: ResMut<RapierConfiguration>) {
    // Set gravity to zero for a space-like environment
    rapier_config.gravity = Vec2::ZERO;

    // Camera
    commands.spawn(Camera2dBundle { ..default() });

    // Create celestial bodies
    spawn_celestial_body(&mut commands, Vec2::new(-200.0, 0.0), 10.0);
    spawn_celestial_body(&mut commands, Vec2::new(200.0, 0.0), 10.0);
}

fn spawn_celestial_body(commands: &mut Commands, position: Vec2, radius: f32) {
    commands
        // .spawn(SpriteBundle {
        //     transform: Transform {
        //         translation: position.extend(0.0),
        //         scale: Vec3::splat(radius * 2.0),
        //         ..Default::default()
        //     },
        //     sprite: Sprite {
        //         color: Color::srgb(0.5, 0.5, 1.0),
        //         ..Default::default()
        //     },
        //     ..Default::default()
        // })
        .spawn(RigidBody::Dynamic)
        .insert(Collider::ball(radius))
        .insert(TransformBundle::from(Transform::from_xyz(
            position.x, position.y, 0f32,
        )))
        .insert(Sleeping::disabled())
        .insert(ExternalForce {
            force: Vec2::new(0.0, 0.0),
            torque: 0.0,
        });
}

fn apply_gravity(mut query: Query<(&mut ExternalForce, &Transform)>) {
    let gravitational_constant = 0.1;
    let mut bodies = query.iter_combinations_mut();

    while let Some([(mut force_a, transform_a), (mut force_b, transform_b)]) = bodies.fetch_next() {
        let direction = transform_b.translation - transform_a.translation;
        let direction2 = Vec2::new(direction.x, direction.y);
        let force = 10000.0 * direction2.normalize();
        force_a.force = force;
        force_b.force = -force;
    }
}
