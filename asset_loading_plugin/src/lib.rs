pub mod index;
pub mod loader;

use crate::loader::*;
use bevy::prelude::*;
use bevy_common_assets::json::JsonAssetPlugin;
use crate::index::TextureIndex;

#[derive(Default)]
pub struct AssetLoadingPlugin<A> {
    _phantom_data: A
}

impl<A: Sync + Send + Asset + Clone + 'static + Default> Plugin for AssetLoadingPlugin<A> {
    fn build(&self, app: &mut App) {
        app.add_state::<LoaderState>()
            .add_plugins(JsonAssetPlugin::<TextureIndex>::new(&["index.json"]))
            .add_event::<AssetLoadedEvent<A>>()
            .add_event::<LoadingFinishedEvent>()
            .add_systems(PostStartup, load_asset)
            .add_systems(Update, await_loading::<A>.run_if(in_state(LoaderState::AssetLoading)))
            .add_systems(OnEnter(LoaderState::AssetLoaded), loaded)
            .insert_resource(LoadedData::<A>::default());
    }
}