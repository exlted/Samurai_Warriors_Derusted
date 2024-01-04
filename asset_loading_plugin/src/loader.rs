use std::any::{Any, TypeId};
use std::ops::Deref;
use std::path::Path;
use bevy::asset::LoadedFolder;
use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::index::TextureIndex;


pub trait Loadable: Sync + Send + Any {
    fn as_any(&self) -> &dyn Any;
}

pub struct AssociatedData {
    pub data: Vec<Box<dyn Loadable>>
}

impl AssociatedData {
    pub fn get_requested_type<A: Loadable>(&self) -> Option<&A> {
        for asset in &self.data {
            if TypeId::of::<A>() == asset.deref().type_id() {
                let _rv:&A = match asset.as_any().downcast_ref::<A>() {
                    Some(rv) => return Some(rv),
                    None => continue
                };
            }
        }
        None
    }

//  pub fn get_requested_type_mut<'a, A: Loadable>(assets: &'a mut Vec<Box<dyn Loadable>>) -> Option<&'a mut A> {
//    for asset in assets {
//        if TypeId::of::<A>() == asset.deref().type_id() {
//            let rv:&mut A = match asset.as_any().downcast_mut::<A>() {
//                Some(rv) => return Some(rv),
//                None => continue
//            };
//        }
//    }
//    None
//}
}

#[derive(Resource, Default)]
pub struct LoadedData<A> {
    pub asset_data: HashMap<A, Vec<AssociatedData>>
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
pub(crate) enum LoaderState {
    #[default]
    AssetLoading,
    AssetLoaded
}

#[derive(Clone)]
pub struct ResourceLocation {
    pub path: String,
    pub handle: Handle<LoadedFolder>
}

#[derive(Event)]
pub struct AssetLoadedEvent<A> {
    pub key: A,
    pub handles: Vec<UntypedHandle>
}

#[derive(Event)]
pub struct LoadingFinishedEvent;

#[derive(Resource, Default)]
pub struct ResourceLocations {
    pub loc: Vec<ResourceLocation>,
    pub loaded_count: u32
}

pub(crate) fn load_asset(
    mut next_state: ResMut<NextState<LoaderState>>,
    server: Res<AssetServer>,
    mut asset_sources: ResMut<ResourceLocations>
) {
    for location in &mut asset_sources.loc {
        location.handle = server.load_folder(location.path.clone());
    }

    asset_sources.loaded_count = 0;
    next_state.set(LoaderState::AssetLoading);
}

pub(crate) fn await_loading<A: Send + Sync + Asset + Clone + 'static>(
    mut next_state: ResMut<NextState<LoaderState>>,
    mut asset_sources: ResMut<ResourceLocations>,
    mut events: EventReader<AssetEvent<LoadedFolder>>,
    mut ev_asset_loaded: EventWriter<AssetLoadedEvent<A>>,
    loaded_keys: ResMut<Assets<A>>,
    loaded_folders: ResMut<Assets<LoadedFolder>>,
    texture_indexes: ResMut<Assets<TextureIndex>>,
    asset_server: Res<AssetServer>
) {
    let mut loaded = false;
    for event in events.read() {
        for location in &asset_sources.loc {
            if event.is_loaded_with_dependencies(&location.handle) {
                let mut texture_index: Option<&TextureIndex> = None;


                if let Some(files) = loaded_folders.get(&location.handle) {
                    for handle in &files.handles {
                        if TypeId::of::<TextureIndex>() == handle.type_id() {
                            texture_index = texture_indexes.get(handle);
                            break;
                        }
                    }
                }

                if let Some(texture_index) = texture_index {
                    for related_files in &texture_index.files {
                        let mut handles: Vec<UntypedHandle> = vec![];
                        let mut key: Option<&A> = None;
                        for file in &related_files.associated_files {
                            let full_path = Path::new(&location.path).join(file).to_string_lossy().to_string();
                            if let Some(untyped_handle) = asset_server.get_handle_untyped(full_path) {
                                if TypeId::of::<A>() == untyped_handle.type_id() {
                                    key = loaded_keys.get(untyped_handle.id());
                                } else {
                                    handles.push(untyped_handle);
                                }
                            }
                        }
                        if let Some(key) = key {
                            ev_asset_loaded.send(AssetLoadedEvent{key: key.clone(), handles});
                        }
                    }
                }

                loaded = true;
            }
        }
        if loaded {
            asset_sources.loaded_count += 1;
            loaded = false;
        }
    }

    if asset_sources.loaded_count >= asset_sources.loc.len() as u32 {
        next_state.set(LoaderState::AssetLoaded);
    }
}

pub(crate) fn loaded(
    mut ev_loading_finished: EventWriter<LoadingFinishedEvent>
) {
    ev_loading_finished.send(LoadingFinishedEvent);
}