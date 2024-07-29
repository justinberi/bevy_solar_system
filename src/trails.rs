use bevy::prelude::*;

use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

pub struct TrailsPlugin;
impl Plugin for TrailsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_polyline);
    }
}

const TRAIL_LENGTH: usize = 512;
#[derive(Component, Clone, Default, Debug)]
pub struct Trail {
    buffer: ConstGenericRingBuffer<Vec2, TRAIL_LENGTH>,
}

/// Draws a polyline for any entity that has a Trail component
// FIXME: The clone here is probs really bad performance due to .into() on the trail.
pub fn draw_polyline(mut gizmos: Gizmos, mut query: Query<(&mut Trail, &Transform)>) {
    for (mut trail, transform) in &mut query {
        let vert = Vec2::new(transform.translation.x, transform.translation.y);
        trail.buffer.push(vert);

        gizmos.linestrip_2d(trail.buffer.clone(), Color::WHITE);
    }
}
