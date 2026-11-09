// Auto-generated wrapper for bevy_composable_dragging/cozy_castle
#[path = "../../../../libs/bevy_composable_dragging/examples/common/mod.rs"]
mod common;
//! A cozy castle builder, drawn with gradient meshes from `bevy_gradmesh`.
//!
//! Pieces are chosen from a radial menu and dragged across the ground, and every position along
//! the drag is evaluated speculatively, so what the pointer would do is on screen while it is
//! still down:
//!
//! - A piece carried onto another one stands on it, and the one underneath changes in the
//!   preview: it gives up its roof for a platform and thickens at the base.
//! - An arch gate carried onto a swept wall opens an arch in it, and dropping it makes the
//!   opening permanent.
//! - The Scale tool of a selected wall shows a gizmo whose handles stretch the wall and its
//!   openings.
//!
//! The animation of the README is a scripted tour of these three, played as real pointer input:
//! run it with `DSD_SCREENSHOT=1`.
//!
//! Run with `cargo run --example cozy_castle_3d --features shapes-3d`.

use bevy::camera::visibility::RenderLayers;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy_composable_dragging::gizmo::GIZMO_LAYER;
use bevy_composable_dragging::hud::{HudPlugin, Layout, Screen, Theme};
use bevy_composable_dragging::ops::across_plane;
use bevy_composable_dragging::radial::{RadialHost, RadialMenuPlugin, petal_id, push_menu};
use bevy_composable_dragging::{DragModel, DragSpace, DragSpec, DraggablePlugin, View, ViewNode};
use bevy_gradmesh::{Shape, ShapeDetail};

mod gate;
mod kit;
mod menu;
mod select;
mod stack;
mod wall;

use common::screenshot::AutoScreenshotPlugin;
use common::tour::{Point, Tour, TourPlugin, through_camera};
use common::{HUD_LAYER, HudCamera, SceneCamera, spawn_hud_camera};
use kit::{Fabric, build_parts};
use menu::{Item, Ring, Tool, ground_spec};
use select::{select_spec, wall_gizmo};
use wall::Wall;

/// How many world units the HUD camera shows vertically.
const HUD_HEIGHT: f32 = 720.0;
/// The aspect ratio of the window the castle starts in.
const ASPECT: f32 = 16.0 / 9.0;
/// Footprints that overlap by less than this stand beside each other instead of stacking.
const OVERLAP_EPSILON: f32 = 0.02;
/// How far a piece that is carried is lifted above what it rests on, so it is seen to float.
const CARRY_LIFT: f32 = 0.35;
/// The width and depth of the ground.
const GROUND: Vec2 = Vec2::new(60.0, 40.0);
/// The longest the animation of the README may be, in frames at 30 frames per second.
const MAX_FRAMES: u32 = 660;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum PieceKind {
    Tower,
    Keep,
    ArchGate,
}

impl PieceKind {
    /// Half the width and depth of the footprint of the piece, around its center.
    fn half_extents(self) -> Vec2 {
        match self {
            Self::Tower => Vec2::splat(0.4),
            Self::Keep => Vec2::splat(0.45),
            Self::ArchGate => Vec2::new(0.5, 0.15),
        }
    }

    /// How far the piece reaches above its base, which is what other pieces stack on.
    fn top(self) -> f32 {
        match self {
            Self::Tower => 1.76,
            Self::Keep => 1.5,
            Self::ArchGate => 0.82,
        }
    }
}

#[derive(Clone, Debug)]
struct Placed {
    id: u32,
    kind: PieceKind,
    /// The center of the base of the piece.
    position: Vec3,
}

impl Placed {
    /// The center of the piece on the ground, in the `(x, z)` plane.
    fn ground(&self) -> Vec2 {
        Vec2::new(self.position.x, self.position.z)
    }

    /// Whether the footprints of two pieces overlap by more than a little.
    fn overlaps(&self, other: &Self) -> bool {
        let reach = self.kind.half_extents() + other.kind.half_extents();
        (self.ground() - other.ground())
            .abs()
            .cmplt(reach - Vec2::splat(OVERLAP_EPSILON))
            .all()
    }
}

/// A wall of the castle.
#[derive(Clone, Debug)]
struct WallPiece {
    id: u32,
    wall: Wall,
}

