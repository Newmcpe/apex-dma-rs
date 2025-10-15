use eframe::{Frame, egui};
use egui::{Align2, Color32, Context, FontData, FontDefinitions, FontId, Key, pos2};
use std::fs;
use std::time::{Duration, Instant};
use tokio::sync::watch;

use crate::types::Snapshot;
use crate::utils::world_to_screen;
use glam::Vec3;

pub struct OverlayApp {
    rx: watch::Receiver<Snapshot>,
    local_player: u64,
    last_report: Instant,
    frame_counter: u32,
    redraws_per_sec: u32,
}

impl OverlayApp {
    pub fn new(rx: watch::Receiver<Snapshot>, local_player: u64) -> Self {
        Self {
            rx,
            local_player,
            last_report: Instant::now(),
            frame_counter: 0,
            redraws_per_sec: 0,
        }
    }

    fn setup_custom_fonts(ctx: &Context) {
        let mut fonts = FontDefinitions::default();

        // Load Harmony OS font
        if let Ok(font_data) = fs::read("assets/HarmonyOS_Sans_Regular.ttf") {
            fonts
                .font_data
                .insert("harmony_os".to_owned(), FontData::from_owned(font_data));

            // Use Harmony OS as the default proportional font
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "harmony_os".to_owned());

            // Also use it for monospace if needed
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push("harmony_os".to_owned());
        }

        ctx.set_fonts(fonts);
    }

    fn get_harmony_font(size: f32) -> FontId {
        FontId::new(size, egui::FontFamily::Proportional)
    }
}

impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        Self::setup_custom_fonts(ctx);

        if ctx.input(|i| i.key_pressed(Key::F11)) {
            let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.is_some_and(|x| x));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
        }

        self.frame_counter = self.frame_counter.saturating_add(1);
        if self.last_report.elapsed() >= Duration::from_secs(1) {
            self.redraws_per_sec = self.frame_counter;
            self.frame_counter = 0;
            self.last_report = Instant::now();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let snapshot = self.rx.borrow_and_update();
                let view = snapshot.view;
                let entities = &snapshot.entities;
                let painter = ui.painter();

                let fps = {
                    let dt = ctx.input(|i| i.stable_dt);
                    if dt > 0.0 { 1.0 / dt } else { 0.0 }
                };
                let rect = ui.max_rect();
                let fps_pos = pos2(rect.max.x - 10.0, rect.min.y + 10.0);
                painter.text(
                    fps_pos,
                    Align2::RIGHT_TOP,
                    format!("FPS: {:.0}", fps),
                    Self::get_harmony_font(16.0),
                    Color32::from_rgb(120, 255, 120),
                );

                // Redraws per second (just below FPS)
                let redraws_pos = fps_pos + egui::vec2(0.0, 18.0);
                painter.text(
                    redraws_pos,
                    Align2::RIGHT_TOP,
                    format!("Redraws: {}/s", self.redraws_per_sec),
                    Self::get_harmony_font(16.0),
                    Color32::from_rgb(120, 200, 255),
                );

                for (idx, e) in entities.iter().enumerate() {
                    if e.base == self.local_player {
                        continue;
                    }
                    // Draw AABB-based shaped bounding box with health bar and name
                    if idx < snapshot.aabbs.len() {
                        let (mins, maxs) = snapshot.aabbs[idx];
                        let corners = [
                            // bottom rectangle (z = mins.z)
                            Vec3::new(mins.x, mins.y, mins.z),
                            Vec3::new(maxs.x, mins.y, mins.z),
                            Vec3::new(maxs.x, maxs.y, mins.z),
                            Vec3::new(mins.x, maxs.y, mins.z),
                            // top rectangle (z = maxs.z)
                            Vec3::new(mins.x, mins.y, maxs.z),
                            Vec3::new(maxs.x, mins.y, maxs.z),
                            Vec3::new(maxs.x, maxs.y, maxs.z),
                            Vec3::new(mins.x, maxs.y, maxs.z),
                        ];
                        let mut pts = [None; 8];
                        for i in 0..8 {
                            pts[i] = world_to_screen(corners[i], view, 1920.0, 1080.0);
                        }
                        // Build 2D bounding rectangle from projected corners
                        let mut min_x = f32::INFINITY;
                        let mut min_y = f32::INFINITY;
                        let mut max_x = f32::NEG_INFINITY;
                        let mut max_y = f32::NEG_INFINITY;
                        let mut any = false;
                        for p in &pts {
                            if let Some(pt) = p {
                                any = true;
                                if pt.x < min_x {
                                    min_x = pt.x;
                                }
                                if pt.y < min_y {
                                    min_y = pt.y;
                                }
                                if pt.x > max_x {
                                    max_x = pt.x;
                                }
                                if pt.y > max_y {
                                    max_y = pt.y;
                                }
                            }
                        }
                        if any {
                            let w = (max_x - min_x).max(1.0);
                            let h = (max_y - min_y).max(1.0);
                            let tl = pos2(min_x, min_y);
                            let tr = pos2(max_x, min_y);
                            let bl = pos2(min_x, max_y);
                            let br = pos2(max_x, max_y);
                            let color = Color32::from_rgb(0, 200, 255);
                            let corner = (w.min(h) * 0.25).clamp(6.0, 20.0);

                            // Corner box (8 segments)
                            painter.line_segment([tl, pos2(tl.x + corner, tl.y)], (2.0, color));
                            painter.line_segment([tl, pos2(tl.x, tl.y + corner)], (2.0, color));

                            painter.line_segment([tr, pos2(tr.x - corner, tr.y)], (2.0, color));
                            painter.line_segment([tr, pos2(tr.x, tr.y + corner)], (2.0, color));

                            painter.line_segment([bl, pos2(bl.x + corner, bl.y)], (2.0, color));
                            painter.line_segment([bl, pos2(bl.x, bl.y - corner)], (2.0, color));

                            painter.line_segment([br, pos2(br.x - corner, br.y)], (2.0, color));
                            painter.line_segment([br, pos2(br.x, br.y - corner)], (2.0, color));

                            // Name at top center
                            let top_center = pos2((tl.x + tr.x) * 0.5, tl.y - 14.0);
                            painter.text(
                                top_center,
                                Align2::CENTER_BOTTOM,
                                e.name.as_str(),
                                Self::get_harmony_font(14.0),
                                Color32::WHITE,
                            );

                            // Health bar at left side
                            let bar_w = 4.0;
                            let ratio = (e.health as f32 / 100.0).clamp(0.0, 1.0);
                            let filled_h = h * ratio;
                            let x0 = tl.x - 6.0;
                            painter.rect_stroke(
                                egui::Rect::from_min_max(pos2(x0 - bar_w, tl.y), pos2(x0, bl.y)),
                                0.0,
                                (1.0, Color32::from_rgb(30, 30, 30)),
                            );
                            painter.rect_filled(
                                egui::Rect::from_min_max(
                                    pos2(x0 - bar_w + 1.0, bl.y - filled_h + 1.0),
                                    pos2(x0 - 1.0, bl.y - 1.0),
                                ),
                                0.0,
                                Color32::from_rgb(80, 220, 120),
                            );
                        }
                    }
                }
            });
        ctx.request_repaint();
    }
}
