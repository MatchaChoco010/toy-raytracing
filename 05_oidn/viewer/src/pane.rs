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
            egui_tiles::Linear::new_binary(egui_tiles::LinearDir::Vertical, right_tabs, 0.15);
        let right_container = egui_tiles::Container::from(right_linear);
        let right = tiles.insert_container(right_container);

        let root_items = [
            tiles.insert_pane(Pane::SceneView(scene_view.clone())),
            right,
        ];
        let root_linear =
            egui_tiles::Linear::new_binary(egui_tiles::LinearDir::Horizontal, root_items, 0.75);
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

                let margin = egui::Margin::symmetric(12.0, 8.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Frame::none().inner_margin(margin).show(ui, |ui| {
                        ui.heading("RayTracing Settings");
                        ui.add_space(8.0);
                        egui::Grid::new("ray_tracing_settings_parameters_grid")
                            .spacing(egui::vec2(16.0, 8.0))
                            .show(ui, |ui| {
                                ui.label("max sample count: ");
                                ui.add(egui::widgets::DragValue::new(&mut state.max_sample_count));
                                ui.end_row();

                                ui.label("max recursion depth: ");
                                ui.add(egui::widgets::DragValue::new(
                                    &mut state.max_recursion_depth,
                                ));
                                state.max_recursion_depth = state.max_recursion_depth.clamp(1, 64);
                                ui.end_row();

                                ui.label("display image: ");
                                egui::ComboBox::from_id_source("display_image")
                                    .selected_text(format!("{:?}", state.display_image))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut state.display_image,
                                            renderer::DisplayImage::Final,
                                            "Final",
                                        );
                                        ui.selectable_value(
                                            &mut state.display_image,
                                            renderer::DisplayImage::BaseColor,
                                            "BaseColor",
                                        );
                                        ui.selectable_value(
                                            &mut state.display_image,
                                            renderer::DisplayImage::Normal,
                                            "Normal",
                                        );
                                        ui.selectable_value(
                                            &mut state.display_image,
                                            renderer::DisplayImage::Resolved,
                                            "Resolved",
                                        );
                                    });
                                ui.end_row();

                                ui.label("denoise every sample: ");
                                ui.add(egui::widgets::Checkbox::without_text(
                                    &mut state.denoise_every_sample,
                                ));
                                ui.end_row();
                            });
                    });

                    ui.separator();

                    egui::Frame::none().inner_margin(margin).show(ui, |ui| {
                        ui.heading("View Size");
                        ui.add_space(8.0);
                        egui::Grid::new("view_size_parameters_grid")
                            .spacing(egui::vec2(16.0, 8.0))
                            .show(ui, |ui| {
                                ui.label("fit view");
                                ui.add(egui::widgets::Checkbox::without_text(&mut state.fit_view));
                                ui.end_row();

                                ui.add_enabled_ui(!state.fit_view, |ui| {
                                    ui.label("size: ");
                                });
                                ui.add_enabled_ui(!state.fit_view, |ui| {
                                    ui.with_layout(
                                        egui::Layout::left_to_right(egui::Align::TOP),
                                        |ui| {
                                            ui.add(egui::widgets::DragValue::new(&mut state.width));
                                            ui.label("x");
                                            ui.add(egui::widgets::DragValue::new(
                                                &mut state.height,
                                            ));
                                        },
                                    );
                                });
                                ui.end_row();
                            });
                    });

                    ui.separator();

                    egui::Frame::none().inner_margin(margin).show(ui, |ui| {
                        ui.heading("Camera");
                        ui.add_space(8.0);
                        egui::Grid::new("camera_parameters_grid")
                            .spacing(egui::vec2(16.0, 8.0))
                            .show(ui, |ui| {
                                ui.label("camera position: ");
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.add(egui::widgets::DragValue::new(
                                            &mut state.position_x,
                                        ));
                                        ui.add(egui::widgets::DragValue::new(
                                            &mut state.position_y,
                                        ));
                                        ui.add(egui::widgets::DragValue::new(
                                            &mut state.position_z,
                                        ));
                                    },
                                );
                                ui.end_row();

                                ui.label("camera rotate: ");
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.add(egui::widgets::DragValue::new(&mut state.rotate_x));
                                        ui.add(egui::widgets::DragValue::new(&mut state.rotate_y));
                                        ui.add(egui::widgets::DragValue::new(&mut state.rotate_z));
                                    },
                                );
                                ui.end_row();

                                ui.label("field of view: ");
                                ui.add(egui::widgets::DragValue::new(&mut state.fov));
                                state.fov = state.fov.clamp(1.0, 179.0);
                                ui.end_row();

                                ui.label("L_white: ");
                                ui.add(egui::widgets::DragValue::new(&mut state.l_white));
                                state.l_white = state.l_white.max(0.01);
                                ui.end_row();

                                ui.label("aperture (f-number): ");
                                ui.add(egui::widgets::DragValue::new(&mut state.aperture));
                                state.aperture = state.aperture.clamp(1.4, 64.0);
                                ui.end_row();

                                ui.label("shutter speed: ");
                                ui.add(egui::widgets::DragValue::new(&mut state.shutter_speed));
                                state.shutter_speed = state.shutter_speed.max(0.0001);
                                ui.end_row();

                                ui.label("ISO: ");
                                ui.add(egui::widgets::DragValue::new(&mut state.iso));
                                state.iso = state.iso.max(100.0);
                                ui.end_row();
                            });
                    });

                    ui.separator();

                    egui::Frame::none().inner_margin(margin).show(ui, |ui| {
                        ui.heading("Lights");

                        egui::Frame::none().inner_margin(margin).show(ui, |ui| {
                            ui.label(egui::RichText::new("Sun").heading().size(14.0));
                            ui.add_space(4.0);
                            egui::Grid::new("sun_parameters_grid")
                                .spacing(egui::vec2(16.0, 8.0))
                                .show(ui, |ui| {
                                    ui.label("sun enabled: ");
                                    let mut sun_enabled = state.sun_enabled != 0;
                                    ui.add(egui::widgets::Checkbox::without_text(&mut sun_enabled));
                                    state.sun_enabled = sun_enabled as u32;
                                    ui.end_row();

                                    ui.add_enabled_ui(state.sun_enabled == 1, |ui| {
                                        ui.label("sun direction: ");
                                    });
                                    ui.add_enabled_ui(state.sun_enabled == 1, |ui| {
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::TOP),
                                            |ui| {
                                                ui.add(egui::widgets::DragValue::new(
                                                    &mut state.sun_direction.x,
                                                ));
                                                ui.add(egui::widgets::DragValue::new(
                                                    &mut state.sun_direction.y,
                                                ));
                                            },
                                        );
                                    });
                                    ui.end_row();

                                    ui.add_enabled_ui(state.sun_enabled == 1, |ui| {
                                        ui.label("sun angle: ");
                                    });
                                    ui.add_enabled_ui(state.sun_enabled == 1, |ui| {
                                        ui.add(egui::widgets::DragValue::new(&mut state.sun_angle));
                                    });
                                    ui.end_row();

                                    ui.add_enabled_ui(state.sun_enabled == 1, |ui| {
                                        ui.label("sun strength: ");
                                    });
                                    ui.add_enabled_ui(state.sun_enabled == 1, |ui| {
                                        ui.add(egui::widgets::DragValue::new(
                                            &mut state.sun_strength,
                                        ));
                                        state.sun_strength = state.sun_strength.max(0.0);
                                    });
                                    ui.end_row();

                                    ui.add_enabled_ui(state.sun_enabled == 1, |ui| {
                                        ui.label("sun color: ");
                                    });
                                    ui.add_enabled_ui(state.sun_enabled == 1, |ui| {
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::TOP),
                                            |ui| {
                                                let mut rgb = state.sun_color.into();
                                                ui.color_edit_button_rgb(&mut rgb);
                                                state.sun_color = rgb.into();
                                            },
                                        );
                                    });
                                    ui.end_row();
                                });
                        });

                        egui::Frame::none().inner_margin(margin).show(ui, |ui| {
                            ui.label(egui::RichText::new("Sky").heading().size(14.0));
                            ui.add_space(4.0);
                            egui::Grid::new("sky_parameters_grid")
                                .spacing(egui::vec2(16.0, 8.0))
                                .show(ui, |ui| {
                                    ui.label("sky enabled: ");
                                    let mut sky_enabled = state.sky_enabled != 0;
                                    ui.add(egui::widgets::Checkbox::without_text(&mut sky_enabled));
                                    state.sky_enabled = sky_enabled as u32;
                                    ui.end_row();

                                    ui.add_enabled_ui(state.sky_enabled == 1, |ui| {
                                        ui.label("sky rotation: ");
                                    });
                                    ui.add_enabled_ui(state.sky_enabled == 1, |ui| {
                                        ui.add(egui::widgets::DragValue::new(
                                            &mut state.sky_rotation,
                                        ));
                                    });
                                    ui.end_row();

                                    ui.add_enabled_ui(state.sky_enabled == 1, |ui| {
                                        ui.label("sky strength: ");
                                    });
                                    ui.add_enabled_ui(state.sky_enabled == 1, |ui| {
                                        ui.add(egui::widgets::DragValue::new(
                                            &mut state.sky_strength,
                                        ));
                                        state.sun_strength = state.sun_strength.max(0.0);
                                    });
                                    ui.end_row();
                                });
                        });
                    });
                });
            }
            Pane::Stats(state) => {
                let state = state.lock().unwrap();

                let margin = egui::Margin::symmetric(12.0, 8.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Frame::none().inner_margin(margin).show(ui, |ui| {
                        ui.heading("Render Stats");
                        ui.add_space(8.0);
                        egui::Grid::new("stats_grid")
                            .spacing(egui::vec2(16.0, 4.0))
                            .show(ui, |ui| {
                                ui.label("sample count");
                                ui.label(format!("{}", state.sample_count));
                                ui.end_row();

                                ui.label("size");
                                ui.label(format!("{}x{}", state.width, state.height));
                                ui.end_row();

                                ui.label("rendering time");
                                ui.label(format!("{:.3}s", state.rendering_time.as_secs_f64()));
                                ui.end_row();
                            });
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
