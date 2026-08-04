//! The tactical collision snapshot used by action generation and validation.
//!
//! `hexx` identifies tactical cells. Rapier owns the physics broad phase and
//! shape query. The types exposed here deliberately do not expose Rapier to
//! `npc-engine` callers.

use std::collections::HashMap;

use glam::Vec3;
use hexx::{HexLayout, HexOrientation};
use pystral_physics::{ProjectileCollider, TrajectoryRequest, TrajectorySolver};
use rapier3d::{
    math::{Pose, Vector},
    parry::query::ShapeCastOptions,
    prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{AgentId, GridCell, GridMap, TileType, UnitState};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CollisionGeometry {
    pub hex_size: f32,
    pub step_height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionMap {
    pub geometry: CollisionGeometry,
    pub trajectory: TrajectoryRequest,
}

impl Default for CollisionMap {
    fn default() -> Self {
        Self {
            geometry: CollisionGeometry::default(),
            trajectory: TrajectoryRequest::new(Vec3::ZERO, Vec3::X),
        }
    }
}

impl CollisionMap {
    pub fn build_world(
        &self,
        grid: &GridMap,
        units: &HashMap<AgentId, UnitState>,
    ) -> Result<CollisionWorld, String> {
        CollisionWorld::from_state(grid, units, self.geometry)
    }
}

impl Default for CollisionGeometry {
    fn default() -> Self {
        let hex_size = 1.0;
        Self {
            hex_size,
            step_height: (3.0_f32).sqrt() * hex_size,
        }
    }
}

impl CollisionGeometry {
    pub fn hex_width(self) -> f32 {
        (3.0_f32).sqrt() * self.hex_size
    }

    pub fn unit_capsule(self) -> (f32, f32) {
        // The total capsule height is one hex width, as required for every job:
        // 2 * segment_half_height + 2 * radius = hex_width.
        let radius = self.hex_width() / 4.0;
        let segment_half_height = self.hex_width() / 4.0;
        (segment_half_height, radius)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColliderKind {
    Tile { cell: GridCell, tile: TileType },
    Unit { agent: AgentId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionFilter {
    pub tiles: bool,
    pub units: bool,
}

impl Default for CollisionFilter {
    fn default() -> Self {
        Self {
            tiles: true,
            units: true,
        }
    }
}

impl CollisionFilter {
    fn query_groups(self) -> InteractionGroups {
        let mut filter = Group::NONE;
        if self.tiles {
            filter |= Group::GROUP_1;
        }
        if self.units {
            filter |= Group::GROUP_2;
        }
        InteractionGroups::new(Group::GROUP_3, filter, InteractionTestMode::And)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueryShape {
    Ball {
        radius: f32,
    },
    Capsule {
        segment_half_height: f32,
        radius: f32,
    },
}

impl QueryShape {
    fn shared(self) -> SharedShape {
        match self {
            Self::Ball { radius } => SharedShape::ball(radius),
            Self::Capsule {
                segment_half_height,
                radius,
            } => SharedShape::capsule_y(segment_half_height, radius),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrajectoryQuery {
    pub start: Vec3,
    pub end: Vec3,
    pub shape: QueryShape,
    pub filter: CollisionFilter,
    pub exclude_agent: Option<AgentId>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionHit {
    pub kind: ColliderKind,
    pub position: Vec3,
    pub normal: Vec3,
    pub fraction: f32,
}

#[derive(Debug, Clone, Copy)]
struct ColliderMetadata {
    kind: ColliderKind,
}

/// A read-only-after-build Rapier snapshot for tactical queries.
///
/// The broad-phase BVH is built once from the tactical snapshot. MCTS can
/// clone this structure or rebuild it from a cloned tactical state; render
/// state is never consulted.
#[derive(Clone)]
pub struct CollisionWorld {
    bodies: RigidBodySet,
    colliders: ColliderSet,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    metadata: HashMap<ColliderHandle, ColliderMetadata>,
}

impl CollisionWorld {
    pub fn arc_reaches_target(
        &self,
        request: &TrajectoryRequest,
        source: AgentId,
        target: AgentId,
    ) -> Result<(), String> {
        let solver = TrajectorySolver::new();
        solver
            .solve_with_collision(request, |position, collider| {
                let hit = self.cast_trajectory(TrajectoryQuery {
                    start: position,
                    end: position,
                    shape: query_shape(collider),
                    filter: CollisionFilter::default(),
                    exclude_agent: Some(source),
                });
                match hit {
                    Some(CollisionHit {
                        kind: ColliderKind::Unit { agent },
                        ..
                    }) if agent == target => false,
                    Some(_) => true,
                    None => false,
                }
            })
            .map(|_| ())
    }

    pub fn unit_world_position(cell: GridCell, geometry: CollisionGeometry) -> Vec3 {
        let layout = HexLayout {
            orientation: HexOrientation::Pointy,
            origin: glam::Vec2::ZERO,
            scale: glam::Vec2::splat(geometry.hex_size),
        };
        let center = layout.hex_to_world_pos(cell.hex);
        let (_, radius) = geometry.unit_capsule();
        let (segment_half_height, _) = geometry.unit_capsule();
        let base = (cell.layer as f32 + 1.0) * geometry.step_height;
        Vec3::new(center.x, base + segment_half_height + radius, center.y)
    }

    pub fn from_state(
        grid: &GridMap,
        units: &HashMap<AgentId, UnitState>,
        geometry: CollisionGeometry,
    ) -> Result<Self, String> {
        let layout = HexLayout {
            orientation: HexOrientation::Pointy,
            origin: glam::Vec2::ZERO,
            scale: glam::Vec2::splat(geometry.hex_size),
        };
        let mut world = Self {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            metadata: HashMap::new(),
        };

        for (&cell, &tile) in &grid.tiles {
            let center = layout.hex_to_world_pos(cell.hex);
            let bottom = cell.layer as f32 * geometry.step_height;
            let shape = hex_prism_shape(&layout, cell.hex, bottom, geometry.step_height)
                .ok_or_else(|| format!("Could not construct tile collider for {cell:?}"))?;
            let handle = world.colliders.insert(
                ColliderBuilder::new(shape)
                    .collision_groups(InteractionGroups::new(
                        Group::GROUP_1,
                        Group::GROUP_1 | Group::GROUP_2 | Group::GROUP_3,
                        InteractionTestMode::And,
                    ))
                    .position(Pose::translation(center.x, 0.0, center.y))
                    .build(),
            );
            world.metadata.insert(
                handle,
                ColliderMetadata {
                    kind: ColliderKind::Tile { cell, tile },
                },
            );
        }

        let (segment_half_height, radius) = geometry.unit_capsule();
        for (&agent, unit) in units {
            let center = layout.hex_to_world_pos(unit.position.hex);
            let base = (unit.position.layer as f32 + 1.0) * geometry.step_height;
            let handle = world.colliders.insert(
                ColliderBuilder::capsule_y(segment_half_height, radius)
                    .collision_groups(InteractionGroups::new(
                        Group::GROUP_2,
                        Group::GROUP_1 | Group::GROUP_2 | Group::GROUP_3,
                        InteractionTestMode::And,
                    ))
                    .translation(Vector::new(
                        center.x,
                        base + segment_half_height + radius,
                        center.y,
                    ))
                    .build(),
            );
            world.metadata.insert(
                handle,
                ColliderMetadata {
                    kind: ColliderKind::Unit { agent },
                },
            );
        }

        let handles: Vec<_> = world.colliders.iter().map(|(handle, _)| handle).collect();
        let mut events = Vec::new();
        world.broad_phase.update(
            &IntegrationParameters::default(),
            &world.colliders,
            &world.bodies,
            &handles,
            &[],
            &mut events,
        );
        Ok(world)
    }

    pub fn cast_trajectory(&self, query: TrajectoryQuery) -> Option<CollisionHit> {
        let shape = query.shape.shared();
        let start = Pose::translation(query.start.x, query.start.y, query.start.z);
        let velocity = Vector::new(
            query.end.x - query.start.x,
            query.end.y - query.start.y,
            query.end.z - query.start.z,
        );
        let excluded = query.exclude_agent.and_then(|agent| {
            self.metadata.iter().find_map(|(handle, metadata)| {
                (metadata.kind == ColliderKind::Unit { agent }).then_some(*handle)
            })
        });
        let mut query_filter = QueryFilter::default().groups(query.filter.query_groups());
        query_filter.exclude_collider = excluded;
        let pipeline = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            query_filter,
        );
        let (handle, hit) = pipeline.cast_shape(
            &start,
            velocity,
            &*shape,
            ShapeCastOptions::with_max_time_of_impact(1.0),
        )?;
        let metadata = self.metadata.get(&handle)?;
        Some(CollisionHit {
            kind: metadata.kind,
            position: query.start + (query.end - query.start) * hit.time_of_impact,
            normal: Vec3::new(hit.normal2.x, hit.normal2.y, hit.normal2.z),
            fraction: hit.time_of_impact,
        })
    }

    pub fn collider_count(&self) -> usize {
        self.colliders.len()
    }
}

fn query_shape(collider: &ProjectileCollider) -> QueryShape {
    match collider {
        ProjectileCollider::Ball { radius } => QueryShape::Ball { radius: *radius },
        ProjectileCollider::Capsule {
            segment_half_height,
            radius,
        } => QueryShape::Capsule {
            segment_half_height: *segment_half_height,
            radius: *radius,
        },
    }
}

fn hex_prism_shape(
    layout: &HexLayout,
    hex: hexx::Hex,
    bottom: f32,
    height: f32,
) -> Option<SharedShape> {
    let corners = layout.hex_corners(hex);
    let center = layout.hex_to_world_pos(hex);
    let mut points = Vec::with_capacity(12);
    for corner in corners {
        points.push(Vector::new(
            corner.x - center.x,
            bottom,
            corner.y - center.y,
        ));
        points.push(Vector::new(
            corner.x - center.x,
            bottom + height,
            corner.y - center.y,
        ));
    }
    SharedShape::convex_hull(&points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_one_regular_hex_prism_per_present_tile() {
        let mut grid = GridMap {
            bounds: crate::GridBounds {
                horizontal: hexx::HexBounds::from_radius(1),
                min_layer: 0,
                max_layer: 1,
            },
            tiles: HashMap::new(),
        };
        grid.set_tile(GridCell::new(hexx::Hex::ZERO, 0), TileType::Grass)
            .unwrap();
        grid.set_tile(GridCell::new(hexx::Hex::ZERO, 1), TileType::Rock)
            .unwrap();

        let world =
            CollisionWorld::from_state(&grid, &HashMap::new(), CollisionGeometry::default())
                .unwrap();
        assert_eq!(world.collider_count(), 2);
    }

    #[test]
    fn trajectory_returns_the_first_tile_hit_and_holes_have_no_collider() {
        let mut grid = GridMap {
            bounds: crate::GridBounds {
                horizontal: hexx::HexBounds::from_radius(1),
                min_layer: 0,
                max_layer: 0,
            },
            tiles: HashMap::new(),
        };
        grid.set_tile(GridCell::new(hexx::Hex::ZERO, 0), TileType::Grass)
            .unwrap();
        let world =
            CollisionWorld::from_state(&grid, &HashMap::new(), CollisionGeometry::default())
                .unwrap();

        let hit = world
            .cast_trajectory(TrajectoryQuery {
                start: Vec3::new(0.0, 3.0, 0.0),
                end: Vec3::new(0.0, -1.0, 0.0),
                shape: QueryShape::Ball { radius: 0.05 },
                filter: CollisionFilter::default(),
                exclude_agent: None,
            })
            .expect("trajectory should hit the present tile");
        assert!(
            matches!(hit.kind, ColliderKind::Tile { cell, tile: TileType::Grass } if cell == GridCell::new(hexx::Hex::ZERO, 0))
        );
        assert!(hit.fraction > 0.0 && hit.fraction < 1.0);

        assert!(
            world
                .cast_trajectory(TrajectoryQuery {
                    start: Vec3::new(0.0, 3.0, 0.0),
                    end: Vec3::new(0.0, -1.0, 0.0),
                    shape: QueryShape::Ball { radius: 0.05 },
                    filter: CollisionFilter {
                        tiles: false,
                        units: true,
                    },
                    exclude_agent: None,
                })
                .is_none()
        );

        let hole_center =
            CollisionWorld::from_state(&grid, &HashMap::new(), CollisionGeometry::default())
                .unwrap()
                .cast_trajectory(TrajectoryQuery {
                    start: Vec3::new(1.732, 3.0, 0.0),
                    end: Vec3::new(1.732, -1.0, 0.0),
                    shape: QueryShape::Ball { radius: 0.05 },
                    filter: CollisionFilter::default(),
                    exclude_agent: None,
                });
        assert!(hole_center.is_none());
    }

    #[test]
    fn every_job_uses_the_same_one_hex_width_capsule_dimensions() {
        let geometry = CollisionGeometry::default();
        let (segment_half_height, radius) = geometry.unit_capsule();
        assert!(
            (2.0 * segment_half_height + 2.0 * radius - geometry.hex_width()).abs() < f32::EPSILON
        );
    }
}
