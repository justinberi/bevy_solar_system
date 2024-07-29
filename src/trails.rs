use bevy::prelude::*;

use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

pub struct TrailsPlugin;
impl Plugin for TrailsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_trail_verticies, draw_trail).chain());
    }
}

const TRAIL_LENGTH: usize = 512;
#[derive(Component, Clone, Default, Debug)]
pub struct Trail {
    buffer: ConstGenericRingBuffer<Vec2, TRAIL_LENGTH>,
    color: Color,
    fadeout: FadeOut,
}

impl Trail {
    pub fn with_fadeout(&self, seconds: f32) -> Self {
        let fadeout = if seconds > 0.0 {
            FadeOut::InSeconds(Timer::from_seconds(seconds, TimerMode::Once))
        } else {
            FadeOut::None
        };

        Self {
            fadeout,
            ..self.clone()
        }
    }

    pub fn add_vertex(&mut self, vertex: Vec2) {
        self.buffer.push(vertex);
    }
}

#[derive(Clone, Debug)]
enum FadeOut {
    None,
    InSeconds(Timer),
}

impl Default for FadeOut {
    fn default() -> Self {
        Self::None
    }
}

fn update_trail_verticies(mut query: Query<(&mut Trail, &Transform)>) {
    for (mut trail, transform) in &mut query {
        let vert = Vec2::new(transform.translation.x, transform.translation.y);
        trail.buffer.push(vert);
    }
}

/// Draws a polyline for any entity that has a Trail component
// FIXME: The clone here is probs really bad performance due to .into() on the trail.
fn draw_trail(
    mut commands: Commands,
    time: Res<Time>,
    mut gizmos: Gizmos,
    mut query: Query<(Entity, &mut Trail)>,
) {
    for (entity, mut trail) in &mut query {
        gizmos.linestrip_2d(trail.buffer.clone(), trail.color);

        // Check how much time is left on a fadeout && despaw component if required
        if let FadeOut::InSeconds(ref mut timer) = trail.fadeout {
            timer.tick(time.delta());

            // Update the color so I can fade it
            let ratio = timer.remaining().as_secs_f32() / timer.duration().as_secs_f32();
            let alpha = if ratio.is_finite() { ratio } else { 1.0 };

            // Check if I should de spawn
            if timer.finished() {
                commands.entity(entity).remove::<Trail>();
            }

            trail.color.set_alpha(alpha);
        }
    }
}
