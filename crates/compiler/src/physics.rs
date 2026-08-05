use rapier3d::prelude::*;
use rapier3d::na::Isometry3;
use pystral_core::domain::HexMap;
use glam::Vec3;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryRequest {
    pub start: Vec3,
    pub target: Vec3,
    pub initial_speed: f32,
    pub gravity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryResponse {
    pub trajectory: Vec<Vec3>,
    pub rotations: Vec<f32>,
    pub yaw: f32,
}

pub struct TrajectorySystem {
}

impl TrajectorySystem {
    pub fn new() -> Self {
        Self {}
    }

    pub fn solve(&self, request: TrajectoryRequest, map: &HexMap) -> Result<TrajectoryResponse, String> {
        let mut physics = PhysicsWorld::new();
        physics.build_from_map(map);

        let diff = request.target - request.start;
        let g = request.gravity;
        
        // Try different speeds and angles
        for speed in [10.0, 15.0, 20.0, 25.0] {
            for angle_deg in (5..85).step_by(2) {
                let angle = (angle_deg as f32).to_radians();
                let v_y = speed * angle.sin();
                let v_h = speed * angle.cos();
                
                let horizontal_diff = Vec3::new(diff.x, 0.0, diff.z);
                if horizontal_diff.length() < 0.001 { continue; }
                let dir_h = horizontal_diff.normalize();
                let velocity = dir_h * v_h + Vec3::Y * v_y;
                
                let mut points = Vec::new();
                let mut rotations = Vec::new();
                let mut curr_pos = request.start;
                let mut curr_vel = velocity;
                let dt = 0.05;
                let mut hit_target = false;
                let mut collided = false;

                for _ in 0..100 {
                    points.push(curr_pos);
                    
                    let v_h_len = Vec3::new(curr_vel.x, 0.0, curr_vel.z).length();
                    let angle = curr_vel.y.atan2(v_h_len);
                    rotations.push(angle);
                    
                    if physics.is_colliding(curr_pos, 0.05) { // Smaller collision radius
                        collided = true;
                        break;
                    }

                    if (curr_pos - request.target).length() < 0.6 {
                        hit_target = true;
                        break;
                    }

                    if curr_pos.y < -5.0 { break; }

                    curr_pos += curr_vel * dt;
                    curr_vel += Vec3::Y * (-g * dt);
                }

                if hit_target && !collided {
                    let yaw = dir_h.z.atan2(dir_h.x);
                    return Ok(TrajectoryResponse { trajectory: points, rotations, yaw });
                }
            }
        }

        Err("Could not find a valid non-colliding trajectory".to_string())
    }
}

pub struct PhysicsWorld {
    pub collider_set: ColliderSet,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            collider_set: ColliderSet::new(),
        }
    }

    pub fn build_from_map(&mut self, map: &HexMap) {
        let layout = map.layout();
        for tile in &map.tiles {
            let world_pos = layout.hex_to_world_pos(tile.hex);
            // Smaller collider to allow for some wiggle room
            let collider = ColliderBuilder::cuboid(0.4, tile.height / 2.0, 0.4)
                .translation(vector![world_pos.x, tile.bottom + tile.height / 2.0, world_pos.y].into())
                .build();
            self.collider_set.insert(collider);
        }
    }

    pub fn is_colliding(&self, pos: Vec3, radius: f32) -> bool {
        let shape = SharedShape::ball(radius);
        let shape_pos = Isometry3::translation(pos.x, pos.y, pos.z);
        
        for (_, collider) in self.collider_set.iter() {
            if rapier3d::parry::query::intersection_test(
                &shape_pos.into(), &*shape,
                collider.position(), collider.shape()
            ).unwrap_or(false) {
                return true;
            }
        }
        false
    }
}
