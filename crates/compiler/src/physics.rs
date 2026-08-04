use glam::Vec3;
use pystral_core::domain::HexMap;
use pystral_physics::TrajectorySolver;
use rapier3d::prelude::*;

pub use pystral_physics::{ProjectileCollider, TrajectoryRequest, TrajectoryResponse};

#[derive(Default, Clone)]
pub struct TrajectorySystem {
    solver: TrajectorySolver,
}

impl TrajectorySystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn solve(
        &self,
        request: &TrajectoryRequest,
        map: &HexMap,
    ) -> Result<TrajectoryResponse, String> {
        let mut physics = PhysicsWorld::default();
        physics.build_from_map(map);
        self.solver
            .solve_with_collision(request, |position, collider| {
                physics.is_colliding(position, collider)
            })
    }
}

#[derive(Default)]
pub struct PhysicsWorld {
    pub collider_set: ColliderSet,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build_from_map(&mut self, map: &HexMap) {
        let layout = map.layout();
        for tile in &map.tiles {
            let world_pos = layout.hex_to_world_pos(tile.hex);
            let collider = ColliderBuilder::cuboid(0.4, tile.height / 2.0, 0.4)
                .translation(
                    vector![world_pos.x, tile.bottom + tile.height / 2.0, world_pos.y].into(),
                )
                .build();
            self.collider_set.insert(collider);
        }
    }

    pub fn is_colliding(&self, pos: Vec3, projectile: &ProjectileCollider) -> bool {
        pystral_physics::intersection_collides(&self.collider_set, pos, projectile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_solver_ranges_are_rejected() {
        let mut request = TrajectoryRequest::new(Vec3::ZERO, Vec3::X);
        request.angle_step_degrees = 0.0;
        assert!(request.validate().is_err());
    }
}
