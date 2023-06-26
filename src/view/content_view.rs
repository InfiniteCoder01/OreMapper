use crate::project::*;
use itertools::Itertools;

pub fn show(ui: &mut Ui, assets: &mut Assets) -> Result<()> {
    ScrollArea::vertical()
        .show(ui, |ui| {
            ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                if assets.content_viewer_path != assets.path && ui.button("<-").clicked() {
                    assets.content_viewer_path.pop();
                }

                for path in std::fs::read_dir(assets.content_viewer_path.clone())
                    .context("[PROBABLY A BUG] Failed to read project dir!")?
                    .sorted_by_key(|file| file.as_ref().unwrap().file_name())
                {
                    let file = path.unwrap();
                    let path = file.path();
                    if let Some(extension) = path.extension() {
                        if extension == "atl" || extension == "json" {
                            continue;
                        }
                    }

                    if ui.button(file.file_name().to_str().unwrap()).clicked() {
                        if file
                            .file_type()
                            .context("[PROBABLY A BUG] Failed to get file type!")?
                            .is_dir()
                        {
                            assets.content_viewer_path.push(file.file_name());
                        }
                        if let Some(extension) = path.extension() {
                            if extension == "png" {
                                // Atlas
                                let uuid = if let Some(uuid) = assets.uuids.get(&path) {
                                    Ok(*uuid)
                                } else {
                                    Atlas::load(&path).map(|atlas| {
                                        let uuid = Uuid::new_v4();
                                        assets.atlases.insert(uuid, atlas);
                                        assets.uuids.insert(path.clone(), uuid);
                                        uuid
                                    })
                                }?;

                                assets.atlas_selected = Some(AtlasView::new(uuid));
                            } else if extension == "map" {
                                // Map
                                let uuid = if let Some(uuid) = assets.uuids.get(&path) {
                                    Ok(*uuid)
                                } else {
                                    Map::load(&path).map(|map| {
                                        let uuid = Uuid::new_v4();
                                        assets.maps.insert(uuid, map);
                                        assets.uuids.insert(path.clone(), uuid);
                                        uuid
                                    })
                                }?;

                                assets.map_selected = Some(MapView::new(uuid));
                                assets.object_selected = None;
                            } else if extension == "cmp" {
                                // Component
                                let uuid = if let Some(uuid) = assets.uuids.get(&path) {
                                    Ok(*uuid)
                                } else {
                                    Component::load(&path).map(|component| {
                                        let uuid = Uuid::new_v4();
                                        assets.components.insert(uuid, component);
                                        assets.uuids.insert(path.clone(), uuid);
                                        uuid
                                    })
                                }?;

                                assets.component_selected = Some(ComponentView::new(uuid));
                                assets.object_selected = None;
                            }
                        }
                    }
                }

                // Create new map/component
                ui.horizontal(|ui| -> Result<()> {
                    macro_rules! numeric_field {
                        ($ui: ident, $assets: ident, $field: literal, $value: expr) => {
                            $ui.horizontal(|ui| {
                                ui.label($field);
                                ui.add(egui::DragValue::new(&mut $value).speed(0.05));
                            });
                        };
                    }

                    if assets.new_map.is_some() {
                        form(ui, "New Map", |ui| -> Result<()> {
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut assets.new_map.as_mut().unwrap().name,
                                )
                                .desired_width(ui.available_width()),
                            );

                            numeric_field!(
                                ui,
                                assets,
                                "Width",
                                assets.new_map.as_mut().unwrap().size.x
                            );
                            numeric_field!(
                                ui,
                                assets,
                                "Height",
                                assets.new_map.as_mut().unwrap().size.y
                            );
                            crate::view::inspector_view::pick_uuid(
                                ui,
                                assets.atlases.iter().map(|(uuid, atlas)| {
                                    (
                                        *uuid,
                                        atlas
                                            .path
                                            .file_stem()
                                            .unwrap()
                                            .to_str()
                                            .unwrap()
                                            .to_owned(),
                                    )
                                }),
                                "Pick atlas for map",
                                &mut assets.new_map.as_mut().unwrap().atlas,
                            )
                            .context("While constructing new map")?;

                            if let Some(accepted) = ok_cancel(ui) {
                                if accepted {
                                    let new_map = assets.new_map.as_ref().unwrap();
                                    let uuid = Uuid::new_v4();
                                    let path = assets
                                        .content_viewer_path
                                        .join(format!("{}.map", new_map.name));
                                    let map = Map::new(&path, new_map.size, new_map.atlas);
                                    map.save()?;
                                    assets.maps.insert(uuid, map);
                                    assets.uuids.insert(path, uuid);
                                }
                                assets.new_map = None;
                            }
                            Ok(())
                        })?;
                    }

                    if assets.new_component_name.is_some() {
                        form(ui, "New Component", |ui| -> Result<()> {
                            let new_component_name = assets.new_component_name.as_mut().unwrap();
                            ui.add(
                                egui::TextEdit::singleline(new_component_name)
                                    .desired_width(ui.available_width()),
                            );

                            if let Some(accepted) = ok_cancel(ui) {
                                if accepted {
                                    let uuid = Uuid::new_v4();
                                    let path = assets
                                        .content_viewer_path
                                        .join(format!("{}.cmp", new_component_name));
                                    let component = Component::new(&path);
                                    component.save()?;
                                    assets.components.insert(uuid, component);
                                    assets.uuids.insert(path, uuid);
                                }
                                assets.new_component_name = None;
                            }
                            Ok(())
                        })?;
                    }

                    if !assets.atlases.is_empty() && ui.button("New Map").clicked() {
                        assets.new_map = Some(NewMap {
                            atlas: *assets.atlases.iter().next().unwrap().0,
                            ..NewMap::default()
                        });
                    }

                    if ui.button("New Component").clicked() {
                        assets.new_component_name = Some("".to_owned());
                    }

                    Ok(())
                })
                .inner
            })
            .inner
        })
        .inner
}
