#[derive(serde::Deserialize, bevy::asset::Asset, bevy::reflect::TypePath)]
pub struct TextureRelation {
    pub associated_files: Vec<String>
}

#[derive(serde::Deserialize, bevy::asset::Asset, bevy::reflect::TypePath)]
pub struct TextureIndex {
    pub files: Vec<TextureRelation>
}