#[derive(Clone)]
struct Castle {
    placed: Vec<Placed>,
    walls: Vec<WallPiece>,
    /// The piece or wall that is selected.
    selected: Option<u32>,
    /// The ring of the radial menu that is open.
    menu: Option<Ring>,
    /// The piece the next drag across the ground places.
    armed: Option<PieceKind>,
    tool: Tool,
    next_id: u32,
    /// The size of the HUD and the projection of the scene camera, which the HUD lays itself
    /// out from.
    screen: Screen,
}

impl Castle {
    fn empty() -> Self {
        Self {
            placed: Vec::new(),
            walls: Vec::new(),
            selected: None,
            menu: None,
            armed: None,
            tool: Tool::Select,
            next_id: 0,
            screen: Screen::for_camera(HUD_HEIGHT, scene_camera(), ASPECT),
        }
    }

    /// The castle the example starts with: a keep behind a swept wall, to build on.
    fn tour() -> Self {
        let mut castle = Self::empty().with_wall(Wall::through(&[
            Vec3::new(-3.2, 0.0, 0.4),
            Vec3::new(-1.4, 0.0, 0.0),
            Vec3::new(1.4, 0.0, 0.0),
            Vec3::new(3.2, 0.0, 0.4),
        ]));
        castle.add(PieceKind::Keep, KEEP.x, KEEP.y);
        castle
    }

    fn piece(&self, id: u32) -> Option<&Placed> {
        self.placed.iter().find(|piece| piece.id == id)
    }

    fn wall(&self, id: u32) -> Option<&Wall> {
        self.walls
            .iter()
            .find(|piece| piece.id == id)
            .map(|piece| &piece.wall)
    }

    /// The castle with `wall` added.
    fn with_wall(&self, wall: Wall) -> Self {
        let mut castle = self.clone();
        castle.walls.push(WallPiece {
            id: castle.next_id,
            wall,
        });
        castle.next_id += 1;
        castle
    }

    /// The castle without piece `id`.
    fn without(&self, id: u32) -> Self {
        let mut castle = self.clone();
        castle.placed.retain(|piece| piece.id != id);
        if castle.selected == Some(id) {
            castle.selected = None;
        }
        castle
    }

    /// The top of the highest piece under the footprint of piece `id` at `at`, or the ground.
    fn rest_height(&self, id: u32, at: Vec2) -> f32 {
        let Some(piece) = self.piece(id) else {
            return 0.0;
        };
        let probe = Placed {
            position: Vec3::new(at.x, 0.0, at.y),
            ..piece.clone()
        };
        self.placed
            .iter()
            .filter(|other| other.id != id && probe.overlaps(other))
            .map(|other| other.position.y + other.kind.top())
            .fold(0.0, f32::max)
    }

    /// The castle with piece `id` standing at `at` on the ground plane, resting on whatever is
    /// under it.
    fn posed(&self, id: u32, at: Vec2) -> Self {
        let rest = self.rest_height(id, at);
        let mut castle = self.clone();
        if let Some(piece) = castle.placed.iter_mut().find(|piece| piece.id == id) {
            piece.position = Vec3::new(at.x, rest, at.y);
        }
        castle
    }

    fn add(&mut self, kind: PieceKind, x: f32, z: f32) {
        let id = self.next_id;
        self.next_id += 1;
        self.placed.push(Placed {
            id,
            kind,
            position: Vec3::ZERO,
        });
        *self = self.posed(id, Vec2::new(x, z));
    }
}

fn piece_id(id: u32) -> String {
    format!("piece-{id}")
}

/// The space of every drag across the ground.
fn ground_space() -> DragSpace {
    DragSpace::plane(Vec3::ZERO, Dir3::Y)
}

/// Carries the piece across the ground, a little lifted so it is seen to float. Where it is
/// carried onto another piece it rests on it.
fn carried(id: u32) -> DragSpec<Castle> {
    let node = piece_id(id);
    DragSpec::from(across_plane(move |castle: &Castle, offset: Vec3| {
        castle.moved(id, offset)
    }))
    .change_result(move |mut frame| {
        if let Some(dragged) = frame.view.get_mut(&node) {
            dragged.transform.translation.y += CARRY_LIFT;
        }
        frame
    })
}

/// The drag of a placed piece: carry it across the ground. Releasing selects the piece, so a
/// press without any movement selects it too, and a gate that is released on a wall opens it.
fn move_spec(id: u32) -> DragSpec<Castle> {
    carried(id).change_result(move |mut frame| {
        frame.drop_state = frame.drop_state.with_selected(Some(id)).gate_dropped(id);
        frame
    })
}

