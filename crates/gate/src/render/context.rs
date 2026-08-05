use web_sys::{WebGlRenderingContext as GL, WebGlProgram, WebGlUniformLocation};
use std::collections::HashMap;
use pystral_core::animation::ActiveFSM;
use crate::render::state::{MovementTween, PropertyTween};
use crate::render::mesh::Mesh;
use web_sys::WebGlTexture;

pub struct TextureSet {
    pub front: WebGlTexture,
    pub mirrored: WebGlTexture,
}

pub struct UniformLocations {
    pub u_model: Option<WebGlUniformLocation>,
    pub u_view: Option<WebGlUniformLocation>,
    pub u_proj: Option<WebGlUniformLocation>,
    pub u_ambient_color: Option<WebGlUniformLocation>,
    pub u_ambient_intensity: Option<WebGlUniformLocation>,
    pub u_lights_dir: Vec<Option<WebGlUniformLocation>>,
    pub u_lights_color: Vec<Option<WebGlUniformLocation>>,
    pub u_lights_intensity: Vec<Option<WebGlUniformLocation>>,
    pub u_obj_color: Option<WebGlUniformLocation>,
    pub u_use_tex: Option<WebGlUniformLocation>,
    pub u_texture: Option<WebGlUniformLocation>,
    pub u_use_normal_map: Option<WebGlUniformLocation>,
    pub u_normal_map: Option<WebGlUniformLocation>,
    pub u_roughness: Option<WebGlUniformLocation>,
    pub u_metalness: Option<WebGlUniformLocation>,
    pub u_emissive: Option<WebGlUniformLocation>,
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
}

pub struct RenderContext {
    pub gl: GL,
    pub _program: WebGlProgram,
    pub sprite_mesh: Mesh,
    pub sphere_mesh: Mesh,
    pub cylinder_mesh: Mesh,
    pub unit_hex_mesh_cache: Option<Mesh>,
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
            u_model: gl.get_uniform_location(&program, "uModel"),
            u_view: gl.get_uniform_location(&program, "uView"),
            u_proj: gl.get_uniform_location(&program, "uProjection"),
            u_ambient_color: gl.get_uniform_location(&program, "uAmbientColor"),
            u_ambient_intensity: gl.get_uniform_location(&program, "uAmbientIntensity"),
            u_lights_dir: (0..4).map(|i| gl.get_uniform_location(&program, &format!("uLights[{}].direction", i))).collect(),
            u_lights_color: (0..4).map(|i| gl.get_uniform_location(&program, &format!("uLights[{}].color", i))).collect(),
            u_lights_intensity: (0..4).map(|i| gl.get_uniform_location(&program, &format!("uLights[{}].intensity", i))).collect(),
            u_obj_color: gl.get_uniform_location(&program, "uObjectColor"),
            u_use_tex: gl.get_uniform_location(&program, "uUseTexture"),
            u_texture: gl.get_uniform_location(&program, "uTexture"),
            u_use_normal_map: gl.get_uniform_location(&program, "uUseNormalMap"),
            u_normal_map: gl.get_uniform_location(&program, "uNormalMap"),
            u_roughness: gl.get_uniform_location(&program, "uRoughness"),
            u_metalness: gl.get_uniform_location(&program, "uMetalness"),
            u_emissive: gl.get_uniform_location(&program, "uEmissive"),
        };

        let attribs = AttribLocations {
            pos: gl.get_attrib_location(&program, "aPosition") as u32,
            norm: gl.get_attrib_location(&program, "aNormal") as u32,
            uv: gl.get_attrib_location(&program, "aUV") as u32,
        };

        Self {
            gl,
            _program: program,
            sprite_mesh,
            sphere_mesh,
            cylinder_mesh,
            unit_hex_mesh_cache: None,
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
