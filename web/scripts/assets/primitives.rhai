// Procedural primitive assets shared by the multifile pg_rpg scenario. Pixel
// generation is intentionally script-owned; Rust only accepts the generic
// RGBA/normal spritestack payload and serializes it for the renderer.
let atlas = load_atlas(atlas_json);
let collection = new_asset_collection();

fn byte(color, index) {
    color[index]
}

fn normal_byte(value) {
    to_int((value * 0.5 + 0.5) * 255.0)
}

fn arrow_slice(layer, size, color) {
    let color_data = blob(size * size * 4, 0);
    let normal_data = blob(size * size * 4, 0);
    for y in 0..size {
        for x in 0..size {
            let index = (y * size + x) * 4;
            let cx = to_float(x) / to_float(size - 1) - 0.5;
            let cy = to_float(y) / to_float(size - 1) - 0.5;
            let ci = to_float(layer) / to_float(size - 1) - 0.5;
            let shaft = cx >= -0.4 && cx <= 0.2 && abs(cy) < 0.03 && abs(ci) < 0.03;
            let head_progress = (cx - 0.2) / 0.3;
            let head = cx > 0.2 && cx <= 0.5 && sqrt(cy * cy + ci * ci) < (1.0 - head_progress) * 0.12;
            let tail = cx >= -0.5 && cx <= -0.3 && ((abs(ci) < 0.01 && abs(cy) < 0.12) || (abs(cy) < 0.01 && abs(ci) < 0.12));
            let current = color;
            if tail { current = [200, 200, 200, 255]; }
            if shaft || head || tail {
                color_data[index] = byte(current, 0);
                color_data[index + 1] = byte(current, 1);
                color_data[index + 2] = byte(current, 2);
                color_data[index + 3] = byte(current, 3);
                let nx = 0.0;
                if cx > 0.4 { nx = 1.0; } else if cx < -0.4 { nx = -1.0; }
                let ny = 0.0;
                if ci > 0.0 { ny = 1.0; } else if ci < 0.0 { ny = -1.0; }
                let nz = 0.0;
                if cy > 0.0 { nz = 1.0; } else if cy < 0.0 { nz = -1.0; }
                let length = sqrt(nx * nx + ny * ny + nz * nz);
                let normal_x = 0.0;
                let normal_y = 1.0;
                let normal_z = 0.0;
                if length > 0.0 {
                    normal_x = nx / length;
                    normal_y = ny / length;
                    normal_z = nz / length;
                }
                normal_data[index] = normal_byte(normal_x);
                normal_data[index + 1] = normal_byte(normal_y);
                normal_data[index + 2] = normal_byte(normal_z);
                normal_data[index + 3] = 255;
            }
        }
    }
    #{ "color_data": color_data, "normal_data": normal_data }
}

fn rock_slice(layer, size, color) {
    let color_data = blob(size * size * 4, 0);
    let normal_data = blob(size * size * 4, 0);
    for y in 0..size {
        for x in 0..size {
            let index = (y * size + x) * 4;
            let fx = to_float(x) / to_float(size - 1) - 0.5;
            let fy = to_float(y) / to_float(size - 1) - 0.5;
            let fz = to_float(layer) / to_float(size - 1) - 0.5;
            let distance = sqrt(fx * fx + fy * fy + fz * fz);
            if distance < 0.45 {
                color_data[index] = byte(color, 0);
                color_data[index + 1] = byte(color, 1);
                color_data[index + 2] = byte(color, 2);
                color_data[index + 3] = byte(color, 3);
                let nx = 0.0;
                let ny = 1.0;
                let nz = 0.0;
                if distance > 0.0 {
                    nx = fx / distance;
                    ny = fz / distance;
                    nz = fy / distance;
                }
                normal_data[index] = normal_byte(nx);
                normal_data[index + 1] = normal_byte(ny);
                normal_data[index + 2] = normal_byte(nz);
                normal_data[index + 3] = 255;
            }
        }
    }
    #{ "color_data": color_data, "normal_data": normal_data }
}

let arrow_size = 32;
let arrow_slices = [];
for layer in 0..arrow_size { arrow_slices.push(arrow_slice(layer, arrow_size, [200, 100, 50, 255])); }
collection.add_spritestack("Arrow", 0.05, arrow_size, arrow_size, arrow_slices);

let rock_size = 64;
let rock_slices = [];
for layer in 0..rock_size { rock_slices.push(rock_slice(layer, rock_size, [100, 100, 100, 255])); }
collection.add_spritestack("Rock", 0.05, rock_size, rock_size, rock_slices);
