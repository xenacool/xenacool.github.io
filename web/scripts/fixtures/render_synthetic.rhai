// A1.0.1 render-baseline fixture: synthetic spritestacks bound to the real
// job names so the runtime pixel path renders cutout, alpha-normalized, and
// non-square stack classes without any renderer change.
// @include "/scenarios/skirmish.rhai"

sim.set_agent_controller(1, "Player");
sim.set_agent_controller(2, "WaitOnly");
sim.set_agent_controller(3, "WaitOnly");
sim.set_agent_controller(4, "WaitOnly");
sim.set_agent_controller(5, "WaitOnly");
sim.set_agent_controller(6, "WaitOnly");

// All four render-class agents packed around the player on walkable grass,
// close enough for sprites to approach on screen without sharing a cell.
sim.set_agent_position(1, cell(hex(0, 0), 0));
sim.set_agent_position(2, cell(hex(1, 0), 0));
sim.set_agent_position(5, cell(hex(0, 1), 0));
sim.set_agent_position(6, cell(hex(-1, 0), 0));

let syn = new_asset_collection();

// Flat up normal for every pixel of a w x h slice.
fn rn_flat_normal(width, height) {
    let data = blob(width * height * 4, 0);
    for i in 0..(width * height * 4) {
        data[i] = if i % 4 == 0 { 128 } else if i % 4 == 1 { 255 } else if i % 4 == 2 { 128 } else { 255 };
    }
    data
}

// Opaque pixel in a color buffer.
fn rn_put_pixel(data, index, r, g, b, a) {
    data[index] = r;
    data[index + 1] = g;
    data[index + 2] = b;
    data[index + 3] = a;
}

// Cutout class: one opaque shape, alpha 255 everywhere inside.
fn rn_cutout_slice(width, height, shape, r, g, b) {
    let color_data = blob(width * height * 4, 0);
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) * 4;
            let cx = to_float(x) / to_float(width - 1) - 0.5;
            let cy = to_float(y) / to_float(height - 1) - 0.5;
            let inside = false;
            if shape == "square" { inside = abs(cx) <= 0.4 && abs(cy) <= 0.4; }
            if shape == "circle" { inside = cx * cx + cy * cy <= 0.2; }
            if shape == "triangle" { inside = cy <= 0.35 && cy >= -0.4 && abs(cx) <= (cy + 0.4) * 0.9; }
            if inside { rn_put_pixel(color_data, index, r, g, b, 255); }
        }
    }
    #{ "color_data": color_data, "normal_data": rn_flat_normal(width, height) }
}

// Alpha-normalization class: 128/127 checkerboard proves the binary cutoff.
fn rn_normalized_slice(width, height) {
    let color_data = blob(width * height * 4, 0);
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) * 4;
            let cx = to_float(x) / to_float(width - 1) - 0.5;
            let cy = to_float(y) / to_float(height - 1) - 0.5;
            let alpha = if (x + y) % 2 == 0 { 128 } else { 127 };
            if cx * cx + cy * cy <= 0.23 { rn_put_pixel(color_data, index, 200, 60, 220, alpha); }
        }
    }
    #{ "color_data": color_data, "normal_data": rn_flat_normal(width, height) }
}

// 16 slices per stack: the runtime draws each slice as a vertical billboard
// at terrain depth, so a 2-slice stack sits in the terrain plane and is depth
// culled (the real atlas stacks have 200-300 slices for the same reason).
let caveman_slices = [];
for i in 0..16 {
    if i % 2 == 0 { caveman_slices.push(rn_cutout_slice(32, 32, "square", 200, 40, 40)); }
    else { caveman_slices.push(rn_cutout_slice(32, 32, "circle", 40, 40, 200)); }
}
syn.add_spritestack("Caveman", 0.05, 32, 32, caveman_slices);

let mage_slices = [];
for i in 0..16 { mage_slices.push(rn_normalized_slice(32, 32)); }
syn.add_spritestack("Mage", 0.05, 32, 32, mage_slices);

let necro_slices = [];
for i in 0..16 {
    if i % 2 == 0 { necro_slices.push(rn_cutout_slice(16, 48, "square", 60, 180, 60)); }
    else { necro_slices.push(rn_cutout_slice(16, 48, "triangle", 30, 120, 160)); }
}
syn.add_spritestack("Necromancer", 0.05, 16, 48, necro_slices);

let skeleton_slices = [];
for i in 0..16 {
    if i % 2 == 0 { skeleton_slices.push(rn_cutout_slice(32, 32, "triangle", 230, 230, 60)); }
    else { skeleton_slices.push(rn_cutout_slice(32, 32, "square", 90, 90, 90)); }
}
syn.add_spritestack("Skeleton_Minion", 0.05, 32, 32, skeleton_slices);

// Rebuild the projectile stacks the abilities resolve at cast time, so the
// replacement collection is complete.
let rock_slices = [];
for layer in 0..64 { rock_slices.push(rock_slice(layer, 64, [100, 100, 100, 255])); }
syn.add_spritestack("Rock", 0.05, 64, 64, rock_slices);
syn = add_orb(syn, "FireballOrb", 24, [255, 90, 20, 255]);
syn = add_orb(syn, "FrostBolt", 20, [90, 190, 255, 255]);
syn = add_orb(syn, "SoulOrb", 20, [170, 70, 255, 255]);

history.define_asset_collection("primitives", syn);
