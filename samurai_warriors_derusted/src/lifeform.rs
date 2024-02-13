use bevy::prelude::Component;
use bevy_ecs_tilemap::tiles::{TilePos, TileStorage};
use crate::worldgen::TerrainData;

#[derive(Component)]
pub struct Player;

struct Enemy;

pub struct Lifeform {
    pub texture: TerrainData,
    pub position: TilePos,
    pub health: u32,
    pub strength: u32,
    pub defense: u32,
    pub level: u32,
    pub experience: u32
}

enum Direction {
    North,
    South,
    East,
    West
}

fn move_lifeform (direction: Direction, tile_pos: &mut TilePos, map_tile_storage: TileStorage, lifeform_tile_storage: TileStorage) -> bool {
    let mut next_pos = tile_pos.clone();
    match direction {
        Direction::North => {
            next_pos.y -= 1;
        }
        Direction::South => {
            next_pos.y += 1;
        }
        Direction::East => {
            next_pos.x += 1;
        }
        Direction::West => {
            next_pos.x -= 1;
        }
    }
    if let Some(map_entity) = map_tile_storage.checked_get(&next_pos) {
        // Check if entity is passable
        return if lifeform_tile_storage.checked_get(&next_pos).is_none() {
            true
        } else {
            // TODO: Send an event to trigger an attack
            false
        }
    }

    return false;
}