use std::any::Any;
use crate::LoadedAssetData;
use bevy::ecs::system::{Res, ResMut};
use bevy_ecs_tilemap::prelude::TileTextureIndex;
use bevy_rand::resource::GlobalEntropy;
use bevy_prng::ChaCha8Rng;
use crate::Lifeform;
use bevy::prelude::{Handle, Image, Resource};
use asset_loading_plugin::loader::{Loadable};
use crate::worldgen::TerrainData;
use crate::utils::rand_range;

#[derive(Default, Eq, Hash, PartialEq, Clone, Debug, Copy, serde::Serialize, serde::Deserialize, bevy::asset::Asset, bevy::reflect::TypePath)]
#[serde(tag = "type")]
pub enum TileTextureData {
    #[default]
    None,
    Floor,
    Corridor {
        start: bool
    },
    Player,
    Enemy,
    Wall {
        #[serde(alias="north")]
        connects_north: bool,
        #[serde(alias="south")]
        connects_south: bool,
        #[serde(alias="east")]
        connects_east: bool,
        #[serde(alias="west")]
        connects_west: bool
    },
    Entrance,
    Exit
}

impl TileTextureData {
    pub fn can_replace(&self, old: TileTextureData) -> bool {
        return match (self, old) {
            (TileTextureData::Corridor{start},TileTextureData::Corridor{start: old_start}) => {
                *start || !old_start
            }
            (_,TileTextureData::Entrance) => {false}
            (_,TileTextureData::Exit) => {false}
            (TileTextureData::Corridor{..},_) => {true}
            (TileTextureData::Floor, TileTextureData::Corridor{..}) => {false}
            (TileTextureData::Wall{..}, TileTextureData::Corridor{..}) => {false}
            (TileTextureData::Wall{..}, TileTextureData::Floor) => {false}
            (TileTextureData::Enemy, TileTextureData::Player) => {false}
            (TileTextureData::Player, TileTextureData::Enemy) => {true}
            (TileTextureData::Enemy, _) => {panic!("Enemy replacing non-entity")}
            (TileTextureData::Player, _) => {panic!("Player replacing non-entity")}
            (_,_) => {true}
        }
    }

    pub fn makes_walls(&self) -> bool {
        return match self {
            TileTextureData::Corridor{..} => {true}
            TileTextureData::Floor => {true}
            _ => {false}
        }
    }

    pub fn connects_to_walls(&self) -> bool {
        return match self {
            TileTextureData::Wall{..} => {true}
            _ => {false}
        }
    }

    pub fn repair_tile_data(&self) -> TileTextureData {
        match self {
            TileTextureData::Wall {connects_north, connects_south, connects_east, connects_west} => {
                if !connects_north && !connects_south && !connects_east && !connects_west{
                    TileTextureData::Wall {
                        connects_north: false,
                        connects_east: true,
                        connects_south: false,
                        connects_west: true
                    }
                } else {
                    *self
                }
            }
            _ => {
                *self
            }
        }
    }

    pub fn pick_texture(&self, tile_data_holder: &Res<LoadedAssetData>, rng: &mut ResMut<GlobalEntropy<ChaCha8Rng>>) -> TileTextureIndex {
        if tile_data_holder.asset_data.contains_key(self) {

            let num_textures = tile_data_holder.asset_data[self].len();
            let mut picked_texture = 0;
            if num_textures > 1 {
                picked_texture = rand_range(0, num_textures as u32, rng);
            }
            if let Some(texture_holder) = &tile_data_holder.asset_data[self][picked_texture as usize].get_requested_type::<RelatedTextureData>() {
                return TileTextureIndex(texture_holder.texture_index);
            }
            panic!("Couldn't retrieve data for Tile: {:#?}", self);
        }
        panic!("Requested Texture for unknown Tile Data: {:#?}", self);
    }
}

#[derive(Default)]
pub struct TileData {
    passable: bool
}

#[derive(Default, Clone)]
pub struct RelatedTextureData {
    pub texture_handle: Handle<Image>,
    pub texture_index: u32,
    // Texture Weights
}

impl Loadable for RelatedTextureData {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Resource)]
pub struct TextureArray {
    pub textures: Vec<Handle<Image>>
}

pub struct WorldState {
    pub terrain: Vec<Vec<TerrainData>>,
    pub entities: Vec<Lifeform>
}