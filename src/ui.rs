// src/ui.rs - Fixed to use resvg's re-exported tiny_skia
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use image::DynamicImage;
use usvg::TreeParsing;

#[derive(Debug, Clone)]
pub struct Theme {
    pub primary: Color32,
    pub secondary: Color32,
    pub background: Color32,
    pub surface: Color32,
    pub error: Color32,
    pub warning: Color32,
    pub success: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Color32::from_rgb(70, 130, 240),
            secondary: Color32::from_rgb(255, 152, 0),
            background: Color32::from_rgb(20, 20, 25),
            surface: Color32::from_rgb(30, 30, 35),
            error: Color32::from_rgb(244, 67, 54),
            warning: Color32::from_rgb(255, 152, 0),
            success: Color32::from_rgb(76, 175, 80),
            text_primary: Color32::WHITE,
            text_secondary: Color32::from_rgb(200, 200, 200),
        }
    }
}

pub struct UIComponents {
    pub logo_texture: Option<egui::TextureHandle>,
    pub theme: Theme,
    animations: AnimationState,
}

#[derive(Default)]
struct AnimationState {
    record_pulse: f32,
    gesture_transitions: std::collections::HashMap<String, f32>,
}

impl UIComponents {
    pub fn new(ctx: &egui::Context) -> Self {
        let mut components = Self {
            logo_texture: None,
            theme: Theme::default(),
            animations: AnimationState::default(),
        };
        
        // Try to load SVG logo
        let logo_path = "/Users/JulioContreras/Desktop/School/Research/Baseball SuPro /SuPro Rewritten/assets/supro.svg";
        if let Ok(logo_rgba) = load_svg_as_rgba(logo_path, 256) {
            let size = [256, 256];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                size,
                &logo_rgba,
            );
            
            components.logo_texture = Some(ctx.load_texture(
                "logo",
                color_image,
                Default::default(),
            ));
        }
        
        components
    }
    
    pub fn draw_gesture_indicator(
        &mut self,
        ui: &mut egui::Ui,
        gesture_type: &str,
        confidence: f32,
        angle: f32,
    ) {
        let available_size = ui.available_size();
        let center = Pos2::new(available_size.x / 2.0, available_size.y / 2.0);
        let radius = available_size.x.min(available_size.y) * 0.4;
        
        // Background circle
        let painter = ui.painter();
        painter.circle_filled(center, radius, self.theme.surface);
        
        // Confidence arc
        let color = match gesture_type {
            "supination" => self.theme.success,
            "pronation" => self.theme.warning,
            _ => self.theme.text_secondary,
        };
        
        let arc_angle = confidence * std::f32::consts::PI * 2.0;
        draw_arc(painter, center, radius * 0.9, 0.0, arc_angle, color, 5.0);
        
        // Center text
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            gesture_type.to_uppercase(),
            egui::FontId::proportional(24.0),
            self.theme.text_primary,
        );
        
        // Angle indicator
        let angle_text = format!("{:.1}°", angle.to_degrees());
        painter.text(
            Pos2::new(center.x, center.y + radius * 0.5),
            egui::Align2::CENTER_CENTER,
            angle_text,
            egui::FontId::proportional(16.0),
            self.theme.text_secondary,
        );
    }
    
    pub fn draw_joint_skeleton(
        &mut self,
        ui: &mut egui::Ui,
        joints: &[(String, (f32, f32))],
    ) {
        let painter = ui.painter();
        let rect = ui.available_rect_before_wrap();
        
        // Define skeleton connections
        let connections = vec![
            ("left_shoulder", "left_elbow"),
            ("left_elbow", "left_wrist"),
            ("right_shoulder", "right_elbow"),
            ("right_elbow", "right_wrist"),
            ("left_shoulder", "right_shoulder"),
        ];
        
        // Draw connections
        for (from, to) in connections {
            if let (Some(from_joint), Some(to_joint)) = (
                joints.iter().find(|(name, _)| name == from),
                joints.iter().find(|(name, _)| name == to),
            ) {
                let from_pos = Pos2::new(
                    rect.left() + from_joint.1.0 * rect.width(),
                    rect.top() + from_joint.1.1 * rect.height(),
                );
                let to_pos = Pos2::new(
                    rect.left() + to_joint.1.0 * rect.width(),
                    rect.top() + to_joint.1.1 * rect.height(),
                );
                
                painter.line_segment(
                    [from_pos, to_pos],
                    Stroke::new(2.0, self.theme.primary),
                );
            }
        }
        
        // Draw joints
        for (name, (x, y)) in joints {
            let pos = Pos2::new(
                rect.left() + x * rect.width(),
                rect.top() + y * rect.height(),
            );
            
            let color = if name.contains("left") {
                self.theme.primary
            } else {
                self.theme.secondary
            };
            
            painter.circle_filled(pos, 5.0, color);
            painter.circle_stroke(pos, 7.0, Stroke::new(2.0, self.theme.text_primary));
        }
    }
    
    pub fn draw_recording_indicator(&mut self, ui: &mut egui::Ui, is_recording: bool) {
        if !is_recording {
            return;
        }
        
        // Animate pulse effect
        self.animations.record_pulse += ui.input(|i| i.unstable_dt) * 2.0;
        let pulse = (self.animations.record_pulse.sin() + 1.0) * 0.5;
        
        let size = 20.0 + pulse * 5.0;
        let color = Color32::from_rgb(
            244,
            (67.0 + pulse * 30.0) as u8,
            54,
        );
        
        let painter = ui.painter();
        let pos = Pos2::new(ui.available_width() - 30.0, 30.0);
        
        painter.circle_filled(pos, size, color);
        painter.text(
            Pos2::new(pos.x - 50.0, pos.y),
            egui::Align2::RIGHT_CENTER,
            "REC",
            egui::FontId::proportional(14.0),
            color,
        );
    }
    
    pub fn draw_confidence_bar(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        value: f32,
    ) {
        ui.horizontal(|ui| {
            ui.label(label);
            
            let bar_width = 200.0;
            let bar_height = 20.0;
            let rect = ui.allocate_space(Vec2::new(bar_width, bar_height)).1;
            
            let painter = ui.painter();
            
            // Background
            painter.rect_filled(
                rect,
                egui::Rounding::same(4.0),
                self.theme.surface,
            );
            
            // Fill
            let fill_width = bar_width * value;
            let fill_rect = Rect::from_min_size(
                rect.min,
                Vec2::new(fill_width, bar_height),
            );
            
            let color = if value > 0.7 {
                self.theme.success
            } else if value > 0.4 {
                self.theme.warning
            } else {
                self.theme.error
            };
            
            painter.rect_filled(
                fill_rect,
                egui::Rounding::same(4.0),
                color,
            );
            
            // Text
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{:.0}%", value * 100.0),
                egui::FontId::proportional(12.0),
                self.theme.text_primary,
            );
        });
    }
}

