use std::sync::{Arc, Mutex};

use crate::scene_view::*;

pub enum Pane {
    SceneView(SceneView),
    Parameters(Arc<Mutex<SceneViewState>>),
    Stats(Arc<Mutex<SceneViewState>>),
}
impl Pane {
    pub fn create_tree(scene_view: SceneView) -> egui_tiles::Tree<Pane> {
        let mut tiles = egui_tiles::Tiles::default();

        let right_tabs = [
            tiles.insert_pane(Pane::Stats(scene_view.state())),
            tiles.insert_pane(Pane::Parameters(scene_view.state())),
        ];
        let right_linear =
            egui_tiles::Linear::new_binary(egui_tiles::LinearDir::Vertical, right_tabs, 0.2);
        let right_container = egui_tiles::Container::from(right_linear);
        let right = tiles.insert_container(right_container);

        let root_items = [
            tiles.insert_pane(Pane::SceneView(scene_view.clone())),
            right,
        ];
        let root_linear =
            egui_tiles::Linear::new_binary(egui_tiles::LinearDir::Horizontal, root_items, 0.7);
        let root_container = egui_tiles::Container::from(root_linear);
        let root = tiles.insert_container(root_container);

        egui_tiles::Tree::new("root", root, tiles)
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
    ) -> egui_tiles::UiResponse {
        match self {
            Pane::SceneView(scene_view) => {
                ui.add(scene_view);
            }
            Pane::Parameters(state) => {
                let mut state = state.lock().unwrap();

                ui.add_space(8.0);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.add_space(12.0);
                    egui::Grid::new("parameters_grid")
                        .spacing(egui::vec2(16.0, 8.0))
                        .show(ui, |ui| {
                            ui.label("max sample count: ");
                            ui.add(egui::widgets::DragValue::new(&mut state.max_sample_count));
                            ui.end_row();

                            ui.label("fit view");
                            ui.add(egui::widgets::Checkbox::without_text(&mut state.fit_view));
                            ui.end_row();

                            ui.label("size: ");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.add(egui::widgets::DragValue::new(&mut state.width));
                                ui.label("x");
                                ui.add(egui::widgets::DragValue::new(&mut state.height));
                            });
                            ui.end_row();

                            ui.label("camera position: ");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.add(egui::widgets::DragValue::new(&mut state.position_x));
                                ui.add(egui::widgets::DragValue::new(&mut state.position_y));
                                ui.add(egui::widgets::DragValue::new(&mut state.position_z));
                            });
                            ui.end_row();

                            ui.label("camera rotate: ");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.add(egui::widgets::DragValue::new(&mut state.rotate_x));
                                ui.add(egui::widgets::DragValue::new(&mut state.rotate_y));
                                ui.add(egui::widgets::DragValue::new(&mut state.rotate_z));
                            });
                            ui.end_row();

                            ui.label("field of view: ");
                            ui.add(egui::widgets::DragValue::new(&mut state.fov));
                            state.fov = state.fov.clamp(1.0, 179.0);
                            ui.end_row();

                            ui.label("L_white: ");
                            ui.add(egui::widgets::DragValue::new(&mut state.l_white));
                            state.l_white = state.l_white.max(0.01);
                            ui.end_row();

                            ui.label("exposure: ");
                            ui.add(egui::widgets::DragValue::new(&mut state.exposure));
                            state.exposure = state.exposure.max(0.0001);
                            ui.end_row();

                            ui.label("max recursion depth: ");
                            ui.add(egui::widgets::DragValue::new(
                                &mut state.max_recursion_depth,
                            ));
                            state.max_recursion_depth = state.max_recursion_depth.clamp(1, 64);
                            ui.end_row();

                            ui.label("sun direction: ");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.add(egui::widgets::DragValue::new(&mut state.sun_direction.x));
                                ui.add(egui::widgets::DragValue::new(&mut state.sun_direction.y));
                            });
                            ui.end_row();

                            ui.label("sun angle: ");
                            ui.add(egui::widgets::DragValue::new(&mut state.sun_angle));
                            ui.end_row();

                            ui.label("sun strength: ");
                            ui.add(egui::widgets::DragValue::new(&mut state.sun_strength));
                            state.sun_strength = state.sun_strength.max(0.0);
                            ui.end_row();

                            ui.label("sun color: ");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                let mut rgb = state.sun_color.into();
                                ui.color_edit_button_rgb(&mut rgb);
                                state.sun_color = rgb.into();
                            });
                            ui.end_row();

                            ui.label("sun enabled: ");
                            let mut sun_enabled = state.sun_enabled != 0;
                            ui.add(egui::widgets::Checkbox::without_text(&mut sun_enabled));
                            state.sun_enabled = sun_enabled as u32;
                            ui.end_row();
                        });
                    ui.add_space(12.0);
                });
            }
            Pane::Stats(state) => {
                let state = state.lock().unwrap();

                ui.add_space(8.0);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.add_space(12.0);
                    egui::Grid::new("stats_grid")
                        .spacing(egui::vec2(16.0, 8.0))
                        .show(ui, |ui| {
                            ui.label("sample count");
                            ui.label(format!("{}", state.sample_count));
                            ui.end_row();

                            ui.label("size");
                            ui.label(format!("{}x{}", state.width, state.height));
                            ui.end_row();
                        });
                });
            }
        }
        Default::default()
    }

    pub fn title(&self) -> egui::WidgetText {
        match self {
            Pane::SceneView(_) => "Scene View".into(),
            Pane::Parameters(_) => "Parameters".into(),
            Pane::Stats(_) => "Stats".into(),
        }
    }
}
