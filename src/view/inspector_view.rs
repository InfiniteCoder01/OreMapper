use crate::project::*;
use indexmap::IndexMap;
use itertools::Itertools;
use std::collections::HashMap;

// * -------------------------------------------------------------------------------- COMPONENT ------------------------------------------------------------------------------- * //
pub const ATLAS_RENDERER_UUID: Uuid = uuid::uuid!("c85d40c6-0b66-44ef-8361-061547fd8125");
pub const SERIALIZE_UUID: Uuid = uuid::uuid!("8d4db834-82c3-4c27-b49c-7c32c638e1ef");

#[derive(Serialize, Deserialize)]
pub struct Component {
    #[serde(skip)]
    pub path: PathBuf,
    pub properties: IndexMap<String, Property>,
}

impl Component {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            properties: IndexMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            ..serde_json::from_str(
                &std::fs::read_to_string(path)
                    .context(format!("Failed to load component from file {:?}!", path))?,
            )
            .context(format!(
                "Failed to deserialize component from file {:?}!",
                path
            ))?
        })
    }

    pub fn save(&self) -> Result<()> {
        if self.path.starts_with("/\nbuiltin/") {
            return Ok(());
        }
        std::fs::write(
            &self.path,
            serde_json::to_string(&self).context(format!(
                "Failed to serialize componnet! File: {:?}!",
                self.path
            ))?,
        )
        .context(format!("Failed to save componnet to file {:?}!", self.path))?;
        Ok(())
    }

    pub fn fix_instance(
        &self,
        instance: &mut HashMap<String, String>,
        valid_atlases: impl Iterator<Item = Uuid> + Clone,
        valid_maps: impl Iterator<Item = Uuid> + Clone,
    ) {
        let mut new_instance = HashMap::new();
        for (name, property_type) in &self.properties {
            new_instance.insert(
                name.clone(),
                property_type.fix_value(
                    valid_atlases.clone(),
                    valid_maps.clone(),
                    instance.get(name).unwrap_or(&String::new()),
                ),
            );
        }
        *instance = new_instance;
    }
}

#[derive(Clone)]
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

