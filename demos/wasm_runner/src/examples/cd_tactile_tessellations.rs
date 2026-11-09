// Auto-generated wrapper for bevy_composable_dragging/tactile_tessellations
#[path = "../../../../libs/bevy_composable_dragging/examples/common/mod.rs"]
mod common;

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::prelude::*;
use bevy_composable_dragging::ops::{
    closest, custom, drop_target, state, switch_to_state_and_follow, vary,
};
use bevy_composable_dragging::{DragSpec, DraggablePlugin, Param, View, ViewNode};
use bevy_gradmesh::{Shape, ShapeDetail, Style};

use common::screenshot::AutoScreenshotPlugin;
use common::script::ScriptedDragPlugin;

/// The side length of every polygon (50 px in the original).
const SIDE: f32 = 1.0;
/// How far two polygons may overlap and still count as touching (2 px in the original).
const OVERLAP_EPSILON: f32 = 0.04;
/// Edges of this length difference or less are equal (0.1 px in the original).
const EDGE_TOLERANCE: f32 = 0.002;
/// The distance from every snap beyond which a tile follows the pointer freely (30 px).
const GAP: f32 = 0.6;
/// The thickness of the outline of a tile.
const OUTLINE: f32 = 0.04;
/// The number of sides of the polygons in the palette.
const KINDS: [u32; 4] = [3, 4, 6, 8];
/// The height of the canvas in world units (450 px in the original).
const CANVAS_HEIGHT: f32 = 9.0;
/// The id of the trash bin node.
const TRASH: &str = "trash-bin";

// ---------------------------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------------------------

/// The vertices of the regular polygon with `sides` sides, counterclockwise from the bottom left,
/// with a flat bottom edge and the center at the origin.
fn vertices(sides: u32) -> Vec<Vec2> {
    let n = sides as f32;
    let radius = SIDE / (2.0 * (PI / n).sin());
    (0..sides)
        .map(|i| Vec2::from_angle(-FRAC_PI_2 - PI / n + TAU * i as f32 / n) * radius)
        .collect()
}

