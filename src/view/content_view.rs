use crate::project::*;
use itertools::Itertools;

pub fn show(ui: &mut Ui, assets: &mut Assets) -> Result<()> {
    ScrollArea::vertical()
        .show(ui, |ui| {
            ui.with_layout(
                Layout::top_down_justified(Align::LEFT),
                |ui| {
                    if assets.content_viewer_path != assets.path {
                        if ui.button("<-").clicked() {
                            assets.content_viewer_path.pop();
                        }
                    }

                    for path in std::fs::read_dir(assets.content_viewer_path.clone())
                        .context("[PROBABLY A BUG] Failed to read project dir!")?
                        .sorted_by_key(|file| file.as_ref().unwrap().file_name())
                    {
                        let file = path.unwrap();
                        let path = file.path();
                        if let Some(extension) = path.extension() {
                            if extension == "atl" || extension == "dat" {
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
                                        Atlas::new(&path).map(|atlas| {
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
                        if assets.new_component_name.is_some() {
                            form(ui, "New Component", |ui| -> Result<()> {
                                ui.add(
                                    egui::TextEdit::singleline(
                                        assets.new_component_name.as_mut().unwrap(),
                                    )
                                    .desired_width(ui.available_width()),
                                );

                                if let Some(accepted) = ok_cancel(ui) {
                                    if accepted {
                                        let uuid = Uuid::new_v4();
                                        let path = assets.content_viewer_path.join(format!(
                                            "{}.cmp",
                                            assets.new_component_name.as_ref().unwrap()
                                        ));
                                        let component = Component::new(path.clone());
                                        component.save()?;
                                        assets.components.insert(uuid, component);
                                        assets.uuids.insert(path, uuid);
                                    }
                                    assets.new_component_name = None;
                                }
                                Ok(())
                            })?;
                        }

                        if ui.button("New Component").clicked() {
                            assets.new_component_name = Some("".to_owned());
                        }
                        Ok(())
                    })
                    .inner?;
                    Ok(())
                },
            )
            .inner
        })
        .inner
}
