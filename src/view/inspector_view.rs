use itertools::Itertools;

use crate::project::*;
use std::collections::HashMap;

pub fn show(ui: &mut Ui, assets: &mut Assets) -> Result<()> {
    // * Component
    if let Some(component_view) = &mut assets.component_selected {
        if let Some(component) = assets.components.get_mut(&component_view.component) {
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
                component.path.file_name().unwrap().to_str().unwrap()
            ));
            ui.separator();
            let mut remove = None;
            for (name, property_type) in component.properties.iter_mut() {
                ui.horizontal(|ui| {
                    ui.label(name);
                    property_type_select(ui, property_type, name);
                    if ui.button("Remove").clicked() {
                        remove = Some(name.clone());
                    }
                });
            }
            if let Some(name) = remove {
                component.properties.remove(&name);
            }
            if component_view.adding.is_some() {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut component_view.adding.as_mut().unwrap().0);
                    property_type_select(
                        ui,
                        &mut component_view.adding.as_mut().unwrap().1,
                        "New Property",
                    );
                });
                if let Some(accepted) = ok_cancel(ui) {
                    if accepted {
                        component.properties.insert(
                            component_view.adding.as_mut().unwrap().0.clone(),
                            component_view.adding.as_mut().unwrap().1.clone(),
                        );
                    }
                    component_view.adding = None;
                }
            } else if ui.button("Add Property").clicked() {
                component_view.adding = Some(("".to_owned(), Property::String));
            }
        }
    }

    // * Object
    if let Some(object) = assets.object_selected {
        let atlas_names = assets
            .atlases
            .iter()
            .map(|(uuid, atlas)| {
                (
                    *uuid,
                    atlas.path.file_name().unwrap().to_str().unwrap().to_owned(),
                )
            })
            .collect::<HashMap<Uuid, String>>();
        let map_names = assets
            .maps
            .iter()
            .map(|(uuid, map)| {
                (
                    *uuid,
                    map.path.file_name().unwrap().to_str().unwrap().to_owned(),
                )
            })
            .collect::<HashMap<Uuid, String>>();

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

        let pos = (object.pos.as_type() as F32Vec2).component_div(&tile_size.as_type());
        ui.label(format!("Object at ({:.2}; {:.2})", pos.x, pos.y));
        for (uuid, properties) in object.components.iter_mut() {
            ui.separator();
            let component = assets
                .components
                .get(uuid)
                .context("[PROBABLY A BUG] Component attached to object was not found! Perhaps it was deleted?")?;

            component.fix_instance(
                properties,
                &atlas_names.keys().copied().collect(),
                &map_names.keys().copied().collect(),
            );
            for (name, property_type) in component.properties.iter() {
                ui.horizontal(|ui| -> Result<()> {
                    ui.label(name);
                    let value = properties.get_mut(name).unwrap();
                    property_input(ui, &atlas_names, &map_names, name, property_type, value)?;
                    Ok(())
                })
                .inner?;
            }
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
                        component.path.file_name().unwrap().to_str().unwrap(),
                    );
                }
                selection
            })
            .inner
        {
            object.components.insert(*uuid, {
                let component = assets.components.get(&uuid).unwrap();
                let mut properties = component
                    .properties
                    .iter()
                    .map(|(name, _property_type)| (name.clone(), "".to_owned()))
                    .collect();
                component.fix_instance(
                    &mut properties,
                    &atlas_names.keys().copied().collect(),
                    &map_names.keys().copied().collect(),
                );
                properties
            });
        }
    }

    Ok(())
}

// ********************* SPECIAL WIDGETS ******************** ///
fn property_input(
    ui: &mut Ui,
    atlas_names: &HashMap<Uuid, String>,
    _map_names: &HashMap<Uuid, String>,
    id: impl std::hash::Hash,
    property_type: &Property,
    value: &mut String,
) -> Result<()> {
    match property_type {
        Property::I8
        | Property::U8
        | Property::I16
        | Property::U16
        | Property::I32
        | Property::U32
        | Property::F32 => {
            let response = ui.text_edit_singleline(value);
            if response.changed() {
                if matches!(property_type, Property::F32) {
                    value.retain(|ch| ch.is_ascii_digit() || ch == '.');
                    if let Some(dot_index) = value.find('.') {
                        value.replace_range(
                            dot_index + 1..,
                            value[dot_index + 1..]
                                .chars()
                                .filter(|ch| *ch != '.')
                                .collect::<String>()
                                .as_str(),
                        );
                    }
                } else {
                    value.retain(|ch| ch.is_ascii_digit());
                }
            }
        }
        Property::String => {
            ui.text_edit_singleline(value);
        }
        Property::Atlas => {
            ComboBox::from_id_source(id)
                .selected_text(atlas_names.get(&Uuid::parse_str(value).context("[PROBABLY A BUG] Atlas UUID from property was invalid!")?).context(
                    "[PROBABLY A BUG] Atlas from property was not found! Perhaps it was deleted?",
                )?)
                .show_ui(ui, |ui| {
                    for (uuid, atlas_name) in atlas_names.iter().sorted_by_key(|(_, name)| name.clone()) {
                        ui.selectable_value(value, uuid.to_string(), atlas_name);
                    }
                });
        }
    }

    Ok(())
}
