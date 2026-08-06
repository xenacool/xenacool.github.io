use web_sys::{WebGlRenderingContext as GL, WebGlProgram, WebGlUniformLocation};
use std::collections::HashMap;
use hexx::HexOrientation;
use pystral_core::animation::ActiveFSM;
use crate::render::state::{MovementTween, PropertyTween};
use crate::render::mesh::Mesh;
use web_sys::WebGlTexture;

pub struct TextureSet {
    pub front: WebGlTexture,
    pub mirrored: WebGlTexture,
}

pub struct UniformLocations {
    pub model: Option<WebGlUniformLocation>,
    pub view: Option<WebGlUniformLocation>,
    pub proj: Option<WebGlUniformLocation>,
    pub ambient_color: Option<WebGlUniformLocation>,
    pub ambient_intensity: Option<WebGlUniformLocation>,
    pub lights_dir: Vec<Option<WebGlUniformLocation>>,
    pub lights_color: Vec<Option<WebGlUniformLocation>>,
    pub lights_intensity: Vec<Option<WebGlUniformLocation>>,
    pub obj_color: Option<WebGlUniformLocation>,
    pub use_tex: Option<WebGlUniformLocation>,
    pub texture: Option<WebGlUniformLocation>,
    pub use_normal_map: Option<WebGlUniformLocation>,
    pub normal_map: Option<WebGlUniformLocation>,
    pub roughness: Option<WebGlUniformLocation>,
    pub metalness: Option<WebGlUniformLocation>,
    pub emissive: Option<WebGlUniformLocation>,
}

pub struct AttribLocations {
    pub pos: u32,
    pub norm: u32,
    pub uv: u32,
}

pub struct SpritestackTextures {
    pub color_textures: Vec<WebGlTexture>,
    pub normal_textures: Vec<WebGlTexture>,
    pub width: u32,
    pub height: u32,
    pub spacing: f32,
    pub aabb: glam::Vec3,
}

pub struct RenderContext {
    pub gl: GL,
    pub program: WebGlProgram,
    pub sprite_mesh: Mesh,
    pub sphere_mesh: Mesh,
    pub cylinder_mesh: Mesh,
    pub unit_hex_mesh_cache: Option<Mesh>,
    pub cached_hex_orientation: Option<HexOrientation>,
    pub active_fsms: HashMap<u64, ActiveFSM>,
    pub movement_tweens: HashMap<u64, MovementTween>,
    pub property_tweens: HashMap<(u64, String), PropertyTween>,
    pub last_index: Option<usize>,
    pub uniforms: UniformLocations,
    pub attribs: AttribLocations,
    pub sprite_part_textures: HashMap<(u64, usize), (Vec<pystral_core::domain::PainterCommand>, TextureSet)>,
    pub bone_textures: HashMap<(u64, usize), (Vec<pystral_core::domain::PainterCommand>, TextureSet)>,
    pub spritestack_assets: HashMap<String, SpritestackTextures>,
    pub asset_collection_cache: HashMap<String, pystral_compiler::assets::AssetCollection>,
    pub active_camera_id: Option<u64>,
    pub next_seq: u64,
}

impl RenderContext {
    pub fn new(gl: GL, program: WebGlProgram, sprite_mesh: Mesh, sphere_mesh: Mesh, cylinder_mesh: Mesh) -> Self {
        let uniforms = UniformLocations {
            model: gl.get_uniform_location(&program, "uModel"),
            view: gl.get_uniform_location(&program, "uView"),
            proj: gl.get_uniform_location(&program, "uProjection"),
            ambient_color: gl.get_uniform_location(&program, "uAmbientColor"),
            ambient_intensity: gl.get_uniform_location(&program, "uAmbientIntensity"),
            lights_dir: (0..4).map(|i| gl.get_uniform_location(&program, &format!("uLights[{}].direction", i))).collect(),
            lights_color: (0..4).map(|i| gl.get_uniform_location(&program, &format!("uLights[{}].color", i))).collect(),
            lights_intensity: (0..4).map(|i| gl.get_uniform_location(&program, &format!("uLights[{}].intensity", i))).collect(),
            obj_color: gl.get_uniform_location(&program, "uObjectColor"),
            use_tex: gl.get_uniform_location(&program, "uUseTexture"),
            texture: gl.get_uniform_location(&program, "uTexture"),
            use_normal_map: gl.get_uniform_location(&program, "uUseNormalMap"),
            normal_map: gl.get_uniform_location(&program, "uNormalMap"),
            roughness: gl.get_uniform_location(&program, "uRoughness"),
            metalness: gl.get_uniform_location(&program, "uMetalness"),
            emissive: gl.get_uniform_location(&program, "uEmissive"),
        };

        let attribs = AttribLocations {
            pos: gl.get_attrib_location(&program, "aPosition") as u32,
            norm: gl.get_attrib_location(&program, "aNormal") as u32,
            uv: gl.get_attrib_location(&program, "aUV") as u32,
        };

        Self {
            gl,
            program,
            sprite_mesh,
            sphere_mesh,
            cylinder_mesh,
            unit_hex_mesh_cache: None,
            cached_hex_orientation: None,
            active_fsms: HashMap::new(),
            movement_tweens: HashMap::new(),
            property_tweens: HashMap::new(),
            last_index: None,
            uniforms,
            attribs,
            sprite_part_textures: HashMap::new(),
            bone_textures: HashMap::new(),
            spritestack_assets: HashMap::new(),
            asset_collection_cache: HashMap::new(),
            active_camera_id: None,
            next_seq: 1,
        }
    }
}
