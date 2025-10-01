// src/app.rs - Corrected version
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MediaPipeStatus {
    NotInitialized,
    Initializing,
    Ready,
    Failed,
    SimulationMode,
}

pub struct ArmTrackerApp {
    // Core components
    tracker: Arc<Mutex<ArmTracker>>,
    video_source: Option<VideoSource>,
    recorder: Option<VideoRecorder>,
    mediapipe_status: MediaPipeStatus,
    
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
    last_valid_result: Option<TrackingResult>,  // ADD THIS - Store last valid tracking
    show_overlay: bool,


    // Video processing
    selected_video: Option<PathBuf>,
    video_progress: f32,
    is_playing: bool,
    
    // UI Components
    ui_components: UIComponents,
    
    // Settings
    settings: AppSettings,

    current_frame_texture: Option<egui::TextureHandle>,

    #[cfg(target_os = "macos")]
    pub(crate) macos_icon_set: bool,
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

fn render_video_panel_with_overlay(&mut self, ui: &mut egui::Ui, with_overlay: bool) {
    let max_w = ui.available_width();
    let aspect = 16.0 / 9.0;
    let display_w = (max_w - 20.0).max(240.0);
    let display_h = (display_w / aspect).clamp(160.0, 420.0);

    let (rect, _resp) = ui.allocate_exact_size(egui::vec2(display_w, display_h), egui::Sense::hover());

    ui.painter().rect_filled(rect, egui::Rounding::same(8.0), egui::Color32::from_rgb(28, 28, 34));

    if let Some(texture_id) = self.get_current_frame_texture() {  // Remove parameter
        ui.painter().image(
            texture_id,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        // Draw overlay if requested and tracking is active
        if with_overlay && !self.current_result.tracking_lost {
            self.draw_tracking_overlay(ui, rect);
        }
    } else {
        ui.painter().rect_stroke(rect, egui::Rounding::same(8.0), egui::Stroke::new(1.0, egui::Color32::from_gray(100)));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No video feed",
            egui::FontId::proportional(16.0),
            egui::Color32::from_gray(180),
        );
    }
}

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let tracker = Arc::new(Mutex::new(
            ArmTracker::new().expect("Failed to initialize tracker")
        ));

