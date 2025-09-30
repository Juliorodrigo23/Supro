// src/app.rs
use crate::tracking::{ArmTracker, TrackingResult, GestureType};
use crate::ui::{Theme, UIComponents};
use crate::video::{VideoSource, VideoRecorder};
use crate::data::DataExporter;

use eframe::egui;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use chrono::{DateTime, Local};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Live,
    VideoFile,
    Playback,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    SingleCamera,
    DualView,
    DataAnalysis,
}

pub struct ArmTrackerApp {
    // Core components
    tracker: Arc<Mutex<ArmTracker>>,
    video_source: Option<VideoSource>,
    recorder: Option<VideoRecorder>,
    
    // UI State
    mode: AppMode,
    view_mode: ViewMode,
    theme: Theme,
    show_settings: bool,
    show_about: bool,
    
    // Recording state
    is_recording: bool,
    recording_start: Option<DateTime<Local>>,
    recording_duration: std::time::Duration,
    
    // Tracking data
    current_result: TrackingResult,
    tracking_history: Vec<TrackingResult>,
    
    // Video processing
    selected_video: Option<PathBuf>,
    video_progress: f32,
    is_playing: bool,
    
    // UI Components
    ui_components: UIComponents,
    
    // Settings
    settings: AppSettings,

    current_frame_texture: Option<egui::TextureHandle>,
}

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub enable_left_arm: bool,
    pub enable_right_arm: bool,
    pub enable_fingers: bool,
    pub confidence_threshold: f32,
    pub smoothing_factor: f32,
    pub auto_save: bool,
    pub output_directory: PathBuf,
    pub video_quality: VideoQuality,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoQuality {
    Low,
    Medium,
    High,
    Ultra,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            enable_left_arm: true,
            enable_right_arm: true,
            enable_fingers: true,
            confidence_threshold: 0.6,
            smoothing_factor: 0.7,
            auto_save: true,
            output_directory: directories::UserDirs::new()
                .and_then(|dirs| dirs.document_dir().map(|p| p.join("ArmTracker")))
                .unwrap_or_else(|| PathBuf::from("./output")),
            video_quality: VideoQuality::High,
        }
    }
}

