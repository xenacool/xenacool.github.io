use serde::{Serialize, Deserialize};
use hexx::{HexLayout, HexOrientation, Hex};
use glam::Vec2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GridOrientation {
    Pointy,
    Flat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Material {
    pub color: [f32; 3],
    pub roughness: f32,
    pub metalness: f32,
    pub emissive: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HexTile {
    pub hex: Hex,
    pub bottom: f32,
    pub height: f32,
    pub material: String, // Reference to a material definition
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HexMap {
    pub orientation: GridOrientation,
    pub hex_size: Vec2,
    pub tiles: Vec<HexTile>,
}

impl HexMap {
    pub fn new() -> Self {
        Self {
            orientation: GridOrientation::Pointy,
            hex_size: Vec2::splat(1.0),
            tiles: Vec::new(),
        }
    }

    pub fn layout(&self) -> HexLayout {
        HexLayout {
            orientation: match self.orientation {
                GridOrientation::Pointy => HexOrientation::Pointy,
                GridOrientation::Flat => HexOrientation::Flat,
            },
            origin: Vec2::ZERO,
            scale: self.hex_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HexGrid {
    pub orientation: GridOrientation,
    pub hex_size: Vec2,
    pub radius: u32,
}

impl HexGrid {
    pub fn new(radius: u32) -> Self {
        Self {
            orientation: GridOrientation::Pointy,
            hex_size: Vec2::splat(1.0),
            radius,
        }
    }

    pub fn layout(&self) -> HexLayout {
        HexLayout {
            orientation: match self.orientation {
                GridOrientation::Pointy => HexOrientation::Pointy,
                GridOrientation::Flat => HexOrientation::Flat,
            },
            origin: Vec2::ZERO,
            scale: self.hex_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Light {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capsule {
    pub radius: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Shape3D {
    Capsule(Capsule),
    Cube(f32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Joint {
    Property(String),
    Constant(f32, f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PainterCommand {
    MoveTo(f32, f32, f32),
    LineTo(f32, f32, f32),
    QuadTo(f32, f32, f32, f32, f32, f32),
    CubicTo(f32, f32, f32, f32, f32, f32, f32, f32, f32),
    Close,
    SetColor([f32; 4], [f32; 4]), // Front, Mirrored
    SetStrokeWidth(f32),
    Fill,
    Stroke,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bone {
    pub start: Joint,
    pub end: Joint,
    pub painter_commands: Vec<PainterCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpritestackSlice {
    pub color_data: Vec<u8>,  // RGBA
    pub normal_data: Vec<u8>, // RGBA (packed Nx, Ny, Nz, 1.0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Spritestack {
    pub width: u32,
    pub height: u32,
    pub spacing: f32,
    pub slices: Vec<SpritestackSlice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpritePart {
    pub x_prop: String,
    pub y_prop: String,
    pub z_prop: String,
    pub rotation_prop: Option<String>,
    pub color: [f32; 3],
    pub scale: f32,
    pub painter_commands: Vec<PainterCommand>,
    pub spritestack: Option<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightingConfig {
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    pub lights: Vec<Light>,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            ambient_color: [1.0, 1.0, 1.0],
            ambient_intensity: 0.1,
            lights: vec![Light {
                direction: [-1.0, -2.0, -1.0],
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            }],
        }
    }
}
