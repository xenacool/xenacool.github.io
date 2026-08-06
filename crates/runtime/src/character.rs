// Note: All of the spriteparts are side profile
use pystral_core::domain::PainterCommand;
use pystral_compiler::assets::AssetCollection;
use pystral_macros::include_layers;


fn color_pair(c: [f32; 4]) -> ([f32; 4], [f32; 4]) {
    let mut mirrored = c;
    mirrored[0] = (c[0] + 0.2).min(1.0);
    mirrored[1] = (c[1] - 0.1).max(0.0);
    (c, mirrored)
}


pub fn register_assets(collection: &mut AssetCollection) {
    const TARGET_HEIGHT: f32 = 4.95;
    const SPACING_300_PNG: f32 = 0.05; // Gives 1.575 width for 32px PNG

    macro_rules! register_minion {
        ($name:expr, $path:expr) => {
            let layers = include_layers!(1..=300, $path).to_vec();
            collection.add_png_spritestack($name, SPACING_300_PNG, layers);
            if let Some(stack) = collection.spritestacks.get_mut($name) {
                stack.aabb.y = TARGET_HEIGHT;
            }
        };
    }

    register_minion!("SkeletonMinion", "../../../assets/spritestacks/skeleton_minion/layer-{}.png");
    register_minion!("Necromancer", "../../../assets/spritestacks/necromancer/layer-{}.png");
    register_minion!("Caveman", "../../../assets/spritestacks/caveman/layer-{}.png");
    register_minion!("Mage", "../../../assets/spritestacks/mage/layer-{}.png");
}

pub fn make_arrow_commands(color: [f32; 4]) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::SetStrokeWidth(6.0),
        // Shaft
        PainterCommand::MoveTo(40.0, 128.0, 0.0),
        PainterCommand::LineTo(200.0, 128.0, 0.0),
        PainterCommand::Stroke,
        // Head
        PainterCommand::MoveTo(200.0, 100.0, 0.0),
        PainterCommand::LineTo(240.0, 128.0, 0.0),
        PainterCommand::LineTo(200.0, 156.0, 0.0),
        PainterCommand::Close,
        PainterCommand::Fill,
        // Tail
        PainterCommand::MoveTo(40.0, 100.0, 0.0),
        PainterCommand::LineTo(40.0, 156.0, 0.0),
        PainterCommand::Stroke,
    ]
}


pub fn make_triangle_commands(color: [f32; 4]) -> Vec<PainterCommand> {
    let cp = color_pair(color);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::MoveTo(128.0, 64.0, 0.0),
        PainterCommand::LineTo(220.0, 160.0, 0.0),
        PainterCommand::LineTo(64.0, 160.0, 0.0),
        PainterCommand::Close,
        PainterCommand::Fill,
    ]
}

pub fn make_bone_commands(width: f32) -> Vec<PainterCommand> {
    let cp = color_pair([0.8, 0.8, 0.8, 1.0]);
    vec![
        PainterCommand::SetColor(cp.0, cp.1),
        PainterCommand::SetStrokeWidth(width),
        PainterCommand::MoveTo(128.0, 0.0, 0.0),
        PainterCommand::LineTo(128.0, 256.0, 0.0),
        PainterCommand::Stroke,
    ]
}