impl ArmTrackerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    let tracker = Arc::new(Mutex::new(
        ArmTracker::new().expect("Failed to initialize tracker")
    ));

    Self {
        tracker,
        video_source: None,                 // << lazy init
        recorder: None,
        mode: AppMode::Live,
        view_mode: ViewMode::DualView,
        theme: Theme::default(),
        show_settings: false,
        show_about: false,
        is_recording: false,
        recording_start: None,
        recording_duration: std::time::Duration::ZERO,
        current_result: TrackingResult::default(),
        tracking_history: Vec::new(),
        selected_video: None,
        video_progress: 0.0,
        is_playing: true,
        ui_components: UIComponents::new(&cc.egui_ctx),
        settings: AppSettings::default(),
        current_frame_texture: None,    
    }
}
    
    fn stop_camera(&mut self) {
        self.video_source = None;
        self.current_frame_texture = None;
        eprintln!("Camera stopped");
    }

   fn start_camera(&mut self) {
    // If already open, just probe a frame
    if let Some(src) = self.video_source.as_mut() {
        if let Err(e) = src.read_frame() {
            eprintln!("Camera already open but failed to read frame: {e}");
        } else {
            eprintln!("Camera already running.");
        }
        return;
    }

    // Try to open camera now (after UI exists, Info.plist present in bundle)
    match VideoSource::new_camera(0) {
        Ok(mut src) => {
            // Probe one frame to ensure the pipeline is live.
            match src.read_frame() {
                Ok(frame) => {
                    eprintln!("Camera started: {}x{}", frame.width(), frame.height());
                    self.video_source = Some(src);
                }
                Err(e) => {
                    eprintln!("Camera opened but failed to read first frame: {e}");
                }
            }
        }
        Err(e) => {
            // Don’t crash the app; show a friendly message in logs/UI
            eprintln!(
                "Failed to open camera: {e}\n\
                 Tip: On macOS you must run the bundled app so TCC can prompt, \
                 and ensure Info.plist includes NSCameraUsageDescription."
            );
        }
    }
}

    fn on_mode_changed(&mut self, old_mode: AppMode, new_mode: AppMode) {
    eprintln!("Mode changed from {:?} to {:?}", old_mode, new_mode);
    
    match new_mode {
        AppMode::Live => {
            // When switching to Live mode, stop any video file playback
            // Camera will be started when user clicks "Start Camera"
            eprintln!("Switched to Live Camera mode");
        }
        AppMode::VideoFile => {
            // Stop camera if running
            if self.video_source.is_some() {
                self.stop_camera();
            }
            eprintln!("Switched to Video File mode");
            // TODO: Open file picker for video file selection
        }
        AppMode::Playback => {
            // Stop camera if running
            if self.video_source.is_some() {
                self.stop_camera();
            }
            eprintln!("Switched to Playback/Analysis mode");
        }
    }
}

    fn render_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(10.0);
            egui::menu::bar(ui, |ui| {
                // Logo and title
                ui.horizontal(|ui| {
                    if let Some(logo) = self.ui_components.logo_texture.as_ref() {
                        ui.image((logo.id(), egui::vec2(40.0, 40.0)));
                    }
                    ui.heading("Arm Rotation Tracking System");
                });
                
                ui.separator();
                
                // Mode selection
                ui.horizontal(|ui| {
                    let old_mode = self.mode;
                    
                    ui.selectable_value(&mut self.mode, AppMode::Live, "🎥 Live Camera");
                    ui.selectable_value(&mut self.mode, AppMode::VideoFile, "📁 Video File");
                    ui.selectable_value(&mut self.mode, AppMode::Playback, "📊 Analysis");
                    
                    // Handle mode changes
                    if self.mode != old_mode {
                        self.on_mode_changed(old_mode, self.mode);
                    }
                });
                
                ui.separator();
                
                // View mode buttons
                ui.horizontal(|ui| {
                    if ui.selectable_label(
                        self.view_mode == ViewMode::SingleCamera,
                        "Single View"
                    ).clicked() {
                        self.view_mode = ViewMode::SingleCamera;
                    }
                    
                    if ui.selectable_label(
                        self.view_mode == ViewMode::DualView,
                        "Dual View"
                    ).clicked() {
                        self.view_mode = ViewMode::DualView;
                    }
                    
                    if ui.selectable_label(
                        self.view_mode == ViewMode::DataAnalysis,
                        "Data Analysis"
                    ).clicked() {
                        self.view_mode = ViewMode::DataAnalysis;
                    }
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Settings button
                    if ui.button("⚙ Settings").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                    
                    // About button
                    if ui.button("ℹ About").clicked() {
                        self.show_about = !self.show_about;
                    }
                });
            });
            ui.add_space(10.0);
        });
    }
    
    fn render_main_content(&mut self, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        match self.mode {
            AppMode::Live => {
                match self.view_mode {
                    ViewMode::SingleCamera => self.render_single_view(ui),
                    ViewMode::DualView => self.render_dual_view(ui),
                    ViewMode::DataAnalysis => self.render_analysis_view(ui),
                }
            }
            AppMode::VideoFile => {
                self.render_video_file_mode(ui);
            }
            AppMode::Playback => {
                self.render_analysis_view(ui);
            }
        }
    });
}

