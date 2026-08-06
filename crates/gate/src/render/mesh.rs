use web_sys::{WebGlRenderingContext as GL, WebGlBuffer, js_sys};
use glam::{Vec3, Vec2};
use hexx::MeshInfo;

pub struct Mesh {
    pub vertex_buffer: WebGlBuffer,
    pub index_buffer: WebGlBuffer,
    pub wire_index_buffer: WebGlBuffer,
    pub index_count: i32,
    pub wire_index_count: i32,
}

impl Mesh {
    pub fn from_mesh_info(gl: &GL, info: &MeshInfo) -> Self {
        let mut interleaved_data = Vec::with_capacity(info.vertices.len() * 8);
        for i in 0..info.vertices.len() {
            interleaved_data.push(info.vertices[i].x);
            interleaved_data.push(info.vertices[i].y);
            interleaved_data.push(info.vertices[i].z);
            interleaved_data.push(info.normals[i].x);
            interleaved_data.push(info.normals[i].y);
            interleaved_data.push(info.normals[i].z);
            interleaved_data.push(info.uvs[i].x);
            interleaved_data.push(info.uvs[i].y);
        }

        let vertex_buffer = gl.create_buffer().expect("Failed to create vertex buffer");
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vertex_buffer));
        unsafe {
            let view = js_sys::Float32Array::view(&interleaved_data);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::STATIC_DRAW);
        }

        let index_buffer = gl.create_buffer().expect("Failed to create index buffer");
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&index_buffer));
        unsafe {
            let view = js_sys::Uint16Array::view(&info.indices);
            gl.buffer_data_with_array_buffer_view(GL::ELEMENT_ARRAY_BUFFER, &view, GL::STATIC_DRAW);
        }

        // Generate wireframe indices (edges of triangles)
        let mut wire_indices = Vec::new();
        for chunk in info.indices.chunks(3) {
            if chunk.len() == 3 {
                wire_indices.push(chunk[0]);
                wire_indices.push(chunk[1]);
                wire_indices.push(chunk[1]);
                wire_indices.push(chunk[2]);
                wire_indices.push(chunk[2]);
                wire_indices.push(chunk[0]);
            }
        }

        let wire_index_buffer = gl.create_buffer().expect("Failed to create wireframe index buffer");
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&wire_index_buffer));
        unsafe {
            let view = js_sys::Uint16Array::view(&wire_indices);
            gl.buffer_data_with_array_buffer_view(GL::ELEMENT_ARRAY_BUFFER, &view, GL::STATIC_DRAW);
        }

        Self {
            vertex_buffer,
            index_buffer,
            wire_index_buffer,
            #[allow(clippy::cast_possible_wrap)]
            index_count: info.indices.len() as i32,
            #[allow(clippy::cast_possible_wrap)]
            wire_index_count: wire_indices.len() as i32,
        }
    }

    pub fn destroy(&self, gl: &GL) {
        gl.delete_buffer(Some(&self.vertex_buffer));
        gl.delete_buffer(Some(&self.index_buffer));
        gl.delete_buffer(Some(&self.wire_index_buffer));
    }

    pub fn draw(&self, gl: &GL, pos_loc: u32, norm_loc: u32, uv_loc: u32) {
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vertex_buffer));
        
        gl.vertex_attrib_pointer_with_i32(pos_loc, 3, GL::FLOAT, false, 32, 0);
        gl.enable_vertex_attrib_array(pos_loc);

        gl.vertex_attrib_pointer_with_i32(norm_loc, 3, GL::FLOAT, false, 32, 12);
        gl.enable_vertex_attrib_array(norm_loc);

        gl.vertex_attrib_pointer_with_i32(uv_loc, 2, GL::FLOAT, false, 32, 24);
        gl.enable_vertex_attrib_array(uv_loc);

        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&self.index_buffer));
        gl.draw_elements_with_i32(GL::TRIANGLES, self.index_count, GL::UNSIGNED_SHORT, 0);
    }

    pub fn draw_wireframe(&self, gl: &GL, pos_loc: u32) {
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vertex_buffer));
        gl.vertex_attrib_pointer_with_i32(pos_loc, 3, GL::FLOAT, false, 32, 0);
        gl.enable_vertex_attrib_array(pos_loc);

        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&self.wire_index_buffer));
        gl.draw_elements_with_i32(GL::LINES, self.wire_index_count, GL::UNSIGNED_SHORT, 0);
    }
}

pub fn create_sprite_mesh(gl: &GL) -> Mesh {
    let vertices = vec![
        Vec3::new(-0.5, -0.5, 0.0),
        Vec3::new(0.5, -0.5, 0.0),
        Vec3::new(0.5, 0.5, 0.0),
        Vec3::new(-0.5, 0.5, 0.0),
    ];
    let normals = vec![Vec3::Z; 4];
    let uvs = vec![
        Vec2::new(0.005, 0.995),
        Vec2::new(0.995, 0.995),
        Vec2::new(0.995, 0.005),
        Vec2::new(0.005, 0.005),
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];

    Mesh::from_mesh_info(gl, &MeshInfo {
        vertices,
        normals,
        uvs,
        indices,
    })
}

pub fn create_sphere_mesh(gl: &GL, sectors: u32, stacks: u32) -> Mesh {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices: Vec<u16> = Vec::new();

    let sector_step = 2.0 * std::f32::consts::PI / sectors as f32;
    let stack_step = std::f32::consts::PI / stacks as f32;

    for i in 0..=stacks {
        let stack_angle = std::f32::consts::PI / 2.0 - i as f32 * stack_step;
        let xy = stack_angle.cos();
        let z = stack_angle.sin();

        for j in 0..=sectors {
            let sector_angle = j as f32 * sector_step;
            let x = xy * sector_angle.cos();
            let y = xy * sector_angle.sin();
            
            vertices.push(Vec3::new(x, y, z));
            normals.push(Vec3::new(x, y, z));
            uvs.push(Vec2::new(j as f32 / sectors as f32, i as f32 / stacks as f32));
        }
    }

    for i in 0..stacks {
        let k1 = i * (sectors + 1);
        let k2 = k1 + sectors + 1;
        for j in 0..sectors {
            if i != 0 {
                indices.push((k1 + j) as u16);
                indices.push((k2 + j) as u16);
                indices.push((k1 + j + 1) as u16);
            }
            if i != (stacks - 1) {
                indices.push((k1 + j + 1) as u16);
                indices.push((k2 + j) as u16);
                indices.push((k2 + j + 1) as u16);
            }
        }
    }

    Mesh::from_mesh_info(gl, &MeshInfo { vertices, normals, uvs, indices })
}

pub fn create_cylinder_mesh(gl: &GL, sectors: u32) -> Mesh {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices: Vec<u16> = Vec::new();

    let sector_step = 2.0 * std::f32::consts::PI / sectors as f32;

    for i in 0..=1 {
        let h = i as f32 - 0.5;
        for j in 0..=sectors {
            let angle = j as f32 * sector_step;
            let x = angle.cos();
            let y = angle.sin();
            
            vertices.push(Vec3::new(x, y, h));
            normals.push(Vec3::new(x, y, 0.0));
            uvs.push(Vec2::new(j as f32 / sectors as f32, i as f32));
        }
    }

    for j in 0..sectors {
        let k1 = j as u16;
        let k2 = (j + sectors + 1) as u16;
        indices.push(k1);
        indices.push(k2);
        indices.push(k1 + 1);
        
        indices.push(k1 + 1);
        indices.push(k2);
        indices.push(k2 + 1);
    }

    Mesh::from_mesh_info(gl, &MeshInfo { vertices, normals, uvs, indices })
}
