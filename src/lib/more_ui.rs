use egui::*;

pub fn form<R, F: FnOnce(&mut Ui) -> R>(ui: &mut Ui, title: &str, function: F) -> R {
    Window::new(title)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ui.ctx(), function)
        .unwrap()
        .inner
        .unwrap()
}

pub fn ok_cancel(ui: &mut Ui) -> Option<bool> {
    ui.horizontal(|ui| {
        if ui.button("Ok").clicked() || ui.input(|input| input.key_pressed(Key::Enter)) {
            Some(true)
        } else if ui.button("Cancel").clicked() || ui.input(|input| input.key_pressed(Key::Escape))
        {
            Some(false)
        } else {
            None
        }
    })
    .inner
}