// * ---------------------------------------------------------------------------------- SHOW ---------------------------------------------------------------------------------- * //
pub fn show(ui: &mut Ui, assets: &mut Assets) -> Result<()> {
    // * Component
    if let Some(view) = &mut assets.component_selected {
        if let Some(component) = assets.components.get_mut(&view.component) {
            fn property_type_select(
                ui: &mut Ui,
                property_type: &mut Property,
                id: impl std::hash::Hash,
            ) {
                ComboBox::from_id_source(id)
                    .selected_text(format!("{:?}", property_type))
                    .show_ui(ui, |ui| {
                        for type_variant in Property::VALUES {
                            ui.selectable_value(
                                property_type,
                                type_variant.clone(),
                                format!("{:?}", type_variant),
                            );
                        }
                    });
            }

            ui.label(format!(
                "Component: {}",
                component.path.file_stem().unwrap().to_str().unwrap()
            ));
            ui.separator();

            let mut remove = None;
            let (mut dragged_property, mut drag_target) = (None, None);
            for (index, (name, property_type)) in component.properties.iter_mut().enumerate() {
                let response = ui
                    .horizontal(|ui| {
                        ui.label(name);
                        property_type_select(ui, property_type, name);
                        if ui.button("Remove").clicked() {
                            remove = Some(name.clone());
                        }
                    })
                    .response;

                if ui.input(|input| input.pointer.button_down(PointerButton::Primary)) {
                    // DragNDrop
                    if let Some(mouse) = ui.input(|input| input.pointer.hover_pos()) {
                        let last_mouse = mouse - ui.input(|input| input.pointer.delta());
                        if response
                            .rect
                            .expand2(ui.spacing().item_spacing)
                            .contains(last_mouse)
                        {
                            dragged_property = Some(index);
                        } else if response
                            .rect
                            .expand2(ui.spacing().item_spacing)
                            .contains(mouse)
                        {
                            drag_target = Some(index);
                        }
                    }
                }
            }
            if let (Some(dragged_property), Some(drag_target)) = (dragged_property, drag_target) {
                component
                    .properties
                    .swap_indices(dragged_property, drag_target);
            }
            if let Some(name) = remove {
                component.properties.remove(&name);
            }
            if view.adding.is_some() {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut view.adding.as_mut().unwrap().0);
                    property_type_select(ui, &mut view.adding.as_mut().unwrap().1, "New Property");
                });
                if let Some(accepted) = ok_cancel(ui) {
                    if accepted {
                        component.properties.insert(
                            view.adding.as_mut().unwrap().0.clone(),
                            view.adding.as_mut().unwrap().1.clone(),
                        );
                    }
                    view.adding = None;
                }
            } else if ui.button("Add Property").clicked() {
                view.adding = Some(("".to_owned(), Property::String));
            }
        }
    }

    // * Object
    if let Some(object) = assets.object_selected {
        let map_names = assets
            .maps
            .iter()
            .map(|(uuid, map)| {
                (
                    *uuid,
                    map.path.file_stem().unwrap().to_str().unwrap().to_owned(),
                )
            })
            .collect::<Vec<_>>();

        let map = assets
            .maps
            .get_mut(
                &assets
                    .map_selected
                    .as_ref()
                    .context(
                        "[PROBABLY A BUG] How can no map be selected when object is selected?",
                    )?
                    .map,
            )
            .context("[PROBABLY A BUG] Selected map was not found! Perhaps it was deleted?")?;
        let object = map
            .objects
            .get_mut(&object)
            .context("[PROBABLY A BUG] Selected object was not found! Perhaps it was deleted?")?;
        let tile_size = assets
            .atlases
            .get(&map.atlas)
            .context(
                "[PROBABLY A BUG] Atlas for selected map was not found! Perhaps it was deleted?",
            )?
            .tile_size;

        let pos = (object.pos.casted() as F32Vec2).component_div(&tile_size.casted());
        ui.label(format!("Object at ({:.2}; {:.2})", pos.x, pos.y));
        ui.checkbox(&mut object.always_on_top, "Always on top");
        let mut removed_component = None;
        for (uuid, properties) in object.components.iter_mut() {
            ui.separator();
            let component = assets
                .components
                .get(uuid)
                .context("[PROBABLY A BUG] Component attached to object was not found! Perhaps it was deleted?")?;

            component.fix_instance(
                properties,
                assets.atlases.keys().copied(),
                map_names.iter().map(|(uuid, _)| *uuid),
            );

            ui.horizontal(|ui| {
                ui.label(component.path.file_stem().unwrap().to_str().unwrap());
                if ui.button("Remove").clicked() {
                    removed_component = Some(*uuid);
                }
            });
            for (name, property_type) in component.properties.iter() {
                ui.horizontal(|ui| -> Result<()> {
                    ui.label(name);
                    let value = properties.get_mut(name).unwrap();
                    property_value_input(
                        ui,
                        assets.atlases.iter().map(|(uuid, atlas)| {
                            (
                                *uuid,
                                atlas.path.file_stem().unwrap().to_str().unwrap().to_owned(),
                            )
                        }),
                        map_names.iter().cloned(),
                        name,
                        property_type,
                        value,
                    )?;
                    Ok(())
                })
                .inner?;
            }
        }
        if let Some(removed_component) = removed_component {
            object.components.remove(&removed_component);
        }

        // Add component
        if let Some(Some(uuid)) = ComboBox::from_id_source("add_component")
            .selected_text("Add Component")
            .show_ui(ui, |ui| {
                let mut selection = None;
                for (uuid, component) in assets.components.iter() {
                    ui.selectable_value(
                        &mut selection,
                        Some(uuid),
                        component.path.file_stem().unwrap().to_str().unwrap(),
                    );
                }
                selection
            })
            .inner
        {
            object.components.insert(*uuid, {
                let component = assets.components.get(uuid).unwrap();
                let mut properties = component
                    .properties
                    .iter()
                    .map(|(name, _property_type)| (name.clone(), "".to_owned()))
                    .collect();
                component.fix_instance(
                    &mut properties,
                    assets.atlases.keys().copied(),
                    map_names.iter().map(|(uuid, _)| *uuid),
                );
                properties
            });
        }
    }

    Ok(())
}

