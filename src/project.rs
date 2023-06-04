pub use crate::lib::egui_image::*;
pub use crate::lib::image_draw::DrawTarget;
pub use crate::lib::math::*;
pub use crate::lib::more_ui::*;

pub use anyhow::{bail, Context, Result};
pub use byteorder::{LittleEndian, WriteBytesExt};
pub use serde::{Deserialize, Serialize};
pub use std::path::{Path, PathBuf};
pub use uuid::Uuid;

pub use crate::view::{
    atlas_view::{Atlas, AtlasView},
    editor_view::{Map, MapView},
    inspector_view::ATLAS_RENDERER_UUID,
    inspector_view::{Component, ComponentView, Property},
};

use std::collections::HashMap;

// ******************** ASSETS ******************** //
pub struct Assets {
    pub path: PathBuf,
    pub content_viewer_path: PathBuf,
    pub new_component_name: Option<String>,

    pub atlas_selected: Option<AtlasView>,
    pub map_selected: Option<MapView>,
    pub object_selected: Option<Uuid>,
    pub component_selected: Option<ComponentView>,

    pub uuids: HashMap<PathBuf, Uuid>,
    pub atlases: HashMap<Uuid, Atlas>,
    pub maps: HashMap<Uuid, Map>,
    pub components: HashMap<Uuid, Component>,

    pub atlas_indices: HashMap<Uuid, u16>,
}

impl Assets {
    pub fn load(path: &Path) -> Result<Self> {
        let mut uuids = HashMap::new();
        let mut atlases = HashMap::new();
        let mut maps = HashMap::new();
        let mut components = HashMap::new();

        // * Atlases
        for (uuid, path) in serde_json::from_str::<HashMap<Uuid, PathBuf>>(
            &std::fs::read_to_string(path.join("atlases.json"))
                .context("Failed to load atlas list!")?,
        )
        .context("Failed to deserialize atlas list!")?
        {
            atlases.insert(uuid, Atlas::new(&path)?);
            uuids.insert(path, uuid);
        }

        // * Maps
        for (uuid, path) in serde_json::from_str::<HashMap<Uuid, PathBuf>>(
            &std::fs::read_to_string(path.join("maps.json")).context("Failed to load map list!")?,
        )
        .context("Failed to deserialize map list!")?
        {
            maps.insert(uuid, Map::load(&path)?);
            uuids.insert(path, uuid);
        }

        // * Components
        for (uuid, path) in serde_json::from_str::<HashMap<Uuid, PathBuf>>(
            &std::fs::read_to_string(path.join("components.json"))
                .context("Failed to load map list!")?,
        )
        .context("Failed to deserialize map list!")?
        {
            components.insert(uuid, Component::load(&path)?);
            uuids.insert(path, uuid);
        }

        components.insert(
            ATLAS_RENDERER_UUID,
            Component {
                path: "/\nbuiltin/0 - AtlasRenderer".into(),
                properties: [("Atlas".to_owned(), Property::Atlas)]
                    .into_iter()
                    .collect(),
            },
        );

        Ok(Self {
            path: path.to_path_buf(),
            content_viewer_path: path.to_path_buf(),
            new_component_name: None,

            atlas_selected: None,
            map_selected: None,
            object_selected: None,
            component_selected: None,

            uuids,
            atlases,
            maps,
            components,

            atlas_indices: HashMap::new(),
        })
    }

    pub fn save(&self) -> Result<()> {
        // * Atlases
        std::fs::write(
            self.path.join("atlases.json"),
            serde_json::to_string(
                &self
                    .atlases
                    .iter()
                    .map(|(uuid, atlas)| (uuid, atlas.path.clone()))
                    .collect::<HashMap<_, _>>(),
            )
            .context("Failed to serialize atlas list!")?,
        )
        .context("Failed to save atlas list!")?;

        // * Maps
        std::fs::write(
            self.path.join("maps.json"),
            serde_json::to_string(
                &self
                    .maps
                    .iter()
                    .map(|(uuid, map)| (uuid, map.path.clone()))
                    .collect::<HashMap<_, _>>(),
            )
            .context("Failed to serialize map list!")?,
        )
        .context("Failed to save map list!")?;

        // * Components
        std::fs::write(
            self.path.join("components.json"),
            serde_json::to_string(
                &(self
                    .components
                    .iter()
                    .filter_map(|(uuid, component)| {
                        if !component.path.starts_with("/\nbuiltin/") {
                            Some((uuid, component.path.clone()))
                        } else {
                            None
                        }
                    })
                    .collect::<HashMap<_, _>>()),
            )
            .context("Failed to serialize component list!")?,
        )
        .context("Failed to save component list!")?;

        // * All
        for (_, atlas) in self.atlases.iter() {
            atlas.save()?;
        }
        for (_, map) in self.maps.iter() {
            map.save()?;
        }
        for (_, component) in self.components.iter() {
            component.save()?;
        }
        Ok(())
    }

    pub fn export(&mut self) -> Result<()> {
        let mut file = std::fs::File::create(self.path.join("data.dat"))
            .context("Failed to create data file.")?;
        crate::view::atlas_view::export(self, &mut file).context("Failed to serialize atlases!")?;
        crate::view::editor_view::export(self, &mut file).context("Failed to serialize maps!")?;
        Ok(())
    }
}

// ******************** VECTORS ******************** //
impl Vec2Cast for egui::Pos2 {
    fn casted<T2: num::NumCast>(&self) -> TVec2<T2> {
        TVec2::new(
            T2::from(self.x).expect("Failed to cast!"),
            T2::from(self.y).expect("Failed to cast!"),
        )
    }
}

impl Vec2Cast for egui::Vec2 {
    fn casted<T2: num::NumCast>(&self) -> TVec2<T2> {
        TVec2::new(
            T2::from(self.x).expect("Failed to cast!"),
            T2::from(self.y).expect("Failed to cast!"),
        )
    }
}

impl<T: num::NumCast + Copy> Vec2Cast for (T, T) {
    fn casted<T2: num::NumCast>(&self) -> TVec2<T2> {
        TVec2::new(
            T2::from(self.0).expect("Failed to cast!"),
            T2::from(self.1).expect("Failed to cast!"),
        )
    }
}

macro_rules! impl_vector_cast_for_primitive {
    ($type: ty) => {
        impl Vec2Cast for $type {
            fn casted<T2: num::NumCast>(&self) -> TVec2<T2> {
                TVec2::new(
                    T2::from(*self).expect("Failed to cast!"),
                    T2::from(*self).expect("Failed to cast!"),
                )
            }
        }
    };
}

impl_vector_cast_for_primitive!(i8);
impl_vector_cast_for_primitive!(i16);
impl_vector_cast_for_primitive!(i32);
impl_vector_cast_for_primitive!(i64);
impl_vector_cast_for_primitive!(i128);
impl_vector_cast_for_primitive!(u8);
impl_vector_cast_for_primitive!(u16);
impl_vector_cast_for_primitive!(u32);
impl_vector_cast_for_primitive!(u64);
impl_vector_cast_for_primitive!(u128);
impl_vector_cast_for_primitive!(f32);
impl_vector_cast_for_primitive!(f64);
