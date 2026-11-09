// Auto-generated wrapper for bevy_composable_dragging/lists_in_lists
#[path = "../../../../libs/bevy_composable_dragging/examples/common/mod.rs"]
mod common;

use bevy::prelude::*;
use bevy_composable_dragging::ops::{closest, state, vary};
use bevy_composable_dragging::{DragSpec, DraggablePlugin, View, ViewNode, param};

use common::screenshot::AutoScreenshotPlugin;
use common::script::ScriptedDragPlugin;

const TILE: f32 = 0.9;
const GAP: f32 = 0.15;
const PADDING: f32 = 0.15;
/// The width reserved at the left of a row for its grip.
const GRIP: f32 = 0.25;
/// Beyond this distance from every insertion point an item is free.
const FAR: f32 = 1.3;
/// Free positions are rounded to this grid when dropped.
const GRID: f32 = 0.25;
/// Ids from here on belong to the rows that wrap a lone free tile.
const WRAPPER: u16 = 100;

#[derive(Clone)]
enum Item {
    Tile(u16),
    Row(Row),
}

#[derive(Clone)]
struct Row {
    id: u16,
    items: Vec<Item>,
}

/// A row on the canvas, centered at `x`, `y`.
#[derive(Clone)]
struct Placed {
    row: Row,
    x: f32,
    y: f32,
}

#[derive(Clone)]
struct Canvas {
    rows: Vec<Placed>,
}

impl Item {
    fn id(&self) -> String {
        match self {
            Self::Tile(id) => format!("tile-{id}"),
            Self::Row(row) => format!("row-{}", row.id),
        }
    }
}

fn item_size(item: &Item) -> Vec2 {
    match item {
        Item::Tile(_) => Vec2::splat(TILE),
        Item::Row(row) => row_size(row),
    }
}

fn row_size(row: &Row) -> Vec2 {
    let sizes: Vec<Vec2> = row.items.iter().map(item_size).collect();
    let gaps = GAP * sizes.len().saturating_sub(1) as f32;
    let width = if sizes.is_empty() {
        TILE
    } else {
        sizes.iter().map(|size| size.x).sum::<f32>() + gaps
    };
    let height = sizes.iter().map(|size| size.y).fold(TILE, f32::max);
    Vec2::new(width + GRIP + 2.0 * PADDING, height + 2.0 * PADDING)
}

fn hue(id: u16) -> f32 {
    f32::from(id) * 53.0 % 360.0
}

/// Removes the item with `id` from anywhere in `items`.
fn take(items: &mut Vec<Item>, id: &str) -> Option<Item> {
    if let Some(index) = items.iter().position(|item| item.id() == id) {
        return Some(items.remove(index));
    }
    items.iter_mut().find_map(|item| match item {
        Item::Row(row) => take(&mut row.items, id),
        Item::Tile(_) => None,
    })
}

/// The canvas without the item `id`, and that item. A wrapper row left empty disappears.
fn without(canvas: &Canvas, id: &str) -> Option<(Canvas, Item)> {
    let mut canvas = canvas.clone();
    let item = if let Some(index) = canvas
        .rows
        .iter()
        .position(|placed| Item::Row(placed.row.clone()).id() == id)
    {
        Item::Row(canvas.rows.remove(index).row)
    } else {
        canvas
            .rows
            .iter_mut()
            .find_map(|placed| take(&mut placed.row.items, id))?
    };
    canvas
        .rows
        .retain(|placed| placed.row.id < WRAPPER || !placed.row.items.is_empty());
    Some((canvas, item))
}

fn row_ids(row: &Row, ids: &mut Vec<u16>) {
    ids.push(row.id);
    for item in &row.items {
        if let Item::Row(nested) = item {
            row_ids(nested, ids);
        }
    }
}

fn find_row(row: &mut Row, id: u16) -> Option<&mut Row> {
    if row.id == id {
        return Some(row);
    }
    row.items.iter_mut().find_map(|item| match item {
        Item::Row(nested) => find_row(nested, id),
        Item::Tile(_) => None,
    })
}

fn row_mut(canvas: &mut Canvas, id: u16) -> Option<&mut Row> {
    canvas
        .rows
        .iter_mut()
        .find_map(|placed| find_row(&mut placed.row, id))
}

/// Every canvas where `item` sits at some position of some row, at any depth.
fn placements(base: &Canvas, item: &Item) -> Vec<Canvas> {
    let mut ids = Vec::new();
    for placed in &base.rows {
        row_ids(&placed.row, &mut ids);
    }
    let mut placements = Vec::new();
    for id in ids {
        let mut probe = base.clone();
        let len = row_mut(&mut probe, id).map_or(0, |row| row.items.len());
        for slot in 0..=len {
            let mut placed = base.clone();
            if let Some(row) = row_mut(&mut placed, id) {
                row.items.insert(slot, item.clone());
                placements.push(placed);
            }
        }
    }
    placements
}