// * -------------------------------------------------------------------------------- PROPERTY -------------------------------------------------------------------------------- * //
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Hash)]
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
    Map,
}

impl Property {
    pub const VALUES: [Self; 10] = [
        Self::I8,
        Self::U8,
        Self::I16,
        Self::U16,
        Self::I32,
        Self::U32,
        Self::F32,
        Self::String,
        Self::Atlas,
        Self::Map,
    ];

    pub fn fix_value(
        &self,
        valid_atalses: impl Iterator<Item = Uuid>,
        valid_maps: impl Iterator<Item = Uuid>,
        value: &str,
    ) -> String {
        fn fix_uuid(mut source: impl Iterator<Item = Uuid>, value: &str) -> String {
            if let Ok(uuid) = Uuid::parse_str(value) {
                if !source.contains(&uuid) {
                    source.next().unwrap().to_string()
                } else {
                    value.to_owned()
                }
            } else {
                source.next().unwrap().to_string()
            }
        }

        match self {
            Property::I8 => value.parse::<i8>().unwrap_or(0).to_string(),
            Property::U8 => value.parse::<u8>().unwrap_or(0).to_string(),
            Property::I16 => value.parse::<i16>().unwrap_or(0).to_string(),
            Property::U16 => value.parse::<u16>().unwrap_or(0).to_string(),
            Property::I32 => value.parse::<i32>().unwrap_or(0).to_string(),
            Property::U32 => value.parse::<u32>().unwrap_or(0).to_string(),
            Property::F32 => value.parse::<f32>().unwrap_or(0.0).to_string(),
            Property::Atlas => fix_uuid(valid_atalses, value),
            Property::Map => fix_uuid(valid_maps, value),
            Property::String => value.to_owned(),
        }
    }
}

pub fn pick_uuid(
    ui: &mut Ui,
    names: impl Iterator<Item = (Uuid, String)> + Clone,
    id: impl std::hash::Hash,
    value: &mut Uuid,
) -> Result<()> {
    ComboBox::from_id_source(id)
        .selected_text(names.clone().find(|(uuid, _)| uuid == value).context(
            "[PROBABLY A BUG] Atlas/Map from property was not found! Perhaps it was deleted?",
        )?.1)
        .show_ui(ui, |ui| {
            for (uuid, name) in names.into_iter().sorted_by_key(|(_, name)| name.clone()) {
                ui.selectable_value(value, uuid, name);
            }
        });
    Ok(())
}

fn property_value_input(
    ui: &mut Ui,
    atlas_names: impl Iterator<Item = (Uuid, String)> + Clone,
    map_names: impl Iterator<Item = (Uuid, String)> + Clone,
    id: impl std::hash::Hash,
    property_type: &Property,
    value: &mut String,
) -> Result<()> {
    macro_rules! numeric_input {
        ($type: ty) => {{
            let mut numeric_value = value.parse::<$type>().unwrap_or(0 as _);
            ui.add(egui::DragValue::new(&mut numeric_value).speed(0.05));
            *value = numeric_value.to_string();
        }};
    }

    match property_type {
        Property::I8 => numeric_input!(i8),
        Property::U8 => numeric_input!(u8),
        Property::I16 => numeric_input!(i16),
        Property::U16 => numeric_input!(u16),
        Property::I32 => numeric_input!(i32),
        Property::U32 => numeric_input!(u32),
        Property::F32 => numeric_input!(f32),
        Property::String => {
            ui.text_edit_singleline(value);
        }
        Property::Atlas => {
            let mut uuid = Uuid::parse_str(value)
                .context("[PROBABLY A BUG] Atlas/Map UUID from property was invalid!")?;
            pick_uuid(ui, atlas_names, id, &mut uuid)?;
            *value = uuid.to_string();
        }
        Property::Map => {
            let mut uuid = Uuid::parse_str(value)
                .context("[PROBABLY A BUG] Atlas/Map UUID from property was invalid!")?;
            pick_uuid(ui, map_names, id, &mut uuid)?;
            *value = uuid.to_string();
        }
    }

    Ok(())
}
