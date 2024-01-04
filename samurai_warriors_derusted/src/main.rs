mod tile_data;
mod worldgen;
mod lifeform;
mod utils;

use crate::tile_data::{RelatedTextureData, TileTextureData, TextureArray, WorldState};
use std::any::TypeId;
use bevy::prelude::*;
use asset_loading_plugin::*;
use asset_loading_plugin::index::*;
use asset_loading_plugin::loader::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_common_assets::json::JsonAssetPlugin;
use bevy_rand::prelude::*;
use bevy_prng::ChaCha8Rng;
use leafwing_input_manager::prelude::*;
use crate::worldgen::{RoomGenerator};
use crate::lifeform::Lifeform;

const MAP_X: u32 = 100;
const MAP_Y: u32 = 40;
//const TILE_SIZE: TilemapTileSize = TilemapTileSize {x: 16.0, y: 16.0};

type LoadedAssetData = LoadedData<TileTextureData>;
type TileAssetLoadedEvent = AssetLoadedEvent<TileTextureData>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
enum AppState {
    #[default]
    AssetLoading,
    AssetLoaded,
    AssetPrepped,
    //Menu,
    Generate,
    //Play
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
enum Action {
    North,
    South,
    East,
    West,
    Skip
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins.set(ImagePlugin::default_nearest())
                           , AssetLoadingPlugin::<TileTextureData>::default()
                           , JsonAssetPlugin::<TileTextureData>::new(&["png.json"])
                           , TilemapPlugin
                           , EntropyPlugin::<ChaCha8Rng>::default()
                           , InputManagerPlugin::<Action>::default()
        ))
        .add_state::<AppState>()
        .add_systems(Startup, load_assets)
        .add_systems(Update, folder_loaded.run_if(on_event::<TileAssetLoadedEvent>()))
        .add_systems(Update, load_finished.run_if(on_event::<LoadingFinishedEvent>()))
        .add_systems(OnEnter(AppState::AssetLoaded), setup)
        .add_systems(OnEnter(AppState::AssetPrepped), skip_forward)
        .add_systems(OnEnter(AppState::Generate), generate)
        .insert_resource(TexturesToLoad{indexes: vec![]})
        .insert_resource(TextureArray{textures: vec![]})
        .run();
    // KV Store Docs: https://crates.io/crates/bevy_pkv
    // Input Docs: https://crates.io/crates/leafwing-input-manager
}

fn spawn_player() {
    //InputManagerBundle::<Action> {
    //    action_state: ActionState::default(),
    //    input_map: InputMap::new([
    //        (QwertyScanCode::W, Action::North),
    //        (QwertyScanCode::A, Action::West),
    //        (QwertyScanCode::S, Action::South),
    //        (QwertyScanCode::D, Action::East)
    //    ])
    //}
}

fn load_assets(
    mut commands: Commands,
    //server: Res<AssetServer>
) {
    commands.insert_resource(ResourceLocations {
        loc: vec![
            ResourceLocation {
                path: "textures".to_string(),
                handle: Handle::default()
            }
        ],
        loaded_count: 0
    });
}

#[derive(Resource, Default)]
pub struct TexturesToLoad {
    pub indexes: Vec<Handle<TextureIndex>>
}

fn folder_loaded(
    mut ev_asset_loaded: EventReader<TileAssetLoadedEvent>,
    mut asset_data: ResMut<LoadedAssetData>,
    //asset_server: Res<AssetServer>,
    mut texture_array: ResMut<TextureArray>
) {
    for ev in ev_asset_loaded.read() {
        let mut loadables: Vec<Box<dyn Loadable>> = vec![];

        for handle in &ev.handles {
            if TypeId::of::<Image>() == handle.type_id() {
                let texture_index = texture_array.textures.len() as u32;
                texture_array.textures.push(handle.clone().typed::<Image>().clone());
                let texture_data = RelatedTextureData {
                    texture_handle: handle.clone().typed::<Image>(),
                    texture_index
                };

                loadables.push(Box::new(texture_data));
            }
        }

        if !asset_data.asset_data.contains_key(&ev.key) {
            asset_data.asset_data.insert(ev.key, vec![AssociatedData{data: loadables}]);
        } else {
            let mut data = asset_data.asset_data.remove(&ev.key).unwrap();
            data.push(AssociatedData{data: loadables});
            asset_data.asset_data.insert(ev.key, data);

        }
    }
}

fn load_finished(
    mut next_state: ResMut<NextState<AppState>>,
    mut ev_loading_finished: EventReader<LoadingFinishedEvent>
) {
    for _ev in ev_loading_finished.read() {
        next_state.set(AppState::AssetLoaded);
    }
}

fn setup(
    mut next_state: ResMut<NextState<AppState>>
) {
    next_state.set(AppState::AssetPrepped);
}

fn skip_forward(
    mut next_state: ResMut<NextState<AppState>>
) {
    next_state.set(AppState::Generate);
}

fn generate_map(
    map_size: &TilemapSize,
    mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>,
    tile_data_holder: Res<LoadedAssetData>,
) -> WorldState {
    /*
        1. Generate a simplistic terrain of walls & floors
        2. Translate that terrain to textures
        3. Generate Entrance/Exit
        4. Generate Enemies
     */
    let mut room_generator = RoomGenerator{
        room_count: 50,
        map_width: map_size.x as usize,
        map_height: map_size.y as usize,
        rooms: vec![],
        mean_room_width: 7,
        mean_room_height: 6,
        width_variance: 3,
        height_variance: 2,
        max_enemies_per_room: 2,
    };

    let (terrain, entities) = room_generator.generate_rooms(&mut rng);
    let mut generated_map = WorldState {
        terrain,
        entities
    };

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            generated_map.terrain[x as usize][y as usize].tile_data = generated_map.terrain[x as usize][y as usize].tile_data.repair_tile_data();
            generated_map.terrain[x as usize][y as usize].texture = generated_map.terrain[x as usize][y as usize].tile_data.pick_texture(&tile_data_holder, &mut rng);
        }
    }

    for entity in &mut generated_map.entities {
        entity.texture.texture = entity.texture.tile_data.pick_texture(&tile_data_holder, &mut rng);
    }

    generated_map
}

fn generate(
    mut commands: Commands,
    texture_array: Res<TextureArray>,
    rng: ResMut<GlobalEntropy<ChaCha8Rng>>,
    tile_data_holder: Res<LoadedAssetData>,
) {
    commands.spawn(Camera2dBundle::default());

    let map_size = TilemapSize {
        x: MAP_X,
        y: MAP_Y
    };

    // 12/29 -
    // 1. Save world state somewhere
    // 2. Allow entities to move
    // 3. Allow the Player to control themselves

    let map = generate_map(&map_size, rng, tile_data_holder);

    let tilemap_entity = commands.spawn_empty().id();
    let mut tile_storage = TileStorage::empty(map_size);
    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: map.terrain[x as usize][y as usize].texture,
                    ..Default::default()
                })
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Vector(texture_array.textures.clone()),
        tile_size,
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });

    let tilemap_entity = commands.spawn_empty().id();
    let tile_storage = TileStorage::empty(map_size);
    for entity in map.entities {
        let _tile_entity = commands
            .spawn(TileBundle {
                position: entity.position,
                tilemap_id: TilemapId(tilemap_entity),
                texture_index: entity.texture.texture,
                ..Default::default()
            })
            .id();
        //tile_storage.set(&entity.position, tile_entity);
    }

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Vector(texture_array.textures.clone()),
        tile_size,
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });
}