        Self {
            tracker,
            video_source: None,
            recorder: None,
            mediapipe_status: MediaPipeStatus::NotInitialized,
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
            last_valid_result: None,  // ADD THIS
            show_overlay: true,
            selected_video: None,
            video_progress: 0.0,
            is_playing: true,
            ui_components: UIComponents::new(&cc.egui_ctx),
            settings: AppSettings::default(),
            current_frame_texture: None,    
            #[cfg(target_os = "macos")]
            macos_icon_set: false,
        }
    }
    
    fn update_mediapipe_status(&mut self) {
        if let Ok(tracker) = self.tracker.lock() {
            if tracker.is_using_mediapipe() {
                self.mediapipe_status = MediaPipeStatus::Ready;
            } else if tracker.is_initializing() {
                self.mediapipe_status = MediaPipeStatus::Initializing;
            } else if self.video_source.is_none() {
                self.mediapipe_status = MediaPipeStatus::NotInitialized;
            } else {
                self.mediapipe_status = MediaPipeStatus::SimulationMode;
            }
        }
    }
    
    fn render_tracking_status(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let (status_text, color) = match self.mediapipe_status {
                MediaPipeStatus::NotInitialized => ("Not Initialized", egui::Color32::GRAY),
                MediaPipeStatus::Initializing => ("Initializing...", egui::Color32::YELLOW),
                MediaPipeStatus::Ready => ("MediaPipe Ready", egui::Color32::GREEN),
                MediaPipeStatus::Failed => ("Failed (Simulation Mode)", egui::Color32::from_rgb(255, 100, 0)),
                MediaPipeStatus::SimulationMode => ("Simulation Mode", egui::Color32::from_rgb(100, 150, 255)),
            };
            
            // Draw status indicator dot
            let radius = 6.0;
            let rect = ui.allocate_space(egui::vec2(radius * 2.0, radius * 2.0)).1;
            ui.painter().circle_filled(rect.center(), radius, color);
            
            ui.add_space(5.0);
            ui.label(egui::RichText::new(status_text).color(color));
            
            // Add spinner animation if initializing
            if self.mediapipe_status == MediaPipeStatus::Initializing {
                ui.add(egui::Spinner::new());
            }
        });
    }

    fn stop_camera(&mut self) {
        // Stop video source
        self.video_source = None;
        self.current_frame_texture = None;
        
        // Shutdown MediaPipe when camera stops
        if let Ok(mut tracker) = self.tracker.lock() {
            tracker.shutdown_mediapipe();
        }
        
        self.mediapipe_status = MediaPipeStatus::NotInitialized;
        eprintln!("Camera and MediaPipe stopped");
    }

   fn start_camera(&mut self) {
    if let Some(src) = self.video_source.as_mut() {
        if let Err(e) = src.read_frame() {
            eprintln!("Camera already open but failed to read frame: {e}");
        } else {
            eprintln!("Camera already running.");
        }
        return;
    }

    // Try to open camera
    match VideoSource::new_camera(0) {
        Ok(mut src) => {
            match src.read_frame() {
                Ok(frame) => {
                    eprintln!("Camera started: {}x{}", frame.width(), frame.height());
                    self.video_source = Some(src);
                    self.mediapipe_status = MediaPipeStatus::Initializing;
                    
                    // DELAYED MediaPipe initialization - spawn in background
                    let tracker = Arc::clone(&self.tracker);
                    std::thread::spawn(move || {
                        // Wait 500ms for camera to stabilize
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        
                        eprintln!("Starting MediaPipe initialization...");
                        if let Ok(mut t) = tracker.lock() {
                            t.initialize_mediapipe();
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Camera opened but failed to read first frame: {e}");
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open camera: {e}");
        }
    }
}
    
    fn on_mode_changed(&mut self, old_mode: AppMode, new_mode: AppMode) {
        eprintln!("Mode changed from {:?} to {:?}", old_mode, new_mode);
        
        match new_mode {
            AppMode::Live => {
                // When switching to Live mode, camera will be started when user clicks "Start Camera"
                eprintln!("Switched to Live Camera mode");
            }
            AppMode::VideoFile => {
                // Stop camera and MediaPipe if running
                if self.video_source.is_some() {
                    self.stop_camera();
                }
                eprintln!("Switched to Video File mode");
            }
            AppMode::Playback => {
                // Stop camera and MediaPipe if running
                if self.video_source.is_some() {
                    self.stop_camera();
                }
                eprintln!("Switched to Playback/Analysis mode");
            }
        }
    }
    
    fn toggle_recording(&mut self) {
        self.is_recording = !self.is_recording;
        
        if self.is_recording {
            self.recording_start = Some(Local::now());
            
            // Only ensure MediaPipe is initialized if we're recording from camera
            if self.mode == AppMode::Live && self.video_source.is_some() {
                if let Ok(mut tracker) = self.tracker.lock() {
                    // This will be a no-op if already initialized
                    tracker.initialize_mediapipe();
                }
            }
        } else {
            self.recording_start = None;
            self.recording_duration = std::time::Duration::ZERO;
        }
    }

   fn render_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            egui::menu::bar(ui, |ui| {
                // Left area: Logo + Titles + Byline
                ui.horizontal(|ui| {
                    if let Some(logo) = self.ui_components.logo_texture.as_ref() {
                        // Bigger logo (was 40x40 → now 64x64)
                        ui.image((logo.id(), egui::vec2(64.0, 64.0)));
                    }

                    ui.vertical(|ui| {
                        // Main title now uses “Supro”
                        ui.heading("Supro Arm Tracker");

                        // Subtitle / section title row (optional small tagline)
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("Arm Rotation Tracking System")
                                .italics()
                                .size(14.0)
                                .color(egui::Color32::LIGHT_GRAY),
                        );

                        // Byline
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("By Julio Contreras — Under Dr. Ortiz's Research Lab")
                                .size(13.0)
                                .color(egui::Color32::WHITE),
                        );
                    });
                });

                ui.separator();

                // Mode selection
                ui.horizontal(|ui| {
                    let old_mode = self.mode;

                    ui.selectable_value(&mut self.mode, AppMode::Live, "🎥 Live Camera");
                    ui.selectable_value(&mut self.mode, AppMode::VideoFile, "📁 Video File");
                    ui.selectable_value(&mut self.mode, AppMode::Playback, "📊 Analysis");

                    if self.mode != old_mode {
                        self.on_mode_changed(old_mode, self.mode);
                    }
                });

                ui.separator();

                // View mode buttons
                ui.horizontal(|ui| {
                    if ui.selectable_label(self.view_mode == ViewMode::SingleCamera, "Single View").clicked() {
                        self.view_mode = ViewMode::SingleCamera;
                    }

                    if ui.selectable_label(self.view_mode == ViewMode::DualView, "Dual View").clicked() {
                        self.view_mode = ViewMode::DualView;
                    }

                    if ui.selectable_label(self.view_mode == ViewMode::DataAnalysis, "Data Analysis").clicked() {
                        self.view_mode = ViewMode::DataAnalysis;
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Settings").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                    if ui.button("ℹ About").clicked() {
                        self.show_about = !self.show_about;
                    }
                });
            });
            ui.add_space(6.0);
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
                }
            }
        });
    }
    
    fn render_single_view(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |columns| {
            // Left column - Video feed
            columns[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("Camera Feed");
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Add overlay toggle button
                        let toggle_text = if self.show_overlay { "Hide Overlay" } else { "Show Overlay" };
                        if ui.button(toggle_text).clicked() {
                            self.show_overlay = !self.show_overlay;
                        }
                    });
                });
                
                self.render_video_panel(ui, self.show_overlay);  // Use show_overlay state
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
    // Top row: two video panels side-by-side with EQUAL sizes
    ui.horizontal(|ui| {
        let avail_w = ui.available_width();
        let panel_w = (avail_w - 20.0) / 2.0;
        
        // Fixed aspect ratio for consistent sizing
        let aspect = 16.0 / 9.0;
        let video_display_h = (panel_w / aspect).clamp(180.0, 360.0);

        // Left panel - Raw Feed
        ui.vertical(|ui| {
            ui.set_width(panel_w);
            ui.group(|ui| {
                ui.heading("Raw Feed");
                ui.add_space(6.0);
                
                // Allocate exact size for video
                let (rect, _resp) = ui.allocate_exact_size(
                    egui::vec2(panel_w - 20.0, video_display_h), 
                    egui::Sense::hover()
                );
                
                ui.painter().rect_filled(
                    rect, 
                    egui::Rounding::same(8.0), 
                    egui::Color32::from_rgb(28, 28, 34)
                );
                
                if let Some(texture_id) = self.get_current_frame_texture() {
                    ui.painter().image(
                        texture_id,
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "No video feed",
                        egui::FontId::proportional(16.0),
                        egui::Color32::from_gray(180),
                    );
                }
            });
        });

        ui.add_space(20.0);

        // Right panel - Tracking Overlay
        ui.vertical(|ui| {
            ui.set_width(panel_w);
            ui.group(|ui| {
                ui.heading("Tracking Overlay");
                ui.add_space(6.0);
                
                // Allocate exact same size for video
                let (rect, _resp) = ui.allocate_exact_size(
                    egui::vec2(panel_w - 20.0, video_display_h), 
                    egui::Sense::hover()
                );
                
                ui.painter().rect_filled(
                    rect, 
                    egui::Rounding::same(8.0), 
                    egui::Color32::from_rgb(28, 28, 34)
                );
                
                if let Some(texture_id) = self.get_current_frame_texture() {
                    ui.painter().image(
                        texture_id,
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    
                    // Draw tracking overlay
                    if !self.current_result.tracking_lost {
                        self.draw_tracking_overlay(ui, rect);
                    }
                } else {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "No video feed",
                        egui::FontId::proportional(16.0),
                        egui::Color32::from_gray(180),
                    );
                }
            });
        });
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Bottom: merged rotation box with DYNAMIC heights
    ui.group(|ui| {
        ui.heading("Arm Rotation");
        ui.add_space(6.0);
        ui.vertical(|ui| {
            self.render_arm_rotation_panel_dynamic(ui, "left");
            ui.add_space(8.0);
            self.render_arm_rotation_panel_dynamic(ui, "right");
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
    // Delegate to the unified version above that properly reserves layout space
    self.render_video_panel_with_overlay(ui, with_overlay);
}
    
    fn render_gesture_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Left arm gesture
            ui.vertical(|ui| {
                ui.label("Left Arm:");
                let gesture = self.current_result.left_gesture.as_ref()
                    .or(self.last_valid_result.as_ref().and_then(|r| r.left_gesture.as_ref()));

                if let Some(gesture) = gesture {
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

                let gesture = self.current_result.right_gesture.as_ref()
                    .or(self.last_valid_result.as_ref().and_then(|r| r.right_gesture.as_ref()));

                if let Some(gesture) = gesture {
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
    
fn render_arm_rotation_panel_dynamic(&mut self, ui: &mut egui::Ui, side: &str) {
    let gesture = if side == "left" {
        self.current_result.left_gesture.as_ref()
            .or(self.last_valid_result.as_ref().and_then(|r| r.left_gesture.as_ref()))
    } else {
        self.current_result.right_gesture.as_ref()
            .or(self.last_valid_result.as_ref().and_then(|r| r.right_gesture.as_ref()))
    };

    ui.group(|ui| {
        ui.horizontal(|ui| {
            // Title on the left
            ui.label(
                egui::RichText::new(format!("{} Arm", if side == "left" { "Left" } else { "Right" }))
                    .size(18.0)
                    .strong()
            );
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(gesture) = gesture {
                    // Display gesture type with colored badge
                    let (bg_color, text_color, text) = match gesture.gesture_type {
                        GestureType::Supination => (
                            egui::Color32::from_rgb(76, 175, 80), 
                            egui::Color32::WHITE,
                            "Supination"
                        ),
                        GestureType::Pronation => (
                            egui::Color32::from_rgb(255, 152, 0), 
                            egui::Color32::BLACK,
                            "Pronation"
                        ),
                        GestureType::None => (
                            egui::Color32::from_rgb(100, 100, 100), 
                            egui::Color32::WHITE,
                            "None"
                        ),
                    };
                    
                    let badge = egui::Button::new(
                        egui::RichText::new(text)
                            .color(text_color)
                            .size(16.0)
                            .strong()
                    )
                    .fill(bg_color)
                    .sense(egui::Sense::hover());
                    
                    ui.add(badge);
                }
            });
        });
        
        ui.add_space(8.0);
        
        if let Some(gesture) = gesture {
            // Show detailed information when gesture is detected
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Confidence: {:.1}%", gesture.confidence * 100.0))
                            .size(15.0)
                    );
                    ui.label(
                        egui::RichText::new(format!("Rotation Angle: {:.1}°", gesture.angle.to_degrees()))
                            .size(15.0)
                    );
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Visual confidence meter
                    let confidence_color = if gesture.confidence > 0.8 {
                        egui::Color32::from_rgb(76, 175, 80)
                    } else if gesture.confidence > 0.5 {
                        egui::Color32::from_rgb(255, 193, 7)
                    } else {
                        egui::Color32::from_rgb(244, 67, 54)
                    };
                    
                    let bar_width = 100.0;
                    let bar_height = 20.0;
                    let (rect, _response) = ui.allocate_exact_size(
                        egui::vec2(bar_width, bar_height),
                        egui::Sense::hover()
                    );
                    
                    // Background
                    ui.painter().rect_filled(
                        rect,
                        egui::Rounding::same(4.0),
                        egui::Color32::from_gray(60)
                    );
                    
                    // Confidence bar
                    let filled_width = bar_width * (gesture.confidence as f32);
                    let filled_rect = egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(filled_width, bar_height)
                    );
                    ui.painter().rect_filled(
                        filled_rect,
                        egui::Rounding::same(4.0),
                        confidence_color
                    );
                });
            });
        } else {
            // Compact display when no gesture detected
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("No rotation detected")
                        .size(15.0)
                        .color(egui::Color32::GRAY)
                );
            });
        }
        
        ui.add_space(4.0);
    });
}