fn render_video_file_mode(&mut self, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(50.0);
        ui.heading("Video File Mode");
        ui.add_space(20.0);
        
        if let Some(path) = &self.selected_video {
            ui.label(format!("Selected: {}", path.display()));
            ui.add_space(10.0);
            
            // Video playback controls would go here
            ui.horizontal(|ui| {
                if ui.button(if self.is_playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                    self.is_playing = !self.is_playing;
                }
                
                ui.add(egui::Slider::new(&mut self.video_progress, 0.0..=100.0)
                    .text("Progress")
                    .suffix("%"));
            });
            
            ui.add_space(20.0);
            
            // Video display
            self.render_video_panel(ui, true);
        } else {
            ui.label("No video file selected");
            ui.add_space(20.0);
            
            if ui.button("📁 Select Video File").clicked() {
                // TODO: Open file picker
                eprintln!("File picker not yet implemented");
                // For now, show a message
                ui.label("File picker coming soon...");
            }
        }
    });
}
    
    fn render_single_view(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |columns| {
            // Left column - Video feed
            columns[0].group(|ui| {
                ui.heading("Camera Feed");
                self.render_video_panel(ui, true);
            });
            
            // Right column - Tracking info
            columns[1].vertical(|ui| {
                // Gesture detection panel
                ui.group(|ui| {
                    ui.heading("Gesture Detection");
                    self.render_gesture_panel(ui);
                });
                
                ui.add_space(20.0);
                
                // Joint tracking panel
                ui.group(|ui| {
                    ui.heading("Joint Tracking");
                    self.render_joint_panel(ui);
                });
            });
        });
    }
    
    fn render_dual_view(&mut self, ui: &mut egui::Ui) {
        // Top section - Video panels
        ui.horizontal(|ui| {
            let available_width = ui.available_width();
            let panel_width = available_width / 2.0 - 10.0;
            
            // Raw feed panel
            ui.allocate_ui(egui::vec2(panel_width, 400.0), |ui| {
                ui.group(|ui| {
                    ui.heading("Raw Feed");
                    self.render_video_panel(ui, false);
                });
            });
            
            ui.add_space(20.0);
            
            // Tracking overlay panel
            ui.allocate_ui(egui::vec2(panel_width, 400.0), |ui| {
                ui.group(|ui| {
                    ui.heading("Tracking Overlay");
                    self.render_video_panel(ui, true);
                });
            });
        });
        
        ui.separator();
        ui.add_space(10.0);
        
        // Bottom section - Rotation info
        ui.horizontal(|ui| {
            let available_width = ui.available_width();
            let panel_width = available_width / 2.0 - 10.0;
            
            // Left arm info
            ui.allocate_ui(egui::vec2(panel_width, 200.0), |ui| {
                self.render_arm_rotation_panel(ui, "left");
            });
            
            ui.add_space(20.0);
            
            // Right arm info
            ui.allocate_ui(egui::vec2(panel_width, 200.0), |ui| {
                self.render_arm_rotation_panel(ui, "right");
            });
        });
    }
    
    fn render_analysis_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Data Analysis");
        
        // Render charts and statistics
        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.heading("Rotation History");
                self.render_rotation_chart(ui);
            });
            
            ui.group(|ui| {
                ui.heading("Statistics");
                self.render_statistics_panel(ui);
            });
        });
        
        ui.separator();
        
        // Data export section
        ui.group(|ui| {
            ui.heading("Export Data");
            if ui.button("Export to CSV").clicked() {
                self.export_data_to_csv();
            }
            if ui.button("Generate Report").clicked() {
                self.generate_report();
            }
        });
    }
    
    fn render_video_panel(&mut self, ui: &mut egui::Ui, with_overlay: bool) {
    let available_size = ui.available_size();
    
    if let Some(texture_id) = self.get_current_frame_texture(with_overlay) {
        // Calculate aspect ratio to maintain proper display
        let aspect_ratio = 16.0 / 9.0; // Adjust based on your camera
        let display_width = available_size.x - 20.0;
        let display_height = display_width / aspect_ratio;
        
        ui.centered_and_justified(|ui| {
            ui.image(egui::load::SizedTexture::new(
                texture_id,
                egui::vec2(display_width, display_height)
            ));
        });
    } else {
        ui.centered_and_justified(|ui| {
            ui.label("No video feed available");
            ui.label("Click 'Start Camera' to begin");
        });
    }
}
    
    fn render_gesture_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Left arm gesture
            ui.vertical(|ui| {
                ui.label("Left Arm:");
                if let Some(gesture) = self.current_result.left_gesture.as_ref() {
                    let color = match gesture.gesture_type {
                        GestureType::Supination => egui::Color32::from_rgb(76, 175, 80),
                        GestureType::Pronation => egui::Color32::from_rgb(255, 152, 0),
                        GestureType::None => egui::Color32::GRAY,
                    };
                    
                    ui.colored_label(color, format!("{:?}", gesture.gesture_type));
                    ui.label(format!("Confidence: {:.1}%", gesture.confidence * 100.0));
                    ui.label(format!("Angle: {:.1}°", gesture.angle.to_degrees()));
                } else {
                    ui.colored_label(egui::Color32::GRAY, "No detection");
                }
            });
            
            ui.separator();
            
            // Right arm gesture
            ui.vertical(|ui| {
                ui.label("Right Arm:");
                if let Some(gesture) = self.current_result.right_gesture.as_ref() {
                    let color = match gesture.gesture_type {
                        GestureType::Supination => egui::Color32::from_rgb(76, 175, 80),
                        GestureType::Pronation => egui::Color32::from_rgb(255, 152, 0),
                        GestureType::None => egui::Color32::GRAY,
                    };
                    
                    ui.colored_label(color, format!("{:?}", gesture.gesture_type));
                    ui.label(format!("Confidence: {:.1}%", gesture.confidence * 100.0));
                    ui.label(format!("Angle: {:.1}°", gesture.angle.to_degrees()));
                } else {
                    ui.colored_label(egui::Color32::GRAY, "No detection");
                }
            });
        });
    }
    
    fn render_arm_rotation_panel(&mut self, ui: &mut egui::Ui, side: &str) {
        let gesture = if side == "left" {
            self.current_result.left_gesture.as_ref()
        } else {
            self.current_result.right_gesture.as_ref()
        };
        
        ui.group(|ui| {
            ui.heading(format!("{} Arm", if side == "left" { "Left" } else { "Right" }));
            
            if let Some(gesture) = gesture {
                // Rotation type with colored background
                let (bg_color, text_color) = match gesture.gesture_type {
                    GestureType::Supination => (
                        egui::Color32::from_rgb(76, 175, 80),
                        egui::Color32::WHITE,
                    ),
                    GestureType::Pronation => (
                        egui::Color32::from_rgb(255, 152, 0),
                        egui::Color32::BLACK,
                    ),
                    GestureType::None => (
                        egui::Color32::from_rgb(100, 100, 100),
                        egui::Color32::WHITE,
                    ),
                };
                
                ui.allocate_ui(egui::vec2(ui.available_width(), 50.0), |ui| {
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(
                        rect,
                        egui::Rounding::same(8.0),
                        bg_color,
                    );
                    
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{:?}", gesture.gesture_type))
                                .size(24.0)
                                .color(text_color)
                        );
                    });
                });
                
                ui.add_space(10.0);
                
                // Confidence bar
                ui.label("Confidence:");
                ui.add(egui::ProgressBar::new(gesture.confidence as f32)
                    .show_percentage()
                    .animate(true));
                
                // Angle indicator
                ui.label(format!("Rotation Angle: {:.1}°", gesture.angle.to_degrees()));
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("No rotation detected")
                            .size(18.0)
                            .color(egui::Color32::GRAY)
                    );
                });
            }
        });
    }
    
    fn render_joint_panel(&mut self, ui: &mut egui::Ui) {
        // Implementation for joint tracking visualization
        ui.label("Joint tracking information...");
    }
    
    fn render_rotation_chart(&mut self, ui: &mut egui::Ui) {
        // Implementation for rotation history chart
        ui.label("Rotation history chart...");
    }
    
    fn render_statistics_panel(&mut self, ui: &mut egui::Ui) {
        // Implementation for statistics display
        ui.label("Statistics...");
    }
    
    fn render_control_panel(&mut self, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("controls").show(ctx, |ui| {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            // Only show camera controls in Live mode
            if self.mode == AppMode::Live {
                if self.video_source.is_some() {
                    if ui.add_sized(
                        [120.0, 40.0],
                        egui::Button::new("⏹ Stop Camera")
                            .fill(egui::Color32::from_rgb(244, 67, 54))
                    ).clicked() {
                        self.stop_camera();
                    }
                } else {
                    if ui.add_sized(
                        [120.0, 40.0],
                        egui::Button::new("📷 Start Camera")
                            .fill(egui::Color32::from_rgb(33, 150, 243))
                    ).clicked() {
                        self.start_camera();
                    }
                }
                
                ui.separator();
            }
            
            // Record button (only in Live or VideoFile mode)
            if self.mode != AppMode::Playback {
                let record_btn = if self.is_recording {
                    ui.add_sized(
                        [120.0, 40.0],
                        egui::Button::new("⏹ Stop Recording")
                            .fill(egui::Color32::from_rgb(244, 67, 54))
                    )
                } else {
                    ui.add_sized(
                        [120.0, 40.0],
                        egui::Button::new("⏺ Record")
                            .fill(egui::Color32::from_rgb(76, 175, 80))
                    )
                };
                
                if record_btn.clicked() {
                    self.toggle_recording();
                }
                
                ui.separator();
            }
            
            // Playback controls for video mode
            if self.mode == AppMode::VideoFile {
                if ui.button(if self.is_playing { "⏸" } else { "▶" }).clicked() {
                    self.is_playing = !self.is_playing;
                }
                
                ui.add(egui::Slider::new(&mut self.video_progress, 0.0..=100.0)
                    .text("Progress")
                    .suffix("%"));
                
                ui.separator();
            }
            
            // Arm toggles (only in Live mode)
            if self.mode == AppMode::Live {
                ui.checkbox(&mut self.settings.enable_left_arm, "Left Arm");
                ui.checkbox(&mut self.settings.enable_right_arm, "Right Arm");
                ui.checkbox(&mut self.settings.enable_fingers, "Fingers");
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.is_recording {
                    let duration = self.recording_duration;
                    let minutes = duration.as_secs() / 60;
                    let seconds = duration.as_secs() % 60;
                    ui.label(
                        egui::RichText::new(format!("Recording: {:02}:{:02}", minutes, seconds))
                            .color(egui::Color32::from_rgb(244, 67, 54))
                    );
                }
            });
        });
        ui.add_space(10.0);
    });
}
    
    fn toggle_recording(&mut self) {
        self.is_recording = !self.is_recording;
        
        if self.is_recording {
            self.recording_start = Some(Local::now());
            // Initialize recorder
            // self.recorder = Some(VideoRecorder::new(...));
        } else {
            self.recording_start = None;
            self.recording_duration = std::time::Duration::ZERO;
            // Save recording
            // self.save_recording();
        }
    }
    
    fn get_current_frame_texture(&self, _with_overlay: bool) -> Option<egui::TextureId> {
        self.current_frame_texture.as_ref().map(|t| t.id())
    }
    
    fn export_data_to_csv(&self) {
        // Implementation for CSV export
    }
    
    fn generate_report(&self) {
        // Implementation for report generation
    }
}