/// The vertices after turning by `rot_deg` degrees around the origin and moving to `pos`.
fn world_vertices(sides: u32, pos: Vec2, rot_deg: f32) -> Vec<Vec2> {
    let rotation = Vec2::from_angle(rot_deg.to_radians());
    vertices(sides)
        .into_iter()
        .map(|vertex| rotation.rotate(vertex) + pos)
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct Edge {
    a: Vec2,
    b: Vec2,
}

impl Edge {
    fn mid(self) -> Vec2 {
        self.a.midpoint(self.b)
    }

    fn length(self) -> f32 {
        self.a.distance(self.b)
    }

    fn dir(self) -> Vec2 {
        (self.b - self.a).normalize()
    }
}

fn edges(vertices: &[Vec2]) -> Vec<Edge> {
    vertices
        .iter()
        .enumerate()
        .map(|(i, &a)| Edge {
            a,
            b: vertices[(i + 1) % vertices.len()],
        })
        .collect()
}

/// The interval of `vertices` projected onto `axis`.
fn project(vertices: &[Vec2], axis: Vec2) -> (f32, f32) {
    vertices
        .iter()
        .map(|vertex| vertex.dot(axis))
        .fold((f32::MAX, f32::MIN), |(min, max), dot| {
            (min.min(dot), max.max(dot))
        })
}

fn axes(vertices: &[Vec2]) -> Vec<Vec2> {
    edges(vertices)
        .into_iter()
        .map(|edge| {
            let along = edge.b - edge.a;
            Vec2::new(-along.y, along.x).normalize()
        })
        .collect()
}

/// Whether two convex polygons overlap by more than `epsilon`, by the separating axis theorem.
fn polygons_overlap(a: &[Vec2], b: &[Vec2], epsilon: f32) -> bool {
    axes(a).into_iter().chain(axes(b)).all(|axis| {
        let (min_a, max_a) = project(a, axis);
        let (min_b, max_b) = project(b, axis);
        max_a - epsilon > min_b && max_b - epsilon > min_a
    })
}

/// Normalizes an angle to `[-PI, PI)`.
fn normalize_angle(radians: f32) -> f32 {
    (radians + PI).rem_euclid(TAU) - PI
}

// ---------------------------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
struct Tile {
    id: u32,
    sides: u32,
    pos: Vec2,
    rot_deg: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Board {
    tiles: Vec<Tile>,
    next_id: u32,
}

/// A position and rotation a tile can snap to.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Snap {
    pos: Vec2,
    rot_deg: f32,
}

impl Board {
    fn tile(&self, id: u32) -> &Tile {
        self.tiles
            .iter()
            .find(|tile| tile.id == id)
            .expect("the dragged tile exists")
    }

    fn tile_mut(&mut self, id: u32) -> &mut Tile {
        self.tiles
            .iter_mut()
            .find(|tile| tile.id == id)
            .expect("the dragged tile exists")
    }

    fn snapped(&self, id: u32, snap: Snap) -> Self {
        let mut board = self.clone();
        let tile = board.tile_mut(id);
        tile.pos = snap.pos;
        tile.rot_deg = snap.rot_deg;
        board
    }

    fn without(&self, id: u32) -> Self {
        let mut board = self.clone();
        board.tiles.retain(|tile| tile.id != id);
        board
    }

    /// The board with a new tile of `sides` sides at `pos`.
    fn with_new(&self, sides: u32, pos: Vec2) -> (Self, u32) {
        let mut board = self.clone();
        let id = board.next_id;
        board.next_id += 1;
        board.tiles.push(Tile {
            id,
            sides,
            pos,
            rot_deg: 0.0,
        });
        (board, id)
    }
}

/// Whether a tile of `sides` sides at `pos` turned by `rot_deg` overlaps any tile but `exclude`.
fn overlaps_any(sides: u32, pos: Vec2, rot_deg: f32, tiles: &[Tile], exclude: Option<u32>) -> bool {
    let candidate = world_vertices(sides, pos, rot_deg);
    tiles
        .iter()
        .filter(|tile| Some(tile.id) != exclude)
        .any(|tile| {
            let existing = world_vertices(tile.sides, tile.pos, tile.rot_deg);
            polygons_overlap(&candidate, &existing, OVERLAP_EPSILON)
        })
}

/// Every valid edge-to-edge placement of a tile with `sides` sides against `tiles`.
///
/// Two edges snap when they have the same length and can be laid flush against each other with
/// reversed direction. The tile turns by the least angle that does so, and snaps whose result
/// overlaps another tile are dropped.
fn compute_snaps(sides: u32, tiles: &[Tile], exclude: Option<u32>) -> Vec<Snap> {
    let at_origin = edges(&vertices(sides));
    let mut snaps = Vec::new();
    for tile in tiles.iter().filter(|tile| Some(tile.id) != exclude) {
        let existing = edges(&world_vertices(tile.sides, tile.pos, tile.rot_deg));
        for edge in existing {
            let target_angle = (-edge.dir()).to_angle();
            let rotation_for = |own: &Edge| normalize_angle(target_angle - own.dir().to_angle());
            let Some(best) = at_origin
                .iter()
                .filter(|own| (own.length() - edge.length()).abs() <= EDGE_TOLERANCE)
                .min_by(|a, b| rotation_for(a).abs().total_cmp(&rotation_for(b).abs()))
            else {
                continue;
            };
            let rotation = rotation_for(best);
            let pos = edge.mid() - Vec2::from_angle(rotation).rotate(best.mid());
            let rot_deg = rotation.to_degrees().rem_euclid(360.0);
            if !overlaps_any(sides, pos, rot_deg, tiles, exclude) {
                snaps.push(Snap { pos, rot_deg });
            }
        }
    }
    snaps
}

// ---------------------------------------------------------------------------------------------
// Drag specs
// ---------------------------------------------------------------------------------------------

/// Makes the center of the node follow the pointer instead of the point that was grabbed, like
/// `withInitContext({ anchorPos: Vec2(0) })` in the original.
fn anchored_at_center(spec: DragSpec<Board>) -> DragSpec<Board> {
    custom(move |context| {
        let offset = context.grab_offset();
        let mut behavior = spec.compile(&context.clone().with_grab_offset(Vec3::ZERO));
        Box::new(move |pointer| behavior(pointer.moved_to(pointer.point + offset)))
    })
}

/// A parameter of the tile `id`.
fn tile_param(id: u32, get: fn(&Tile) -> f32, set: fn(&mut Tile, f32)) -> Param<Board> {
    Param::new(
        move |board: &Board| get(board.tile(id)),
        move |board: &mut Board, value| set(board.tile_mut(id), value),
    )
}

/// The drag of a tile: snap to the nearest valid edge, drop on the bin to delete, or follow the
/// pointer when far from both.
fn tile_spec(board: &Board, id: u32) -> DragSpec<Board> {
    let tile = board.tile(id);
    let snaps = compute_snaps(tile.sides, &board.tiles, Some(id));
    let mut branches: Vec<DragSpec<Board>> = snaps
        .into_iter()
        .map(|snap| state(board.snapped(id, snap)))
        .collect();
    branches.push(drop_target(TRASH, board.without(id)));
    let free = vary(
        board.clone(),
        [
            tile_param(id, |tile| tile.pos.x, |tile, x| tile.pos.x = x),
            tile_param(id, |tile| tile.pos.y, |tile, y| tile.pos.y = y),
        ],
    );
    let node = tile_id(id);
    anchored_at_center(closest(branches).when_far(free, GAP)).change_result(move |mut frame| {
        if let Some(dragged) = frame.view.get_mut(&node) {
            dragged.transform.translation.z = TILE_Z + 1.0;
        }
        frame
    })
}

// ---------------------------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------------------------

const CANVAS_Z: f32 = -1.0;
const PALETTE_Z: f32 = -0.5;
const PALETTE_ITEM_Z: f32 = 0.0;
const TILE_Z: f32 = 0.2;

fn tile_id(id: u32) -> String {
    format!("shape-{id}")
}

fn fill(sides: u32) -> Color {
    match sides {
        3 => Color::srgb_u8(0xfc, 0xa5, 0xa5),
        4 => Color::srgb_u8(0x93, 0xc5, 0xfd),
        6 => Color::srgb_u8(0x86, 0xef, 0xac),
        _ => Color::srgb_u8(0xfd, 0xe6, 0x8a),
    }
}

fn stroke(sides: u32) -> Color {
    match sides {
        3 => Color::srgb_u8(0xef, 0x44, 0x44),
        4 => Color::srgb_u8(0x3b, 0x82, 0xf6),
        6 => Color::srgb_u8(0x16, 0xa3, 0x4a),
        _ => Color::srgb_u8(0xca, 0x8a, 0x04),
    }
}

/// The polygon of a tile with the given look. The mesh is shared by all tiles of a kind.
fn polygon<S>(
    id: impl Into<bevy_composable_dragging::NodeId>,
    sides: u32,
    color: Color,
) -> ViewNode<S> {
    ViewNode::grad(
        id,
        Shape::arbitrary_polygon(vertices(sides), ShapeDetail::Medium),
    )
    .with_style(Style::shaded(color, 1.0))
}

/// A polygon with its outline: the outline is a larger polygon of the stroke color behind it.
fn outlined(id: &str, sides: u32, opacity: f32) -> [ViewNode<Board>; 2] {
    let apothem = SIDE / (2.0 * (PI / sides as f32).tan());
    let outline_scale = 1.0 + OUTLINE / apothem;
    [
        polygon(id.to_owned(), sides, fill(sides)).with_opacity(opacity),
        polygon(format!("{id}/outline"), sides, stroke(sides))
            .with_parent(id.to_owned())
            .with_opacity(opacity)
            .with_z(-0.01)
            .scaled_xy(Vec2::splat(outline_scale)),
    ]
}

/// The center of the palette item for a polygon: the bottoms of all items rest on one line.
fn palette_position(sides: u32) -> Vec2 {
    const BASELINE: f32 = 1.8;
    let x = match sides {
        3 => -5.0,
        4 => -3.2,
        6 => -1.1,
        _ => 1.4,
    };
    let lowest = vertices(sides)
        .iter()
        .map(|vertex| vertex.y)
        .fold(f32::MAX, f32::min);
    Vec2::new(x, BASELINE - lowest)
}

fn view(board: &Board) -> View<Board> {
    let mut view = View::new();
    view.push(
        ViewNode::rectangle("canvas", Vec2::new(12.0, CANVAS_HEIGHT))
            .with_color(Color::WHITE)
            .with_z(CANVAS_Z),
    );
    view.push(
        ViewNode::rectangle("palette", Vec2::new(9.6, 3.0))
            .at_xy(Vec2::new(-1.2, 3.0))
            .with_color(Color::srgb_u8(0xf9, 0xfa, 0xfb))
            .with_z(PALETTE_Z),
    );
    view.push(
        ViewNode::rectangle(TRASH, Vec2::splat(1.0))
            .at_xy(Vec2::new(5.3, 3.5))
            .with_color(Color::srgb_u8(0xff, 0xee, 0xee))
            .with_z(PALETTE_Z),
    );
    view.push(
        ViewNode::rectangle("trash-bin/lid", Vec2::new(0.5, 0.08))
            .with_parent(TRASH)
            .at_xy(Vec2::new(0.0, 0.26))
            .with_color(Color::srgb_u8(0x99, 0x99, 0x99))
            .with_z(0.1),
    );
    view.push(
        ViewNode::rectangle("trash-bin/body", Vec2::new(0.4, 0.5))
            .with_parent(TRASH)
            .at_xy(Vec2::new(0.0, -0.05))
            .with_color(Color::srgb_u8(0x99, 0x99, 0x99))
            .with_z(0.1),
    );

    for sides in KINDS {
        let (created, id) = board.with_new(sides, palette_position(sides));
        let [item, outline] = outlined(&format!("palette-{sides}"), sides, 0.8);
        view.push(
            item.at_xy(palette_position(sides))
                .with_z(PALETTE_ITEM_Z)
                .draggable(move || switch_to_state_and_follow(created.clone(), tile_id(id))),
        );
        view.push(outline);
    }

    for tile in &board.tiles {
        let id = tile.id;
        let snapshot = board.clone();
        let [fill, outline] = outlined(&tile_id(id), tile.sides, 1.0);
        view.push(
            fill.at_xy(tile.pos)
                .with_z(TILE_Z)
                .with_rotation_xy(tile.rot_deg.to_radians())
                .draggable(move || tile_spec(&snapshot, id)),
        );
        view.push(outline);
    }
    view
}


/// Run this example. Called by the WASM runner.
pub fn run(app: &mut App) {
    // A hexagon with a triangle and a square snapped to it, and a square waiting in the palette
    // to be dragged against the other side.
    let hexagon = Tile {
        id: 0,
        sides: 6,
        pos: Vec2::new(-1.0, -1.2),
        rot_deg: 0.0,
    };
    let triangle = compute_snaps(3, std::slice::from_ref(&hexagon), None)[0];
    let board = Board {
        tiles: vec![
            hexagon.clone(),
            Tile {
                id: 1,
                sides: 3,
                pos: triangle.pos,
                rot_deg: triangle.rot_deg,
            },
        ],
        next_id: 2,
    };
    let target = edges(&world_vertices(6, hexagon.pos, 0.0))[5].mid() + Vec2::new(-0.3, 0.2);
    app
        .add_plugins(DraggablePlugin::new(board, view))
        .add_plugins(ScriptedDragPlugin::straight(palette_position(4), target))
        .add_systems(Startup, |mut commands: Commands| {
            common::spawn_cameras(&mut commands, CANVAS_HEIGHT);
        })
        ;
}


