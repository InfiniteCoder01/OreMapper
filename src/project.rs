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
    inspector_view::{Component, ComponentView, Property},
    inspector_view::{ATLAS_RENDERER_UUID, SERIALIZE_UUID},
};

use std::collections::HashMap;

// * --------------------------------------------------------------------------------- ASSETS --------------------------------------------------------------------------------- * //
#[derive(Default)]
pub struct NewMap {
    pub name: String,
    pub size: U16Vec2,
    pub atlas: Uuid,
}

pub struct Assets {
    pub path: PathBuf,
    pub content_viewer_path: PathBuf,
    pub new_map: Option<NewMap>,
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

        macro_rules! load_uuids {
            ($target: ident, $type: ty, $file: literal, $load_error: literal, $deserialize_error: literal) => {
                if path.join("atlases.json").exists() {
                    for (uuid, path) in serde_json::from_str::<HashMap<Uuid, PathBuf>>(
                        &std::fs::read_to_string(path.join($file))
                            .context(format!($load_error, path.join($file)))?,
                    )
                    .context($deserialize_error)?
                    {
                        $target.insert(uuid, <$type>::load(&path)?);
                        uuids.insert(path, uuid);
                    }
                }
            };
        }

        load_uuids!(
            atlases,
            Atlas,
            "atlases.json",
            "Failed to load atlas list from {:?}!",
            "Failed to deserialize atlas list!"
        );
        load_uuids!(
            maps,
            Map,
            "maps.json",
            "Failed to load map list from {:?}!",
            "Failed to deserialize map list!"
        );
        load_uuids!(
            components,
            Component,
            "components.json",
            "Failed to load component list from {:?}!",
            "Failed to deserialize component list!"
        );

        components.insert(
            ATLAS_RENDERER_UUID,
            Component {
                path: "/\nbuiltin/0 - AtlasRenderer".into(),
                properties: [("Atlas".to_owned(), Property::Atlas)]
                    .into_iter()
                    .collect(),
            },
        );
        components.insert(
            SERIALIZE_UUID,
            Component {
                path: "/\nbuiltin/1 - Serialize".into(),
                properties: indexmap::IndexMap::new(),
            },
        );

        Ok(Self {
            path: path.to_path_buf(),
            content_viewer_path: path.to_path_buf(),
            new_map: None,
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
        macro_rules! save_uuids {
            ($target: ident, $name: ident, $file: literal, $serialize_error: literal, $save_error: literal) => {
                std::fs::write(
                    self.path.join($file),
                    serde_json::to_string(
                        &self
                            .$target
                            .iter()
                            .map(|(uuid, $name)| (uuid, $name.path.clone()))
                            .collect::<HashMap<_, _>>(),
                    )
                    .context($serialize_error)?,
                )
                .context($save_error)?;
            };
        }

        save_uuids!(
            atlases,
            atlas,
            "atlases.json",
            "Failed to serialize atlas list!",
            "Failed to save atlas list!"
        );
        save_uuids!(
            maps,
            map,
            "maps.json",
            "Failed to serialize map list!",
            "Failed to save map list!"
        );

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

// * --------------------------------------------------------------------------------- VECTORS -------------------------------------------------------------------------------- * //
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