/// The drag of piece or wall `id`: pieces are carried, walls are selected.
fn scene_spec(castle: &Castle, id: u32) -> DragSpec<Castle> {
    if castle.piece(id).is_some() {
        move_spec(id)
    } else {
        select_spec(castle, id)
    }
}

/// Adds the nodes of a piece to `view`: the draggable root, and its parts as children.
fn push_piece(view: &mut View<Castle>, placed: &Placed, castle: &Castle) {
    let mut parts = build_parts(placed.kind, castle.variant(placed.id)).into_iter();
    let body = parts.next().expect("every piece has a body");
    let lift = body.transform.translation.y;
    let root = piece_id(placed.id);
    let id = placed.id;
    let mut transform = Transform::from_translation(placed.position);
    transform.translation.y += lift;
    view.push(
        ViewNode::grad(root.clone(), body.shape)
            .with_style(body.fabric.style())
            .with_transform(transform)
            .draggable_by_in(ground_space(), move |castle| scene_spec(castle, id)),
    );
    for (index, part) in parts.enumerate() {
        let mut local = part.transform;
        local.translation.y -= lift;
        view.push(
            ViewNode::grad(format!("{root}/{index}"), part.shape)
                .with_style(part.fabric.style())
                .with_transform(local)
                .with_parent(root.clone()),
        );
    }
}

/// Adds a wall to `view`, open under the gates over it. Pressing it selects it, and while it is
/// selected with the Scale tool it has the gizmo that scales it.
fn push_wall(view: &mut View<Castle>, piece: &WallPiece, castle: &Castle) {
    let id = piece.id;
    let shown = castle.wall_shown(&piece.wall);
    let base = Transform::from_translation(piece.wall.pivot);
    view.push(
        ViewNode::grad(piece_id(id), shown.shape())
            .with_transform(base)
            .with_style(Fabric::Stone.style())
            .draggable_by_in(ground_space(), move |castle| scene_spec(castle, id)),
    );
    if castle.selected == Some(id) {
        wall_gizmo(id, &piece.wall, base).add_if_revealed(view, castle);
    }
}

fn view(castle: &Castle) -> View<Castle> {
    let mut view = View::new();
    view.push(
        ViewNode::grad(
            "ground",
            Shape::cuboid(Vec3::new(GROUND.x, 0.2, GROUND.y), 0.05, ShapeDetail::Low),
        )
        .at(Vec3::new(0.0, -0.1, 0.0))
        .with_style(Fabric::Ground.style())
        .draggable_by_in(ground_space(), ground_spec),
    );
    for piece in &castle.walls {
        push_wall(&mut view, piece, castle);
    }
    for placed in &castle.placed {
        push_piece(&mut view, placed, castle);
    }
    push_menu(&mut view, castle, &Layout::new(&castle.screen), &theme());
    view
}

/// The look of the HUD: the library theme, drawn on the layer of the HUD camera.
fn theme() -> Theme {
    Theme::new(HUD_LAYER)
}

/// Where the scene camera stands and what it looks at.
fn scene_camera() -> Transform {
    Transform::from_xyz(0.0, 5.0, 6.4).looking_at(Vec3::new(0.0, 0.9, -0.6), Vec3::Y)
}

fn setup(mut commands: Commands) {
    spawn_hud_camera(&mut commands, HUD_HEIGHT);
    let layers = RenderLayers::from_layers(&[0, GIZMO_LAYER]);
    commands.spawn((SceneCamera, Camera3d::default(), layers, scene_camera()));
    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            color: Color::srgb(1.0, 0.93, 0.8),
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 3,
            minimum_distance: 4.0,
            maximum_distance: 26.0,
            first_cascade_far_bound: 12.0,
            overlap_proportion: 0.2,
        }
        .build(),
        Transform::from_xyz(-5.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.75, 0.8, 1.0),
        brightness: 280.0,
        ..default()
    });
}

/// Adds the castle to `app`: everything but the windowing and rendering plugins.
fn castle(app: &mut App) -> &mut App {
    app.insert_resource(ClearColor(Color::srgb(0.16, 0.2, 0.34)))
        .add_plugins(
            DraggablePlugin::new(Castle::tour(), view)
                .with_transition_duration(theme().tween_seconds),
        )
        .add_plugins(HudPlugin::<Castle>::new(HUD_HEIGHT).camera::<SceneCamera>())
        .add_plugins(RadialMenuPlugin::<Castle>::new(theme()))
        .add_systems(Startup, setup)
}

