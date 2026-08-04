use glam::Vec3;
use rapier3d::{na::Isometry3, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectileCollider {
    Ball {
        radius: f32,
    },
    Capsule {
        segment_half_height: f32,
        radius: f32,
    },
}

impl ProjectileCollider {
    pub fn shape(&self) -> SharedShape {
        match self {
            Self::Ball { radius } => SharedShape::ball(*radius),
            Self::Capsule {
                segment_half_height,
                radius,
            } => SharedShape::capsule_y(*segment_half_height, *radius),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryRequest {
    pub start: Vec3,
    pub target: Vec3,
    pub speed_min: f32,
    pub speed_max: f32,
    pub speed_step: f32,
    pub angle_min_degrees: f32,
    pub angle_max_degrees: f32,
    pub angle_step_degrees: f32,
    pub gravity: f32,
    pub time_step: f32,
    pub max_steps: u32,
    pub ground_cutoff: f32,
    pub collider: ProjectileCollider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryResponse {
    pub trajectory: Vec<Vec3>,
    pub rotations: Vec<f32>,
    pub yaw: f32,
}

#[derive(Default, Clone)]
pub struct TrajectorySolver;

impl TrajectorySolver {
    pub fn new() -> Self {
        Self
    }

    pub fn solve_with_collision(
        &self,
        request: &TrajectoryRequest,
        mut is_colliding: impl FnMut(Vec3, &ProjectileCollider) -> bool,
    ) -> Result<TrajectoryResponse, String> {
        request.validate()?;
        let diff = request.target - request.start;
        let horizontal_diff = Vec3::new(diff.x, 0.0, diff.z);
        if horizontal_diff.length() < 0.001 {
            return Err("Projectile start and target must differ horizontally".to_string());
        }
        let dir_h = horizontal_diff.normalize();

        for speed in stepped_values(request.speed_min, request.speed_max, request.speed_step) {
            for angle_deg in stepped_values(
                request.angle_min_degrees,
                request.angle_max_degrees,
                request.angle_step_degrees,
            ) {
                let angle = angle_deg.to_radians();
                let velocity = dir_h * (speed * angle.cos()) + Vec3::Y * (speed * angle.sin());
                let mut points = Vec::new();
                let mut rotations = Vec::new();
                let mut curr_pos = request.start;
                let mut curr_vel = velocity;
                let mut hit_target = false;
                let mut collided = false;

                for _ in 0..request.max_steps {
                    points.push(curr_pos);
                    let horizontal_velocity = Vec3::new(curr_vel.x, 0.0, curr_vel.z).length();
                    rotations.push(curr_vel.y.atan2(horizontal_velocity));
                    if is_colliding(curr_pos, &request.collider) {
                        collided = true;
                        break;
                    }
                    if (curr_pos - request.target).length() < 0.6 {
                        hit_target = true;
                        break;
                    }
                    if curr_pos.y < request.ground_cutoff {
                        break;
                    }
                    curr_pos += curr_vel * request.time_step;
                    curr_vel += Vec3::Y * (-request.gravity * request.time_step);
                }

                if hit_target && !collided {
                    return Ok(TrajectoryResponse {
                        trajectory: points,
                        rotations,
                        yaw: dir_h.z.atan2(dir_h.x),
                    });
                }
            }
        }
        Err("Could not find a valid non-colliding trajectory".to_string())
    }
}

impl TrajectoryRequest {
    pub fn new(start: Vec3, target: Vec3) -> Self {
        Self {
            start,
            target,
            speed_min: 10.0,
            speed_max: 25.0,
            speed_step: 5.0,
            angle_min_degrees: 5.0,
            angle_max_degrees: 85.0,
            angle_step_degrees: 2.0,
            gravity: 9.81,
            time_step: 0.05,
            max_steps: 100,
            ground_cutoff: -5.0,
            collider: ProjectileCollider::Ball { radius: 0.05 },
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.speed_min <= 0.0 || self.speed_max < self.speed_min || self.speed_step <= 0.0 {
            return Err("Invalid projectile speed range".to_string());
        }
        if self.angle_max_degrees < self.angle_min_degrees || self.angle_step_degrees <= 0.0 {
            return Err("Invalid projectile angle range".to_string());
        }
        if self.gravity < 0.0 || self.time_step <= 0.0 || self.max_steps == 0 {
            return Err("Invalid projectile solver integration parameters".to_string());
        }
        Ok(())
    }
}

fn stepped_values(min: f32, max: f32, step: f32) -> Vec<f32> {
    let mut values = Vec::new();
    let mut value = min;
    while value <= max + f32::EPSILON {
        values.push(value.min(max));
        value += step;
    }
    values
}

pub fn intersection_collides(
    colliders: &ColliderSet,
    pos: Vec3,
    projectile: &ProjectileCollider,
) -> bool {
    let shape = projectile.shape();
    let shape_pos = Isometry3::translation(pos.x, pos.y, pos.z);
    colliders.iter().any(|(_, collider)| {
        rapier3d::parry::query::intersection_test(
            &shape_pos.into(),
            &*shape,
            collider.position(),
            collider.shape(),
        )
        .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_and_angle_ranges_are_independent() {
        let mut request = TrajectoryRequest::new(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0));
        request.speed_min = 10.0;
        request.speed_max = 10.0;
        request.speed_step = 1.0;
        request.angle_min_degrees = 0.0;
        request.angle_max_degrees = 0.0;
        request.angle_step_degrees = 1.0;
        request.gravity = 0.0;
        request.time_step = 0.1;
        request.max_steps = 10;
        request.ground_cutoff = -100.0;
        let response = TrajectorySolver::new()
            .solve_with_collision(&request, |_, _| false)
            .unwrap();
        assert!(response.trajectory.len() >= 3);
    }
}
