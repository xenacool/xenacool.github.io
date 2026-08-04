use hexx::Hex;
use pystral_games::{GridCell, GridMap, SkirmishConfig, TileType};

#[test]
fn scenario_grid_can_define_large_sparse_multilayer_domain() {
    let mut grid = GridMap::default();
    grid.bounds.horizontal = hexx::HexBounds::from_radius(50);
    grid.bounds.min_layer = 0;
    grid.bounds.max_layer = 31;
    grid.set_tile(GridCell::new(Hex::ZERO, 0), TileType::Grass)
        .unwrap();
    grid.set_tile(GridCell::new(Hex::ZERO, 1), TileType::Rock)
        .unwrap();

    let mut config = SkirmishConfig::new(42);
    config.set_grid(grid);
    config
        .add_unit(1, 1, "Caveman", GridCell::new(Hex::ZERO, 0))
        .unwrap();
    let state = config.build_state().unwrap();

    assert_eq!(
        state.grid.bounds.horizontal,
        hexx::HexBounds::from_radius(50)
    );
    assert_eq!(state.grid.bounds.max_layer, 31);
    assert_eq!(state.grid.tiles.len(), 2);
    assert_eq!(
        state.grid.tiles[&GridCell::new(Hex::ZERO, 1)],
        TileType::Rock
    );
    assert!(!state.grid.contains(GridCell::new(Hex::new(20, 0), 0)));
}