impl eframe::App for ArmTrackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update recording duration if recording
        if self.is_recording {
            if let Some(start) = self.recording_start {
                self.recording_duration = Local::now()
                    .signed_duration_since(start)
                    .to_std()
                    .unwrap_or_default();
            }
        }
        
                if let Some(video_source) = self.video_source.as_mut() {
            match video_source.read_frame() {
                Ok(frame) => {
                    // Convert DynamicImage to egui texture
                    let size = [frame.width() as usize, frame.height() as usize];
                    let rgba = frame.to_rgba8();
                    let pixels = rgba.as_flat_samples();
                    
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        pixels.as_slice(),
                    );
                    
                    // Update or create texture
                    if let Some(texture) = &mut self.current_frame_texture {
                        texture.set(color_image, Default::default());
                    } else {
                        self.current_frame_texture = Some(ctx.load_texture(
                            "video_frame",
                            color_image,
                            Default::default(),
                        ));
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read frame: {}", e);
                }
            }
        }

        // Render UI components
        self.render_header(ctx);
        self.render_control_panel(ctx);
        
        if self.show_settings {
            self.render_settings_window(ctx);
        }
        
        if self.show_about {
            self.render_about_window(ctx);
        }
        
        self.render_main_content(ctx);
        
        ctx.request_repaint();
    }
}

