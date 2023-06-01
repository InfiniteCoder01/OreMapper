pub use crate::lib::egui_image::*;
pub use crate::lib::image_draw::DrawTarget;
pub use crate::lib::math::*;
pub use crate::lib::more_ui::*;

pub use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};

pub use anyhow::{bail, Context, Error, Result};
pub use uuid::Uuid;

use linked_hash_map::LinkedHashMap;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

// ******************** ATLAS ******************** //
pub struct Atlas {
    pub path: PathBuf,
    pub image: image::RgbaImage,
    pub tile_size: U16Vec2,
}

impl Atlas {
    pub fn new(path: &Path) -> Result<Self> {
        let image = image::open(path.to_str().unwrap())
            .context(format!(
                "Failed to load texture atlas from file {:?}!",
                path
            ))?
            .to_rgba8();

        let atl_path = path.with_extension("atl");
        let file = File::open(atl_path.clone());
        if let Ok(mut file) = file {
            let tile_size = TVec2::new(
                file.read_u16::<LittleEndian>().context(
                    format!("Failed to read tile width. Perhaps you are using unsupported atlas format? File {:?}.", atl_path),
                )?,
                file.read_u16::<LittleEndian>().context(
                    format!("Failed to read tile height. Perhaps you are using unsupported atlas format? File {:?}.", atl_path),
                )?,
            );
            Ok(Self {
                path: path.to_path_buf(),
                image,
                tile_size,
            })
        } else {
            let tile_size = TVec2::new(image.width(), image.height()).as_type();
            Ok(Self {
                path: atl_path,
                image,
                tile_size,
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        let mut file =
            File::create(&PathBuf::from(&self.path).with_extension("atl")).context(format!(
                "Failed to open/create file to save atlas! File {:?}.",
                self.path
            ))?;
        file.write_u16::<LittleEndian>(self.tile_size.x)
            .context(format!("Failed to save tile width! File {:?}.", self.path))?;
        file.write_u16::<LittleEndian>(self.tile_size.y)
            .context(format!("Failed to save tile height! File {:?}.", self.path))?;
        Ok(())
    }

    pub fn width(&self) -> u16 {
        (self.image.width() / self.tile_size.x as u32) as u16
    }

    pub fn height(&self) -> u16 {
        (self.image.height() / self.tile_size.y as u32) as u16
    }

    pub fn draw_tile(&self, to: &mut image::RgbaImage, pos: I32Vec2, tile: U32Vec2, size: I32Vec2) {
        to.draw_subimage(
            &self.image,
            pos.as_type(),
            size,
            tile.component_mul(&self.tile_size.as_type()),
            self.tile_size.as_type(),
        );
    }
}

pub struct AtlasView {
    pub atlas: Uuid,
    pub selection_pos: U32Vec2,
    pub selection_size: U32Vec2,
}

impl AtlasView {
    pub fn new(atlas: Uuid) -> Self {
        Self {
            atlas,
            selection_pos: U32Vec2::zeros(),
            selection_size: U32Vec2::new(1, 1),
        }
    }
}

// ******************** COMPONENT ******************** //
pub const ATLAS_RENDERER_UUID: Uuid = uuid::uuid!("c85d40c6-0b66-44ef-8361-061547fd8125");

#[derive(PartialEq, Clone, Debug)]
pub enum Property {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    String,
    Atlas,
}

impl Property {
    pub const VALUES: [Self; 9] = [
        Self::I8,
        Self::U8,
        Self::I16,
        Self::U16,
        Self::I32,
        Self::U32,
        Self::F32,
        Self::String,
        Self::Atlas,
    ];

    pub fn fix_value(
        &self,
        valid_atalses: &HashSet<Uuid>,
        _valid_maps: &HashSet<Uuid>,
        value: &String,
    ) -> String {
        fn fix_uuid(source: &HashSet<Uuid>, value: &String) -> String {
            if let Ok(uuid) = Uuid::parse_str(value) {
                if !source.contains(&uuid) {
                    source.iter().next().unwrap().to_string()
                } else {
                    value.clone()
                }
            } else {
                source.iter().next().unwrap().to_string()
            }
        }

        match self {
            Property::I8
            | Property::U8
            | Property::I16
            | Property::U16
            | Property::I32
            | Property::U32
            | Property::F32 => {
                if value.is_empty() {
                    "0".to_owned()
                } else {
                    value.chars().filter(|ch| ch.is_ascii_digit()).collect()
                }
            }
            Property::Atlas => fix_uuid(valid_atalses, value),
            Property::String => value.clone(),
        }
    }
}

pub struct Component {
    pub path: PathBuf,
    pub properties: LinkedHashMap<String, Property>,
}

impl Component {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            properties: LinkedHashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let mut file = File::open(path.clone())
            .context(format!("Failed to open component file {:?}!", path))?;
        let mut properties = LinkedHashMap::new();
        while let Ok(name) = read_string(&mut file) {
            let property_type = file.read_u8().context(format!(
                "Failed to read component property type! File {:?}.",
                path
            ))?;
            let property_type = match property_type {
                0 => Property::I8,
                1 => Property::U8,
                2 => Property::I16,
                3 => Property::U16,
                4 => Property::I32,
                5 => Property::U32,
                6 => Property::F32,
                7 => Property::String,
                8 => Property::Atlas,
                _ => {
                    bail!(
                        "Invalid component property type: {}! File {:?}.",
                        property_type,
                        path
                    );
                }
            };
            properties.insert(name, property_type);
        }
        Ok(Self {
            path: path.to_path_buf(),
            properties,
        })
    }

    pub fn save(&self) -> Result<()> {
        if self.path.starts_with("builtin/") {
            return Ok(());
        }
        let mut file = File::create(&PathBuf::from(&self.path)).context(format!(
            "Failed to open/create file to save component! File {:?}.",
            self.path
        ))?;
        for (name, property_type) in &self.properties {
            write_string(&mut file, name).context(format!(
                "Failed to write component property name! File {:?}.",
                self.path
            ))?;
            file.write_u8(match property_type {
                Property::I8 => 0,
                Property::U8 => 1,
                Property::I16 => 2,
                Property::U16 => 3,
                Property::I32 => 4,
                Property::U32 => 5,
                Property::F32 => 6,
                Property::String => 7,
                Property::Atlas => 8,
            })
            .context(format!(
                "Failed to write component property type! File {:?}.",
                self.path
            ))?;
        }
        Ok(())
    }

    pub fn fix_instance(
        &self,
        instance: &mut HashMap<String, String>,
        valid_atlases: &HashSet<Uuid>,
        valid_maps: &HashSet<Uuid>,
    ) {
        let mut new_instance = HashMap::new();
        for (name, property_type) in &self.properties {
            new_instance.insert(
                name.clone(),
                property_type.fix_value(
                    valid_atlases,
                    valid_maps,
                    instance.get(name).unwrap_or(&String::new()),
                ),
            );
        }
        *instance = new_instance;
    }
}

pub struct ComponentView {
    pub component: Uuid,
    pub adding: Option<(String, Property)>,
}

impl ComponentView {
    pub fn new(component: Uuid) -> Self {
        Self {
            component,
            adding: None,
        }
    }
}

// ******************** MAP ******************** //
pub struct Object {
    pub pos: I32Vec2,
    pub components: HashMap<Uuid, HashMap<String, String>>,
}

impl Object {
    pub fn new(pos: I32Vec2, components: &[(Uuid, HashMap<String, String>)]) -> Self {
        Self {
            pos,
            components: components.iter().cloned().collect(),
        }
    }

    pub fn altas<'a>(&mut self, atlases: &'a HashMap<Uuid, Atlas>) -> Result<Option<&'a Atlas>> {
        if let Some(atlas) = self.components.get(&ATLAS_RENDERER_UUID) {
            if let Some(atlas) = atlases.get(
                &Uuid::parse_str(
                    atlas
                        .get("Atlas")
                        .context("[PROBABLY A BUG] Atlas component without atlas!")?,
                )
                .context("[PROBABLY A BUG] Atlas UUID in atlas renderer is invalid!")?,
            ) {
                Ok(Some(atlas))
            } else {
                self.components.remove(&ATLAS_RENDERER_UUID);
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

pub struct Map {
    pub path: PathBuf,
    pub size: U16Vec2,
    pub data: Vec<I16Vec2>,
    pub atlas: Uuid,
    pub objects: HashMap<Uuid, Object>,
}

impl Map {
    pub fn load(path: &Path) -> Result<Self> {
        let mut file =
            File::open(path.clone()).context(format!("Failed to open map file {:?}!", path))?;
        let size = TVec2::new(
            file.read_u16::<LittleEndian>().context(
                format!("Failed to read map width. Perhaps you are using unsupported map format? File {:?}.",path)
            )? as _,
            file.read_u16::<LittleEndian>().context(format!(
                "Failed to read map height. Perhaps you are using unsupported map format? File {:?}.",
                path
            ))? as _,
        );
        let atlas = read_uuid(&mut file).context(format!(
            "Failed to read atlas UUID. Perhaps you are using unsupported map format? File {:?}.",
            path
        ))?;
        let bytes = size.x as usize * size.y as usize;
        let mut data = vec![TVec2::new(-1, -1); bytes];
        for i in 0..bytes {
            data[i as usize] = TVec2::new(
                file.read_i16::<LittleEndian>().context(
                    format!(
                        "Failed to read map tile.x. Perhaps you are using unsupported map format? File {:?}.",
                        path
                    )
                )?,
                file.read_i16::<LittleEndian>().context(
                    format!(
                        "Failed to read map tile.y. Perhaps you are using unsupported map format? File {:?}.",
                        path
                    )
                )?,
            );
        }
        Ok(Self {
            path: path.to_path_buf(),
            size,
            data,
            atlas,
            objects: HashMap::new(),
        })
    }

    pub fn save(&self) -> Result<()> {
        let mut file = File::create(&self.path).context(format!(
            "Failed to open/create file to save map! File {:?}.",
            self.path
        ))?;
        file.write_u16::<LittleEndian>(self.size.x)
            .context(format!("Failed to save map width! File {:?}.", self.path))?;
        file.write_u16::<LittleEndian>(self.size.y)
            .context(format!("Failed to save map height! File {:?}.", self.path))?;
        file.write(self.atlas.as_bytes()).context(format!(
            "Failed to save atlas UUID for map! File {:?}.",
            self.path
        ))?;
        for tile in &self.data {
            file.write_i16::<LittleEndian>(tile.x as i16)
                .context(format!("Failed to save map tile.x! File {:?}.", self.path))?;
            file.write_i16::<LittleEndian>(tile.y as i16)
                .context(format!("Failed to save map tile.y! File {:?}.", self.path))?;
        }
        Ok(())
    }

    pub fn get_tile(&self, pos: I16Vec2) -> I16Vec2 {
        if pos < TVec2::zeros() || pos >= self.size.as_type() {
            TVec2::new(-1, -1)
        } else {
            self.data[pos.x as usize + pos.y as usize * self.size.x as usize]
        }
    }

    pub fn set_tile(&mut self, pos: I16Vec2, tile: I16Vec2) {
        if pos >= TVec2::zeros() && pos < self.size.as_type() {
            self.data[pos.x as usize + pos.y as usize * self.size.x as usize] = tile;
        }
    }
}

pub struct MapView {
    pub map: Uuid,
    pub camera: I32Vec2,
    pub scale: f32,
    pub hover_tile: Option<U32Vec2>,
}

impl MapView {
    pub fn new(map: Uuid) -> Self {
        Self {
            map,
            camera: TVec2::zeros(),
            scale: 1.0,
            hover_tile: None,
        }
    }
}

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
        if let Ok(mut file) = File::open(path.join("atlases.dat")) {
            while let Ok(uuid) = read_uuid(&mut file) {
                let path =
                    PathBuf::from(read_string(&mut file).context("Failed to read atlas path!")?);
                atlases.insert(uuid, Atlas::new(&path)?);
                uuids.insert(path, uuid);
            }
        }

        // * Maps
        if let Ok(mut file) = File::open(path.join("maps.dat")) {
            while let Ok(uuid) = read_uuid(&mut file) {
                let path =
                    PathBuf::from(read_string(&mut file).context("Failed to read map path!")?);
                let map = Map::load(&path)?;
                if !atlases.contains_key(&map.atlas) {
                    // Check Atlas
                    std::fs::remove_file(map.path)?;
                    continue;
                }
                maps.insert(uuid, map);
                uuids.insert(path, uuid);
            }
        }

        // * Components
        if let Ok(mut file) = File::open(path.join("components.dat")) {
            while let Ok(uuid) = read_uuid(&mut file) {
                let path = PathBuf::from(
                    read_string(&mut file).context("Failed to read component path!")?,
                );
                components.insert(uuid, Component::load(&path)?);
                uuids.insert(path, uuid);
            }
        }

        components.insert(
            ATLAS_RENDERER_UUID,
            Component {
                path: "builtin/AtlasRenderer".into(),
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
        let mut file = File::create(self.path.join("atlases.dat"))
            .context("Failed to save atlases to a file!")?;
        for (uuid, atlas) in self.atlases.iter() {
            file.write(uuid.as_bytes())
                .context("Failed to write atlas UUID!")?;
            write_string(
                &mut file,
                &atlas.path.to_str().context(format!(
                    "Failed to convert atlas path to string! Path {:?}.",
                    atlas.path
                ))?,
            )
            .context("Failed to write atlas path!")?;
        }

        // * Maps
        let mut file =
            File::create(self.path.join("maps.dat")).context("Failed to save maps to a file!")?;
        for (uuid, map) in self.maps.iter() {
            file.write(uuid.as_bytes())
                .context("Failed to write map UUID!")?;
            write_string(
                &mut file,
                &map.path.to_str().context(format!(
                    "Failed to convert map path to string! Path {:?}.",
                    map.path
                ))?,
            )
            .context("Failed to write map path!")?;
        }

        // * Components
        let mut file = File::create(self.path.join("components.dat"))
            .context("Failed to save components to a file!")?;
        for (uuid, component) in self.components.iter() {
            if component.path.starts_with("builtin/") {
                continue;
            }
            file.write(uuid.as_bytes())
                .context("Failed to write component UUID!")?;
            write_string(
                &mut file,
                &component.path.to_str().context(format!(
                    "Failed to convert component path to string! Path {:?}.",
                    component.path
                ))?,
            )
            .context("Failed to write component path!")?;
        }
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
        crate::view::atlas_view::export(self, &mut file)?;
        crate::view::editor_view::export(self, &mut file)?;
        Ok(())
    }
}

// ******************** FILES ******************** //
pub fn write_string(file: &mut File, s: &str) -> std::io::Result<()> {
    let len = s.len() as u32;
    file.write_u32::<LittleEndian>(len)?;
    file.write_all(s.as_bytes())?;
    Ok(())
}

pub fn read_string(file: &mut File) -> std::io::Result<String> {
    let len = file.read_u32::<LittleEndian>()?;
    let mut buffer = vec![0; len as usize];
    file.read_exact(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

pub fn read_uuid(file: &mut File) -> Result<Uuid, std::io::Error> {
    let mut uuid = [0u8; 16];
    file.read_exact(&mut uuid)?;
    Ok(Uuid::from_bytes(uuid))
}

// ******************** VECTORS ******************** //
impl Vec2Cast for egui::Pos2 {
    fn as_type<T2: num::NumCast>(&self) -> TVec2<T2> {
        TVec2::new(
            T2::from(self.x).expect("Failed to cast!"),
            T2::from(self.y).expect("Failed to cast!"),
        )
    }
}

impl Vec2Cast for egui::Vec2 {
    fn as_type<T2: num::NumCast>(&self) -> TVec2<T2> {
        TVec2::new(
            T2::from(self.x).expect("Failed to cast!"),
            T2::from(self.y).expect("Failed to cast!"),
        )
    }
}

impl<T: num::NumCast + Copy> Vec2Cast for (T, T) {
    fn as_type<T2: num::NumCast>(&self) -> TVec2<T2> {
        TVec2::new(
            T2::from(self.0).expect("Failed to cast!"),
            T2::from(self.1).expect("Failed to cast!"),
        )
    }
}

macro_rules! impl_vector_cast_for_primitive {
    ($type: ty) => {
        impl Vec2Cast for $type {
            fn as_type<T2: num::NumCast>(&self) -> TVec2<T2> {
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
