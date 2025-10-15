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
                let local_origin = entities
                    .iter()
                    .find(|en| en.base == self.local_player)
                    .map(|en| en.origin);
                let painter = ui.painter();
                let screen_w = ui.max_rect().width();
                let screen_h = ui.max_rect().height();

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

                for (_idx, e) in entities.iter().enumerate() {
                    if e.base == self.local_player {
                        continue;
                    }
                    // Corner box using origin only (no AABB): assume Z+ is up
                    {
                        const HEAD_OFFSET: f32 = 72.0; // tweak if needed
                        let head3d = e.origin + Vec3::new(0.0, 0.0, HEAD_OFFSET);
                        let feet3d = e.origin;
                        if let (Some(head2d), Some(feet2d)) = (
                            world_to_screen(head3d, view, screen_w, screen_h),
                            world_to_screen(feet3d, view, screen_w, screen_h),
                        ) {
                            let h = (feet2d.y - head2d.y).abs().max(1.0);
                            let base_w = (h * 0.45).max(8.0);
                            let distance_scale = if let Some(lo) = local_origin {
                                let d = e.origin.distance(lo).max(1.0);
                                (400.0 / d).clamp(0.6, 1.8)
                            } else {
                                1.0
                            };
                            let w = base_w * distance_scale;
                            let cx = feet2d.x;
                            let top_y = head2d.y.min(feet2d.y);
                            let bot_y = head2d.y.max(feet2d.y);

                            let tl = pos2(cx - w * 0.5, top_y);
                            let tr = pos2(cx + w * 0.5, top_y);
                            let bl = pos2(cx - w * 0.5, bot_y);
                            let br = pos2(cx + w * 0.5, bot_y);
                            let color = Color32::from_rgb(0, 200, 255);
                            let corner = (w.min(h) * 0.25 * distance_scale).clamp(4.0, 20.0);
                            let thickness = (2.0 * distance_scale).clamp(1.0, 3.0);

                            painter
                                .line_segment([tl, pos2(tl.x + corner, tl.y)], (thickness, color));
                            painter
                                .line_segment([tl, pos2(tl.x, tl.y + corner)], (thickness, color));

                            painter
                                .line_segment([tr, pos2(tr.x - corner, tr.y)], (thickness, color));
                            painter
                                .line_segment([tr, pos2(tr.x, tr.y + corner)], (thickness, color));

                            painter
                                .line_segment([bl, pos2(bl.x + corner, bl.y)], (thickness, color));
                            painter
                                .line_segment([bl, pos2(bl.x, bl.y - corner)], (thickness, color));

                            painter
                                .line_segment([br, pos2(br.x - corner, br.y)], (thickness, color));
                            painter
                                .line_segment([br, pos2(br.x, br.y - corner)], (thickness, color));

                            // Name at top center (scale with distance)
                            let name_offset = 14.0 * distance_scale;
                            let top_center = pos2((tl.x + tr.x) * 0.5, tl.y - name_offset);
                            painter.text(
                                top_center,
                                Align2::CENTER_BOTTOM,
                                e.name.as_str(),
                                Self::get_harmony_font((14.0 * distance_scale).clamp(10.0, 22.0)),
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
                    // Debug local axes from entity origin: X=red, Y=green, Z=blue
                    let o = e.origin;
                    let axis = 15.0;
                    if let Some(p0) = world_to_screen(o, view, screen_w, screen_h) {
                        if let Some(px) =
                            world_to_screen(o + Vec3::new(axis, 0.0, 0.0), view, screen_w, screen_h)
                        {
                            painter.line_segment([p0, px], (2.0, Color32::from_rgb(220, 80, 80)));
                        }
                        if let Some(py) =
                            world_to_screen(o + Vec3::new(0.0, axis, 0.0), view, screen_w, screen_h)
                        {
                            painter.line_segment([p0, py], (2.0, Color32::from_rgb(80, 220, 120)));
                        }
                        if let Some(pz) =
                            world_to_screen(o + Vec3::new(0.0, 0.0, axis), view, screen_w, screen_h)
                        {
                            painter.line_segment([p0, pz], (2.0, Color32::from_rgb(80, 120, 220)));
                        }
                    }
                }
            });
        ctx.request_repaint();
    }
}