fn draw_tracking_overlay(&self, ui: &mut egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    
    // Draw skeleton connections
    let connections = vec![
        ("left_shoulder", "left_elbow"),
        ("left_elbow", "left_wrist"),
        ("right_shoulder", "right_elbow"),
        ("right_elbow", "right_wrist"),
        ("left_shoulder", "right_shoulder"),
    ];
    
    for (from, to) in connections {
        if let (Some(from_joint), Some(to_joint)) = (
            self.current_result.joints.get(from),
            self.current_result.joints.get(to),
        ) {
            let from_pos = egui::pos2(
                rect.left() + from_joint.position.x as f32 * rect.width(),
                rect.top() + from_joint.position.y as f32 * rect.height(),
            );
            let to_pos = egui::pos2(
                rect.left() + to_joint.position.x as f32 * rect.width(),
                rect.top() + to_joint.position.y as f32 * rect.height(),
            );
            
            painter.line_segment(
                [from_pos, to_pos],
                egui::Stroke::new(3.0, egui::Color32::from_rgb(0, 255, 0)),
            );
        }
    }
    
    // Draw joints
    for (name, joint) in &self.current_result.joints {
        let pos = egui::pos2(
            rect.left() + joint.position.x as f32 * rect.width(),
            rect.top() + joint.position.y as f32 * rect.height(),
        );
        
        let color = if name.contains("left") {
            egui::Color32::from_rgb(255, 0, 0)
        } else {
            egui::Color32::from_rgb(0, 0, 255)
        };
        
        painter.circle_filled(pos, 8.0, color);
        painter.circle_stroke(pos, 10.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
    }

    // Draw hand landmarks and connections
    for (side, hand) in &self.current_result.hands {
        if !hand.is_tracked {
            continue;
        }
        
        let hand_color = if side == "left" {
            egui::Color32::from_rgb(255, 100, 100)
        } else {
            egui::Color32::from_rgb(100, 100, 255)
        };
        
        // Draw hand landmarks
        for (i, landmark) in hand.landmarks.iter().enumerate() {
            let pos = egui::pos2(
                rect.left() + landmark.x as f32 * rect.width(),
                rect.top() + landmark.y as f32 * rect.height(),
            );
            
            // Larger circle for wrist, smaller for other landmarks
            let radius = if i == 0 { 6.0 } else { 3.0 };
            painter.circle_filled(pos, radius, hand_color);
        }
        
        // Draw connections between finger joints
        let finger_connections = [
            // Thumb
            (0, 1), (1, 2), (2, 3), (3, 4),
            // Index
            (0, 5), (5, 6), (6, 7), (7, 8),
            // Middle
            (0, 9), (9, 10), (10, 11), (11, 12),
            // Ring
            (0, 13), (13, 14), (14, 15), (15, 16),
            // Pinky
            (0, 17), (17, 18), (18, 19), (19, 20),
        ];
        
        for (from, to) in finger_connections.iter() {
            if *from < hand.landmarks.len() && *to < hand.landmarks.len() {
                let from_pos = egui::pos2(
                    rect.left() + hand.landmarks[*from].x as f32 * rect.width(),
                    rect.top() + hand.landmarks[*from].y as f32 * rect.height(),
                );
                let to_pos = egui::pos2(
                    rect.left() + hand.landmarks[*to].x as f32 * rect.width(),
                    rect.top() + hand.landmarks[*to].y as f32 * rect.height(),
                );
                
                painter.line_segment(
                    [from_pos, to_pos],
                    egui::Stroke::new(2.0, hand_color.linear_multiply(0.7)),
                );
            }
        }
    }
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
            // Camera controls (Live mode only)
            if self.mode == AppMode::Live {
                if self.video_source.is_some() {
                    let stop_cam = egui::Button::new(
                        egui::RichText::new("⏹ Stop Camera").color(egui::Color32::WHITE)
                    ).fill(egui::Color32::from_rgb(244, 67, 54));
                    if ui.add_sized([140.0, 40.0], stop_cam).clicked() {
                        self.stop_camera();
                    }

                    ui.separator();
                    self.render_tracking_status(ui);
                } else {
                    let start_cam = egui::Button::new(
                        egui::RichText::new("📷 Start Camera").color(egui::Color32::WHITE)
                    ).fill(egui::Color32::from_rgb(33, 150, 243));
                    if ui.add_sized([140.0, 40.0], start_cam).clicked() {
                        self.start_camera();
                    }
                }
                ui.separator();
            }

            // Record controls (hidden in Playback mode)
            if self.mode != AppMode::Playback {
                if self.is_recording {
                    let stop_rec = egui::Button::new(
                        egui::RichText::new("⏹ Stop Recording").color(egui::Color32::WHITE)
                    ).fill(egui::Color32::from_rgb(244, 67, 54));
                    if ui.add_sized([160.0, 40.0], stop_rec).clicked() {
                        self.toggle_recording();
                    }
                } else {
                    let start_rec = egui::Button::new(
                        egui::RichText::new("⏺ Record").color(egui::Color32::WHITE)
                    ).fill(egui::Color32::from_rgb(76, 175, 80));
                    if ui.add_sized([140.0, 40.0], start_rec).clicked() {
                        self.toggle_recording();
                    }
                }
                ui.separator();
            }

            // Video file playback controls
            if self.mode == AppMode::VideoFile {
                if ui.button(if self.is_playing { "⏸" } else { "▶" }).clicked() {
                    self.is_playing = !self.is_playing;
                }
                ui.add(
                    egui::Slider::new(&mut self.video_progress, 0.0..=100.0)
                        .text("Progress").suffix("%")
                );
                ui.separator();
            }

            // Arm toggles (Live mode only)
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
                            .color(egui::Color32::from_rgb(244, 67, 54)),
                    );
                }
            });
        });
        ui.add_space(10.0);
    });
}

    
    fn get_current_frame_texture(&self) -> Option<egui::TextureId> {
        self.current_frame_texture.as_ref().map(|t| t.id())
    }
    
    fn export_data_to_csv(&self) {
        // Implementation for CSV export
        eprintln!("Exporting data to CSV...");
    }
    
    fn generate_report(&self) {
        // Implementation for report generation
        eprintln!("Generating report...");
    }
    
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
                    eprintln!("File browser not yet implemented");
                }
            });
    }
    
   fn render_about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("About")
            .open(&mut self.show_about)
            .resizable(false)
            .default_size([420.0, 320.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Supro Arm Tracker");
                    ui.label("Version 1.0.0");
                    ui.add_space(12.0);
                    ui.label("A sophisticated motion tracking application");
                    ui.label("for analyzing arm rotation patterns.");
                    ui.add_space(16.0);
                    ui.hyperlink("https://github.com/Juliorodrigo23/Supro");
                });
            });
    }
}