impl ArmTrackerApp {
    fn render_settings_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Settings")
            .open(&mut self.show_settings)
            .resizable(true)
            .default_size([400.0, 500.0])
            .show(ctx, |ui| {
                ui.heading("Tracking Settings");
                
                ui.add_space(10.0);
                
                ui.label("Confidence Threshold:");
                ui.add(egui::Slider::new(&mut self.settings.confidence_threshold, 0.0..=1.0)
                    .step_by(0.01));
                
                ui.label("Smoothing Factor:");
                ui.add(egui::Slider::new(&mut self.settings.smoothing_factor, 0.0..=1.0)
                    .step_by(0.01));
                
                ui.separator();
                
                ui.heading("Video Settings");
                
                ui.label("Quality:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", self.settings.video_quality))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.settings.video_quality, VideoQuality::Low, "Low");
                        ui.selectable_value(&mut self.settings.video_quality, VideoQuality::Medium, "Medium");
                        ui.selectable_value(&mut self.settings.video_quality, VideoQuality::High, "High");
                        ui.selectable_value(&mut self.settings.video_quality, VideoQuality::Ultra, "Ultra");
                    });
                
                ui.separator();
                
                ui.heading("Output Settings");
                
                ui.checkbox(&mut self.settings.auto_save, "Auto-save recordings");
                
                ui.label("Output Directory:");
                ui.label(self.settings.output_directory.display().to_string());
                if ui.button("Browse...").clicked() {
                    // Open file dialog
                }
            });
    }
    
    fn render_about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("About")
            .open(&mut self.show_about)
            .resizable(false)
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Arm Rotation Tracking System");
                    ui.label("Version 1.0.0");
                    ui.add_space(20.0);
                    ui.label("A sophisticated motion tracking application");
                    ui.label("for analyzing arm rotation patterns.");
                    ui.add_space(20.0);
                    ui.hyperlink("https://github.com/Juliorodrigo23/Supro");
                });
            });
    }
}