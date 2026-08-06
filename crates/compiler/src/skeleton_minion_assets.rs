use pystral_macros::include_layers;

pub static LAYERS: &[&[u8]] = include_layers!(1..=300, "../../../assets/spritestacks/skeleton_minion/layer-{}.png");