impl eframe::App for ArmTrackerApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "macos")]
        if !self.macos_icon_set {
            crate::set_macos_dock_icon_from_bundle();
            self.macos_icon_set = true;
        }
        
        // Update MediaPipe status
        self.update_mediapipe_status();
        
        // Update recording duration if recording
        if self.is_recording {
            if let Some(start) = self.recording_start {
                self.recording_duration = Local::now()
                    .signed_duration_since(start)
                    .to_std()
                    .unwrap_or_default();
            }
        }
        
        // Process video frame
        if let Some(video_source) = self.video_source.as_mut() {
            match video_source.read_frame() {
                Ok(frame) => {
                    // Process EVERY frame - no skipping
                    if let Ok(mut tracker) = self.tracker.lock() {
                        match tracker.process_frame(&frame) {
                            Ok(tracking_result) => {
                                self.current_result = tracking_result.clone();

                                if tracking_result.left_gesture.is_some() || tracking_result.right_gesture.is_some() {
                                    self.last_valid_result = Some(tracking_result.clone());
                                }

                                self.tracking_history.push(tracking_result);
                                
                                if self.tracking_history.len() > 1000 {
                                    self.tracking_history.remove(0);
                                }
                            }
                            Err(e) => {
                                eprintln!("Tracking error: {}", e);
                            }
                        }
                    }
            

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