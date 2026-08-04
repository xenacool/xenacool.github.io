use pystral_core::domain::{HexMap, HexTile};
use hexx::Hex;

pub fn create_demo_world() -> HexMap {
    let mut map = HexMap::new();
    let width = 11;
    let height_grid = 12;
    
    for q in 0..width {
        for r in 0..height_grid {
            let hex = Hex::new(q as i32 - width as i32 / 2, r as i32 - height_grid as i32 / 2);
            
            // 4 different minor height variations
            let base_height = ((q + r) % 4) as f32 * 0.05;
            
            // Wall at q = width/2 (middle of the grid)
            let is_wall = q == width / 2;
            let is_hole = is_wall && (r >= height_grid / 2 - 1 && r <= height_grid / 2 + 1);

            if is_wall && !is_hole {
                // Tall wall
                for i in 0..10 {
                    map.tiles.push(HexTile {
                        hex,
                        bottom: i as f32 * 0.5,
                        height: 0.5,
                        material: "rock".to_string(),
                    });
                }
            } else if is_wall && is_hole {
                // Hole in the wall
                // Bottom part
                for i in 0..2 {
                    map.tiles.push(HexTile {
                        hex,
                        bottom: i as f32 * 0.5,
                        height: 0.5,
                        material: "rock".to_string(),
                    });
                }
                // Top part
                for i in 8..10 {
                    map.tiles.push(HexTile {
                        hex,
                        bottom: i as f32 * 0.5,
                        height: 0.5,
                        material: "rock".to_string(),
                    });
                }
                // Base layer for the hole
                map.tiles.push(HexTile {
                    hex,
                    bottom: 1.0,
                    height: 0.1,
                    material: "dirt".to_string(),
                });
            } else {
                // Normal ground
                map.tiles.push(HexTile {
                    hex,
                    bottom: 0.0,
                    height: 0.1 + base_height,
                    material: "grass".to_string(),
                });
            }
        }
    }
    map
}