/// Where the keep of the starting castle stands on the ground.
const KEEP: Vec2 = Vec2::new(-2.2, -1.5);
/// Where on the wall the tour drops the gate.
const GATE: Vec2 = Vec2::new(-1.0, 0.0);
/// How far, in world units, the tour pulls the handle that stretches the wall.
const SCALE_PULL: f32 = 2.2;

/// The pixel where the camera of the scene shows the point `(x, 0, z)` of the ground.
fn ground(x: f32, z: f32) -> Point {
    through_camera::<SceneCamera>(move |_| Some(Vec3::new(x, 0.0, z)))
}

/// The pixel where the camera of the HUD shows the HUD node `id` of the view.
fn hud(id: String) -> Point {
    through_camera::<HudCamera>(move |world| {
        let at = world
            .resource::<DragModel<Castle>>()
            .view()
            .world_position(&id)?;
        Some(at.truncate().extend(0.0))
    })
}

/// The pixel `offset` away, in the world, from where the camera of the scene shows the node `id`.
fn scene_node(id: String, offset: Vec3) -> Point {
    through_camera::<SceneCamera>(move |world| {
        let at = world
            .resource::<DragModel<Castle>>()
            .view()
            .world_position(&id)?;
        Some(at + offset)
    })
}

/// The id of the hub that is in use: the one over the selection, or else the one that rests in
/// its corner.
fn hub_id(castle: &Castle) -> &'static str {
    if castle.is_popped() {
        "menu/pop"
    } else {
        "menu/hub"
    }
}

/// The pixel of the hub that is in use.
fn hub() -> Point {
    through_camera::<HudCamera>(|world| {
        let model = world.resource::<DragModel<Castle>>();
        let at = model.view().world_position(hub_id(model.state()))?;
        Some(at.truncate().extend(0.0))
    })
}

/// The pixel of the petal `item` of `ring`.
fn petal(ring: Ring, item: Item) -> Point {
    hud(petal_id::<Castle>(&ring, &item))
}

/// The tour: open the menu and carry a tower onto the keep, carry an arch gate onto the wall,
/// then scale the wall. Every step is pointer input, so it goes through the real drag engine.
fn tour() -> Tour {
    let handle = format!("{}/gizmo/scale-x", piece_id(0));
    Tour::new()
        .wait(0.5)
        .click(hub())
        .wait(0.5)
        .click(petal(Ring::Root, Item::Piece(PieceKind::Tower)))
        .wait(0.3)
        .glide(ground(1.2, -2.8))
        .press()
        .glide(ground(KEEP.x, KEEP.y))
        .wait(1.3)
        .release()
        .wait(0.7)
        .click(hub())
        .wait(0.5)
        .click(petal(Ring::Root, Item::Piece(PieceKind::ArchGate)))
        .wait(0.3)
        .glide(ground(GATE.x, GATE.y + 1.8))
        .press()
        .glide(ground(GATE.x, GATE.y))
        .wait(1.3)
        .release()
        .wait(0.7)
        .click(hub())
        .wait(0.4)
        .click(petal(Ring::Root, Item::Open(Ring::Tools)))
        .wait(0.4)
        .click(petal(Ring::Tools, Item::Tool(Tool::Scale)))
        .wait(0.4)
        .glide(scene_node(handle.clone(), Vec3::ZERO))
        .press()
        .glide(scene_node(handle, Vec3::X * SCALE_PULL))
        .wait(0.5)
        .release()
        .wait(1.0)
}

/// Spawns the camera that draws the title banner of the capture above everything else.
fn spawn_banner_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        RenderLayers::layer(common::OVERLAY_LAYER),
        Camera {
            order: 2,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 100.0),
    ));
}


/// Run this example. Called by the WASM runner.
pub fn run(app: &mut App) {
    app
let mut app = App::new();
    app.add_plugins(common::default_plugins("cozy_castle"));
    castle(&mut app)
        .add_systems(Startup, spawn_banner_camera)
        .add_plugins(TourPlugin::new(tour))
        .add_plugins(
            AutoScreenshotPlugin::new("screenshot_cozy_castle.webp")
                .with_title("cozy_castle")
                .with_description("menu, stacks, arches")
                .animation(MAX_FRAMES),
        )
        ;
}