fn item_spec(canvas: &Canvas, id: &str) -> DragSpec<Canvas> {
    let Some((base, item)) = without(canvas, id) else {
        return state(canvas.clone());
    };
    let center = view(canvas)
        .world_position(id)
        .map_or(Vec2::ZERO, Vec3::truncate);
    let (row, at) = match &item {
        Item::Row(row) => (row.clone(), center),
        Item::Tile(tile) => (
            Row {
                id: WRAPPER + tile,
                items: vec![item.clone()],
            },
            center - Vec2::new(GRIP / 2.0, 0.0),
        ),
    };
    let mut free = base.clone();
    free.rows.push(Placed {
        row,
        x: at.x,
        y: at.y,
    });
    let index = free.rows.len() - 1;
    let free_spec = vary(
        free,
        [
            param!(Canvas, rows[index].x).with_scale(1.0),
            param!(Canvas, rows[index].y).with_scale(1.0),
        ],
    );
    let candidates = placements(&base, &item);
    if candidates.is_empty() {
        return free_spec;
    }
    let item_id = item.id();
    let size = item_size(&item);
    closest(candidates.into_iter().map(|placed| {
        let slot = view(&placed)
            .world_position(&item_id)
            .map_or(Vec2::ZERO, Vec3::truncate);
        state(placed).with_overlay(move |_| vec![insertion_marker(slot, size)])
    }))
    .with_floating()
    .when_far(free_spec, FAR)
    .on_drop_with(|canvas| {
        let mut canvas = canvas.clone();
        for placed in &mut canvas.rows {
            placed.x = (placed.x / GRID).round() * GRID;
            placed.y = (placed.y / GRID).round() * GRID;
        }
        canvas
    })
}

/// A bright bar at the left edge of the item that is about to be inserted.
fn insertion_marker(slot: Vec2, size: Vec2) -> ViewNode<Canvas> {
    ViewNode::rectangle("insertion-marker", Vec2::new(0.08, size.y + 0.1))
        .at(Vec3::new(slot.x - size.x / 2.0 - GAP / 2.0, slot.y, 1.0))
        .with_color(Color::srgb(1.0, 0.85, 0.3))
}

/// Positions `node` at `offset` from the center of its parent, or in the world without one.
fn place(
    node: ViewNode<Canvas>,
    parent: Option<(&str, Vec2)>,
    offset: Vec2,
    size: Vec2,
    z: f32,
) -> ViewNode<Canvas> {
    match parent {
        None => node.at_xy(offset).with_z(z).scaled_xy(size),
        Some((id, parent_size)) => node
            .with_parent(id.to_owned())
            .at_xy(offset / parent_size)
            .with_z(z)
            .scaled_xy(size / parent_size),
    }
}

fn emit(
    view: &mut View<Canvas>,
    canvas: &Canvas,
    item: &Item,
    parent: Option<(&str, Vec2)>,
    offset: Vec2,
    z: f32,
) {
    let size = item_size(item);
    let id = item.id();
    let key = id.clone();
    let snapshot = canvas.clone();
    let color = match item {
        Item::Tile(tile) => Color::hsl(hue(*tile), 0.6, 0.62),
        Item::Row(row) => Color::hsl(hue(row.id), 0.45, 0.84),
    };
    let node = place(
        ViewNode::rectangle(id.clone(), size),
        parent,
        offset,
        size,
        z,
    );
    view.push(
        node.with_color(color)
            .draggable(move || item_spec(&snapshot, &key)),
    );
    let Item::Row(row) = item else {
        return;
    };
    let grip = Vec2::new(0.1, size.y * 0.5);
    view.push(
        place(
            ViewNode::rectangle(format!("{id}-grip"), grip),
            Some((&id, size)),
            Vec2::new(-size.x / 2.0 + PADDING + GRIP / 2.0, 0.0),
            grip,
            0.05,
        )
        .with_color(Color::hsl(hue(row.id), 0.4, 0.55)),
    );
    let mut x = -size.x / 2.0 + PADDING + GRIP;
    for child in &row.items {
        let width = item_size(child).x;
        emit(
            view,
            canvas,
            child,
            Some((&id, size)),
            Vec2::new(x + width / 2.0, 0.0),
            0.1,
        );
        x += width + GAP;
    }
}

fn view(canvas: &Canvas) -> View<Canvas> {
    let mut view = View::new();
    let mut z = 0.0;
    for placed in &canvas.rows {
        emit(
            &mut view,
            canvas,
            &Item::Row(placed.row.clone()),
            None,
            Vec2::new(placed.x, placed.y),
            z,
        );
        z += 0.5;
    }
    view
}

fn initial_canvas() -> Canvas {
    let tiles = |ids: &[u16]| ids.iter().copied().map(Item::Tile).collect::<Vec<_>>();
    let mut first = tiles(&[1, 2]);
    first.push(Item::Row(Row {
        id: 10,
        items: tiles(&[3, 4]),
    }));
    Canvas {
        rows: vec![
            Placed {
                row: Row {
                    id: 1,
                    items: first,
                },
                x: -2.5,
                y: 1.5,
            },
            Placed {
                row: Row {
                    id: 2,
                    items: tiles(&[5, 6, 7]),
                },
                x: 3.0,
                y: -1.5,
            },
            Placed {
                row: Row {
                    id: WRAPPER + 8,
                    items: tiles(&[8]),
                },
                x: 3.5,
                y: 1.75,
            },
        ],
    }
}


/// Run this example. Called by the WASM runner.
pub fn run(app: &mut App) {
    app
        .add_plugins(DraggablePlugin::new(initial_canvas(), view))
        .add_plugins(ScriptedDragPlugin::straight(
            Vec2::new(-4.2, 1.5),
            Vec2::new(-1.7, 1.45),
        ))
        .add_systems(Startup, |mut commands: Commands| {
            common::spawn_cameras(&mut commands, 7.0);
        })
        ;
}