fn draw_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    color: Color32,
    thickness: f32,
) {
    let points_count = ((end_angle - start_angle).abs() * 50.0) as usize;
    let mut points = Vec::with_capacity(points_count);
    
    for i in 0..=points_count {
        let t = i as f32 / points_count as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        let x = center.x + radius * angle.cos();
        let y = center.y + radius * angle.sin();
        points.push(Pos2::new(x, y));
    }
    
    for i in 1..points.len() {
        painter.line_segment(
            [points[i - 1], points[i]],
            Stroke::new(thickness, color),
        );
    }
}

fn load_svg_as_rgba(path: &str, size: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let svg_data = std::fs::read_to_string(path)?;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg_data, &opt)?;
    
    // Use resvg's re-exported tiny_skia types
    let pixmap_size = tree.size.to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).unwrap();
    
    let scale = size as f32 / pixmap_size.width().max(pixmap_size.height()) as f32;
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    
    // Use the Tree's render method directly with consistent types
    resvg::Tree::from_usvg(&tree).render(transform, &mut pixmap.as_mut());
    
    Ok(pixmap.data().to_vec())
}

fn load_logo_image() -> Result<DynamicImage, image::ImageError> {
    // This is now a fallback
    Ok(DynamicImage::new_rgba8(128, 128))
}

// Custom widget for video display
pub struct VideoWidget {
    texture_id: Option<egui::TextureId>,
    aspect_ratio: f32,
}

impl VideoWidget {
    pub fn new() -> Self {
        Self {
            texture_id: None,
            aspect_ratio: 16.0 / 9.0,
        }
    }
    
    pub fn update_frame(&mut self, ctx: &egui::Context, frame: &DynamicImage) {
        // Convert image to egui texture
        let size = [frame.width() as _, frame.height() as _];
        let rgba = frame.to_rgba8();
        let pixels = rgba.as_flat_samples();
        
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            size,
            pixels.as_slice(),
        );
        
        self.texture_id = Some(ctx.load_texture(
            "video_frame",
            color_image,
            Default::default(),
        ).id());
    }
    
    pub fn show(&self, ui: &mut egui::Ui) {
        let available_size = ui.available_size();
        let widget_width = available_size.x;
        let widget_height = widget_width / self.aspect_ratio;
        
        let size = Vec2::new(widget_width, widget_height);
        let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
        
        if let Some(texture_id) = self.texture_id {
            ui.painter().image(
                texture_id,
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            ui.painter().rect_filled(
                rect,
                egui::Rounding::same(4.0),
                Color32::from_rgb(50, 50, 55),
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No Video Signal",
                egui::FontId::proportional(16.0),
                Color32::from_rgb(150, 150, 155),
            );
        }
    }
}