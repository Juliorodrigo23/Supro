// src/app.rs - Enhanced with video upload, gallery, and streamlined UI
use crate::tracking::{ArmTracker, TrackingResult, GestureType};
use crate::ui::{Theme, UIComponents};
use crate::video::{VideoSource, VideoRecorder, VideoGallery, VideoEntry};
use crate::data::DataExporter;

use eframe::egui;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use chrono::{DateTime, Local};
use rfd::FileDialog;
use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::{draw_line_segment_mut, draw_filled_circle_mut, draw_hollow_circle_mut};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Live,
    VideoFile,
    Gallery,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    SingleCamera,
    DualView,
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
    data_exporter: Option<DataExporter>,
    mediapipe_status: MediaPipeStatus,

    // UI State
    mode: AppMode,
    view_mode: ViewMode,
    theme: Theme,
    show_settings: bool,
    show_about: bool,
    show_save_message: bool,
    save_message_timer: f32,

    // Recording state
    is_recording: bool,
    recording_start: Option<DateTime<Local>>,
    recording_duration: std::time::Duration,

    // Tracking data
    current_result: TrackingResult,
    tracking_history: Vec<TrackingResult>,
    last_valid_result: Option<TrackingResult>,
    show_overlay: bool,

    // Video processing
    selected_video: Option<PathBuf>,
    video_progress: f32,
    is_playing: bool,
    is_processing: bool,
    processing_complete: bool,
    processing_message: String,
    current_video_frame: usize,
    video_playback_speed: f32,
    is_playback_mode: bool,
    video_aspect_ratio: Option<f32>,
    overlay_video_source: Option<VideoSource>,

    // Gallery
    video_gallery: VideoGallery,
    selected_gallery_video: Option<VideoEntry>,

    // UI Components
    ui_components: UIComponents,

    // Settings - Simplified to just directories
    settings: AppSettings,

    current_frame_texture: Option<egui::TextureHandle>,
    overlay_frame_texture: Option<egui::TextureHandle>,

    // Time tracking for frame processing
    sim_time: f64,
    last_frame_time: f64,

    #[cfg(target_os = "macos")]
    pub(crate) macos_icon_set: bool,
}

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub working_directory: PathBuf,  // For processing videos
    pub output_directory: PathBuf,   // For saving recordings
}

impl Default for AppSettings {
    fn default() -> Self {
        let base_dir = directories::UserDirs::new()
            .and_then(|dirs| dirs.document_dir().map(|p| p.join("SuproTracker")))
            .unwrap_or_else(|| PathBuf::from("./SuproTracker"));
        
        Self {
            working_directory: base_dir.join("working"),
            output_directory: base_dir.join("recordings"),
        }
    }
}

impl ArmTrackerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let tracker = Arc::new(Mutex::new(
            ArmTracker::new().expect("Failed to initialize tracker")
        ));
        
        let settings = AppSettings::default();
        
        // Ensure directories exist
        let _ = std::fs::create_dir_all(&settings.working_directory);
        let _ = std::fs::create_dir_all(&settings.output_directory);
        
        let mut gallery = VideoGallery::new(&settings.output_directory);
        let _ = gallery.scan_videos();

        Self {
            tracker,
            video_source: None,
            recorder: None,
            data_exporter: None,
            mediapipe_status: MediaPipeStatus::NotInitialized,
            mode: AppMode::Live,
            view_mode: ViewMode::DualView,
            theme: Theme::default(),
            show_settings: false,
            show_about: false,
            show_save_message: false,
            save_message_timer: 0.0,
            is_recording: false,
            recording_start: None,
            recording_duration: std::time::Duration::ZERO,
            current_result: TrackingResult::default(),
            tracking_history: Vec::new(),
            last_valid_result: None,
            show_overlay: true,
            selected_video: None,
            video_progress: 0.0,
            is_playing: false,
            is_processing: false,
            processing_complete: false,
            processing_message: String::new(),
            current_video_frame: 0,
            video_playback_speed: 1.0,
            is_playback_mode: false,
            video_aspect_ratio: None,
            overlay_video_source: None,
            video_gallery: gallery,
            selected_gallery_video: None,
            ui_components: UIComponents::new(&cc.egui_ctx),
            settings,
            current_frame_texture: None,
            overlay_frame_texture: None,
            sim_time: 0.0,
            last_frame_time: 0.0,
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
            
            let radius = 6.0;
            let rect = ui.allocate_space(egui::vec2(radius * 2.0, radius * 2.0)).1;
            ui.painter().circle_filled(rect.center(), radius, color);
            
            ui.add_space(5.0);
            ui.label(egui::RichText::new(status_text).color(color));
            
            if self.mediapipe_status == MediaPipeStatus::Initializing {
                ui.add(egui::Spinner::new());
            }
        });
    }
    
    fn stop_camera(&mut self) {
        self.video_source = None;
        self.current_frame_texture = None;
        self.current_result = TrackingResult::default();
        self.last_valid_result = None;
        
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

        match VideoSource::new_camera(0) {
            Ok(mut src) => {
                match src.read_frame() {
                    Ok(frame) => {
                        eprintln!("Camera started: {}x{}", frame.width(), frame.height());

                        // Store the camera aspect ratio
                        self.video_aspect_ratio = src.get_aspect_ratio();

                        self.video_source = Some(src);
                        self.mediapipe_status = MediaPipeStatus::Initializing;

                        let tracker = Arc::clone(&self.tracker);
                        std::thread::spawn(move || {
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
    
    fn open_video_file(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Video", &["mp4", "avi", "mov", "mkv"])
            .pick_file() 
        {
            self.selected_video = Some(path.clone());
            self.load_selected_video();
        }
    }
    
    fn load_selected_video(&mut self) {
        if let Some(path) = &self.selected_video {
            match VideoSource::new_file(path) {
                Ok(source) => {
                    // Store the aspect ratio
                    self.video_aspect_ratio = source.get_aspect_ratio();

                    self.video_source = Some(source);
                    self.overlay_video_source = None;
                    self.is_playing = true;
                    self.is_processing = true;
                    self.processing_complete = false;
                    self.is_playback_mode = false;
                    self.processing_message = "Initializing video processing...".to_string();
                    self.video_progress = 0.0;

                    // Initialize MediaPipe for video processing
                    if let Ok(mut tracker) = self.tracker.lock() {
                        tracker.initialize_mediapipe();
                    }

                    // Initialize recorder for saving processed video to gallery folder
                    if let Some(info) = self.video_source.as_ref().and_then(|s| s.get_info()) {
                        match VideoRecorder::new(
                            &self.settings.output_directory, // Save to gallery folder instead of working directory
                            info.width as u32,
                            info.height as u32,
                            info.fps,
                        ) {
                            Ok(recorder) => {
                                let output_dir = recorder.get_output_dir().to_path_buf();
                                self.recorder = Some(recorder);

                                // Initialize data exporter
                                self.data_exporter = Some(DataExporter::new(
                                    output_dir,
                                    Some(format!("processed_{}", Local::now().format("%Y%m%d_%H%M%S")))
                                ));
                            }
                            Err(e) => {
                                eprintln!("Failed to initialize recorder: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to open video file: {}", e);
                    self.processing_message = format!("Error: {}", e);
                    self.is_processing = false;
                    self.processing_complete = false;
                }
            }
        }
    }

    fn load_processed_video_for_playback(&mut self) {
        if let Some(video_entry) = &self.selected_gallery_video {
            // Load raw video
            let raw_path = &video_entry.path;
            match VideoSource::new_file(raw_path) {
                Ok(source) => {
                    self.video_aspect_ratio = source.get_aspect_ratio();
                    self.video_source = Some(source);

                    // Load overlay video if it exists
                    let overlay_path = raw_path.parent()
                        .map(|p| p.join("overlay_video.mp4"));

                    if let Some(overlay_path) = overlay_path {
                        if overlay_path.exists() {
                            match VideoSource::new_file(&overlay_path) {
                                Ok(overlay_source) => {
                                    self.overlay_video_source = Some(overlay_source);
                                }
                                Err(e) => {
                                    eprintln!("Failed to load overlay video: {}", e);
                                }
                            }
                        }
                    }

                    // Set playback mode
                    self.is_playback_mode = true;
                    self.is_playing = false;
                    self.is_processing = false;
                    self.processing_complete = true;
                    self.current_video_frame = 0;
                    self.video_progress = 0.0;
                }
                Err(e) => {
                    eprintln!("Failed to load video for playback: {}", e);
                }
            }
        }
    }

    fn get_video_loading_info(&self) -> (f32, String) {
        if let Some(VideoSource::File(reader)) = &self.video_source {
            (reader.get_loading_progress(), reader.get_loading_message().to_string())
        } else {
            (0.0, String::new())
        }
    }
    
    fn save_processed_video(&mut self) {
        if let Some(recorder) = self.recorder.take() {
            self.processing_message = "Saving videos to gallery...".to_string();

            match recorder.save_videos() {
                Ok((raw_path, overlay_path)) => {
                    // Save CSV data
                    if let Some(exporter) = self.data_exporter.take() {
                        match exporter.export_csv() {
                            Ok(csv_path) => {
                                self.processing_message = format!(
                                    "Saved to gallery:\n- Raw: {}\n- Overlay: {}\n- CSV: {}",
                                    raw_path.display(),
                                    overlay_path.display(),
                                    csv_path.display()
                                );
                                self.show_save_message = true;
                                self.save_message_timer = 5.0;

                                // Refresh gallery to show the newly saved video
                                let _ = self.video_gallery.scan_videos();

                                // Load the processed video for playback
                                if let Some(entry) = self.video_gallery.get_videos().iter()
                                    .find(|v| v.path == raw_path) {
                                    self.selected_gallery_video = Some(entry.clone());
                                    self.selected_video = Some(raw_path.clone());
                                    self.load_processed_video_for_playback();
                                }
                            }
                            Err(e) => {
                                self.processing_message = format!("CSV save error: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    self.processing_message = format!("Video save error: {}", e);
                }
            }

            self.processing_complete = true;
        }
    }
    
    fn toggle_recording(&mut self) {
        self.is_recording = !self.is_recording;
        
        if self.is_recording {
            self.recording_start = Some(Local::now());
            
            // Initialize recorder and data exporter
            if let Some(info) = self.video_source.as_ref().and_then(|s| s.get_info()) {
                match VideoRecorder::new(
                    &self.settings.output_directory,
                    info.width as u32,
                    info.height as u32,
                    info.fps,
                ) {
                    Ok(recorder) => {
                        let output_dir = recorder.get_output_dir().to_path_buf();
                        self.recorder = Some(recorder);
                        
                        // Initialize data exporter
                        self.data_exporter = Some(DataExporter::new(
                            output_dir,
                            Some(format!("session_{}", Local::now().format("%Y%m%d_%H%M%S")))
                        ));
                    }
                    Err(e) => {
                        eprintln!("Failed to start recording: {}", e);
                        self.is_recording = false;
                    }
                }
            }
            
            // Ensure MediaPipe is initialized if recording from camera
            if self.mode == AppMode::Live && self.video_source.is_some() {
                if let Ok(mut tracker) = self.tracker.lock() {
                    tracker.initialize_mediapipe();
                }
            }
        } else {
            // Stop recording and save
            self.recording_start = None;
            self.recording_duration = std::time::Duration::ZERO;
            
            if self.recorder.is_some() {
                self.save_processed_video();
            }
        }
    }
    
    fn on_mode_changed(&mut self, old_mode: AppMode, new_mode: AppMode) {
        match new_mode {
            AppMode::Live => {
                // Clear any video file sources when switching to live
                if old_mode == AppMode::VideoFile || old_mode == AppMode::Gallery {
                    self.video_source = None;
                    self.overlay_video_source = None;
                    self.selected_video = None;
                    self.selected_gallery_video = None;
                    self.is_playback_mode = false;
                    self.processing_complete = false;
                    self.is_processing = false;
                    self.current_frame_texture = None;
                    self.overlay_frame_texture = None;
                }
                eprintln!("Switched to Live Camera mode");
            }
            AppMode::VideoFile => {
                // Stop camera when switching to video file
                if old_mode == AppMode::Live {
                    self.stop_camera();
                }
                eprintln!("Switched to Video File mode");
            }
            AppMode::Gallery => {
                // Clear camera when switching to gallery
                if old_mode == AppMode::Live {
                    self.stop_camera();
                }
                // Clear video processing state
                self.video_source = None;
                self.overlay_video_source = None;
                self.is_playback_mode = false;
                self.processing_complete = false;
                self.is_processing = false;

                // Refresh gallery when entering gallery mode
                let _ = self.video_gallery.scan_videos();
                eprintln!("Switched to Gallery mode");
            }
        }
    }
    
    fn render_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            egui::menu::bar(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(logo) = self.ui_components.logo_texture.as_ref() {
                        ui.image((logo.id(), egui::vec2(64.0, 64.0)));
                    }
                    
                    ui.vertical(|ui| {
                        ui.heading("SuPro");
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("Arm Rotation Tracking System")
                                .italics()
                                .size(14.0)
                                .color(egui::Color32::LIGHT_GRAY),
                        );
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
                    ui.selectable_value(&mut self.mode, AppMode::VideoFile, "📁 Upload Video");
                    ui.selectable_value(&mut self.mode, AppMode::Gallery, "🖼 Gallery");
                    
                    if self.mode != old_mode {
                        self.on_mode_changed(old_mode, self.mode);
                    }
                });
                
                ui.separator();

                // View mode buttons (only for Live mode)
                if self.mode == AppMode::Live {
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.view_mode == ViewMode::SingleCamera, "Single View").clicked() {
                            self.view_mode = ViewMode::SingleCamera;
                        }
                        if ui.selectable_label(self.view_mode == ViewMode::DualView, "Dual View").clicked() {
                            self.view_mode = ViewMode::DualView;
                        }
                    });
                    ui.separator();
                }

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
                        ViewMode::SingleCamera => self.render_single_view_streamlined(ui),
                        ViewMode::DualView => self.render_dual_view_streamlined(ui),
                    }
                }
                AppMode::VideoFile => {
                    self.render_video_file_mode(ui);
                }
                AppMode::Gallery => {
                    self.render_gallery_mode(ui);
                }
            }
            
            // Show save message overlay
            if self.show_save_message {
                egui::Window::new("Save Complete")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(&self.processing_message);
                        ui.add_space(10.0);
                        if ui.button("✖ Close").clicked() {
                            self.show_save_message = false;
                        }
                    });
            }
        });
    }
    
    fn render_single_view_streamlined(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |columns| {
            // Left column - Video feed
            columns[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("Camera Feed");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let toggle_text = if self.show_overlay { "Hide Overlay" } else { "Show Overlay" };
                        if ui.button(toggle_text).clicked() {
                            self.show_overlay = !self.show_overlay;
                        }
                    });
                });
                self.render_video_panel(ui, self.show_overlay);
            });

            // Right column - Gesture info only
            columns[1].vertical(|ui| {
                ui.group(|ui| {
                    ui.heading("Gesture Detection");
                    self.render_gesture_panel(ui);
                });
            });
        });
    }
    
    fn render_dual_view_streamlined(&mut self, ui: &mut egui::Ui) {
        // Top row: two video panels side-by-side
        ui.horizontal(|ui| {
            let avail_w = ui.available_width();
            let panel_w = (avail_w - 20.0) / 2.0;

            let aspect = self.video_aspect_ratio.unwrap_or(16.0 / 9.0);
            let video_display_h = (panel_w / aspect).clamp(180.0, 360.0);

            // Left panel - Raw Feed
            ui.vertical(|ui| {
                ui.set_width(panel_w);
                ui.group(|ui| {
                    ui.heading("Raw Feed");
                    ui.add_space(6.0);
                    self.render_video_panel_sized(ui, panel_w - 20.0, video_display_h, false);
                });
            });

            ui.add_space(20.0);

            // Right panel - Tracking Overlay
            ui.vertical(|ui| {
                ui.set_width(panel_w);
                ui.group(|ui| {
                    ui.heading("Tracking Overlay");
                    ui.add_space(6.0);
                    self.render_video_panel_sized(ui, panel_w - 20.0, video_display_h, true);
                });
            });
        });
        
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
        
        // Bottom: rotation info only
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
    
    fn render_video_file_mode(&mut self, ui: &mut egui::Ui) {
        if self.selected_video.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading("Video Upload & Processing");
                ui.add_space(20.0);

                ui.group(|ui| {
                    ui.add_space(40.0);
                    ui.label("Upload a video to process with MediaPipe tracking");
                    ui.add_space(20.0);

                    let button = egui::Button::new(
                        egui::RichText::new("📁 Select Video File")
                            .size(20.0)
                            .color(egui::Color32::WHITE)
                    )
                    .fill(egui::Color32::from_rgb(33, 150, 243));

                    if ui.add_sized([200.0, 50.0], button).clicked() {
                        self.open_video_file();
                    }

                    ui.add_space(20.0);
                    ui.label("Supported formats: MP4, AVI, MOV, MKV");
                    ui.add_space(40.0);
                });
            });
        } else if self.is_playback_mode {
            // Playback mode - show dual view with controls
            self.render_video_playback_ui(ui);
        } else if self.selected_video.is_some() {
            let video_name = self.selected_video.as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading("Video Upload & Processing");
                ui.add_space(20.0);
                ui.label(format!("Processing: {}", video_name));
                ui.add_space(10.0);

                if self.is_processing && !self.processing_complete {
                    // Get loading info from video reader
                    let (load_progress, load_message) = self.get_video_loading_info();

                    // Show loading progress if available, otherwise show processing progress
                    let display_progress = if load_progress > 0.0 && load_progress < 1.0 {
                        load_progress
                    } else {
                        self.video_progress
                    };

                    // Get total frames for display
                    let total_frames = if let Some(VideoSource::File(reader)) = &self.video_source {
                        reader.get_total_frames()
                    } else {
                        0
                    };

                    let current_frame = (display_progress * total_frames as f32) as usize;

                    ui.group(|ui| {
                        ui.add_space(20.0);

                        // Spinner and status
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().size(24.0));
                            ui.add_space(10.0);
                            if !load_message.is_empty() {
                                ui.label(
                                    egui::RichText::new(&load_message)
                                        .size(16.0)
                                        .color(egui::Color32::from_rgb(100, 150, 255))
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new(&self.processing_message)
                                        .size(16.0)
                                        .color(egui::Color32::from_rgb(100, 150, 255))
                                );
                            }
                        });

                        ui.add_space(15.0);

                        // Progress bar
                        let progress_bar = egui::ProgressBar::new(display_progress)
                            .show_percentage()
                            .animate(true);
                        ui.add_sized([ui.available_width() * 0.8, 25.0], progress_bar);

                        ui.add_space(10.0);

                        // Frame count
                        if total_frames > 0 {
                            ui.label(
                                egui::RichText::new(format!("Processing frame {}/{}", current_frame.min(total_frames), total_frames))
                                    .size(14.0)
                                    .color(egui::Color32::LIGHT_GRAY)
                            );
                        }

                        ui.add_space(20.0);
                    });

                    ui.add_space(20.0);

                    // Display the video being processed
                    match self.view_mode {
                        ViewMode::SingleCamera => {
                            ui.group(|ui| {
                                ui.heading("Processing View");
                                self.render_video_panel(ui, true);
                            });
                        }
                        ViewMode::DualView => {
                            ui.horizontal(|ui| {
                                let avail_w = ui.available_width();
                                let panel_w = (avail_w - 20.0) / 2.0;

                                ui.vertical(|ui| {
                                    ui.set_width(panel_w);
                                    ui.group(|ui| {
                                        ui.heading("Raw");
                                        self.render_video_panel(ui, false);
                                    });
                                });

                                ui.vertical(|ui| {
                                    ui.set_width(panel_w);
                                    ui.group(|ui| {
                                        ui.heading("With Tracking");
                                        self.render_video_panel(ui, true);
                                    });
                                });
                            });
                        }
                    }
                } else if self.processing_complete {
                    // After processing is complete, switch to playback mode
                    self.is_playback_mode = true;
                    self.render_video_playback_ui(ui);
                }

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if !self.processing_complete {
                        if ui.button("Cancel").clicked() {
                            self.selected_video = None;
                            self.is_processing = false;
                            self.video_source = None;
                        }
                    }
                });
            });
        }
    }

    fn render_video_playback_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);

        // Top row: two video panels side-by-side
        ui.horizontal(|ui| {
            let avail_w = ui.available_width();
            let panel_w = (avail_w - 20.0) / 2.0;

            let aspect = self.video_aspect_ratio.unwrap_or(16.0 / 9.0);
            let video_display_h = (panel_w / aspect).clamp(200.0, 500.0);

            // Left panel - Raw Feed
            ui.vertical(|ui| {
                ui.set_width(panel_w);
                ui.group(|ui| {
                    ui.heading("Raw Video");
                    ui.add_space(6.0);
                    self.render_video_panel_sized(ui, panel_w - 20.0, video_display_h, false);
                });
            });

            ui.add_space(20.0);

            // Right panel - Tracking Overlay
            ui.vertical(|ui| {
                ui.set_width(panel_w);
                ui.group(|ui| {
                    ui.heading("With Tracking Overlay");
                    ui.add_space(6.0);
                    self.render_video_panel_with_overlay_sized(ui, panel_w - 20.0, video_display_h);
                });
            });
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Bottom: playback controls
        self.render_video_playback_controls(ui);

        ui.add_space(20.0);

        // Navigation buttons
        ui.horizontal(|ui| {
            if ui.button("⬅ Back to Gallery").clicked() {
                self.mode = AppMode::Gallery;
                self.selected_video = None;
                self.selected_gallery_video = None;
                self.processing_complete = false;
                self.is_processing = false;
                self.is_playback_mode = false;
                self.video_source = None;
                self.overlay_video_source = None;
            }

            if !self.is_playback_mode {
                if ui.button("📁 Process Another Video").clicked() {
                    self.selected_video = None;
                    self.processing_complete = false;
                    self.is_processing = false;
                    self.is_playback_mode = false;
                    self.video_source = None;
                    self.overlay_video_source = None;
                }
            }
        });
    }
    
    fn render_gallery_mode(&mut self, ui: &mut egui::Ui) {
    ui.heading("Video Gallery");
    ui.add_space(10.0);
    
    if ui.button("🔄 Refresh").clicked() {
        let _ = self.video_gallery.scan_videos();
    }
    
    ui.separator();
    ui.add_space(10.0);
    
    // Clone the videos to avoid borrow issue
    let videos = self.video_gallery.get_videos().to_vec();
    
    if videos.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label("No recorded videos yet");
            ui.add_space(20.0);
            ui.label("Videos will appear here after recording");
        });
    } else {
        // Display videos in a grid
        egui::ScrollArea::vertical().show(ui, |ui| {
            let columns = 4;
            let mut current_row = Vec::new();
            
            for (i, video) in videos.iter().enumerate() {
                current_row.push(video.clone());
                
                if current_row.len() == columns || i == videos.len() - 1 {
                    ui.horizontal(|ui| {
                        for video_entry in &current_row {
                            self.render_video_thumbnail(ui, video_entry);
                            ui.add_space(10.0);
                        }
                    });
                    ui.add_space(10.0);
                    current_row.clear();
                }
            }
        });
    }
}
    
    fn render_video_thumbnail(&mut self, ui: &mut egui::Ui, video: &VideoEntry) {
        ui.vertical(|ui| {
            ui.set_width(200.0);

            ui.group(|ui| {
                // Display thumbnail
                let (rect, response) = ui.allocate_exact_size(egui::vec2(200.0, 150.0), egui::Sense::click());

                if let Some(thumbnail) = &video.thumbnail {
                    // Convert thumbnail to texture if needed
                    let size = [thumbnail.width() as usize, thumbnail.height() as usize];
                    let rgba = thumbnail.to_rgba8();
                    let pixels = rgba.as_flat_samples();

                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        pixels.as_slice(),
                    );

                    let texture = ui.ctx().load_texture(
                        format!("thumb_{}", video.name),
                        color_image,
                        Default::default(),
                    );

                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    // Placeholder
                    ui.painter().rect_filled(rect, egui::Rounding::same(4.0), egui::Color32::from_rgb(50, 50, 55));
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "📹",
                        egui::FontId::proportional(40.0),
                        egui::Color32::WHITE,
                    );
                }

                if response.clicked() {
                    self.selected_gallery_video = Some(video.clone());
                    self.selected_video = Some(video.path.clone());
                    // Switch to VideoFile mode to view the processed video
                    self.mode = AppMode::VideoFile;
                    self.load_processed_video_for_playback();
                }
                
                ui.label(&video.name);
                ui.label(
                    egui::RichText::new(video.date.format("%Y-%m-%d %H:%M").to_string())
                        .size(11.0)
                        .color(egui::Color32::GRAY)
                );
                
                ui.horizontal(|ui| {
                    if video.has_overlay {
                        ui.colored_label(egui::Color32::GREEN, "✓ Overlay");
                    }
                    if video.has_csv {
                        ui.colored_label(egui::Color32::GREEN, "✓ CSV");
                    }
                });
            });
        });
    }
    
    fn render_video_panel(&mut self, ui: &mut egui::Ui, with_overlay: bool) {
        let max_w = ui.available_width();
        let aspect = self.video_aspect_ratio.unwrap_or(16.0 / 9.0);
        let display_w = (max_w - 20.0).max(240.0);
        let display_h = (display_w / aspect).clamp(160.0, 420.0);

        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(display_w, display_h), egui::Sense::hover());

        ui.painter().rect_filled(rect, egui::Rounding::same(8.0), egui::Color32::from_rgb(28, 28, 34));

        if let Some(texture_id) = self.get_current_frame_texture() {
            ui.painter().image(
                texture_id,
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );

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
    
    fn render_video_panel_sized(&mut self, ui: &mut egui::Ui, width: f32, height: f32, with_overlay: bool) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

        ui.painter().rect_filled(rect, egui::Rounding::same(8.0), egui::Color32::from_rgb(28, 28, 34));

        if let Some(texture_id) = self.get_current_frame_texture() {
            ui.painter().image(
                texture_id,
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );

            if with_overlay && !self.current_result.tracking_lost {
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
    }

    fn render_video_panel_with_overlay_sized(&mut self, ui: &mut egui::Ui, width: f32, height: f32) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

        ui.painter().rect_filled(rect, egui::Rounding::same(8.0), egui::Color32::from_rgb(28, 28, 34));

        // Try to use overlay video source if available
        if let Some(texture) = &self.overlay_frame_texture {
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else if let Some(texture_id) = self.get_current_frame_texture() {
            // Fallback to regular frame with overlay drawn on top
            ui.painter().image(
                texture_id,
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );

            if !self.current_result.tracking_lost {
                self.draw_tracking_overlay(ui, rect);
            }
        } else {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No overlay available",
                egui::FontId::proportional(16.0),
                egui::Color32::from_gray(180),
            );
        }
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
                ui.label(
                    egui::RichText::new(format!("{} Arm", if side == "left" { "Left" } else { "Right" }))
                        .size(18.0)
                        .strong()
                );
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(gesture) = gesture {
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
                        
                        ui.painter().rect_filled(
                            rect,
                            egui::Rounding::same(4.0),
                            egui::Color32::from_gray(60)
                        );
                        
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
                
                // Record controls (only in Live mode)
                if self.mode == AppMode::Live {
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

    fn draw_overlay_on_image(&self, image: &DynamicImage, tracking_result: &TrackingResult) -> DynamicImage {
        let mut img = image.to_rgba8();
        let width = img.width() as f32;
        let height = img.height() as f32;

        // Draw skeleton connections with thicker lines for higher quality
        let connections = vec![
            ("left_shoulder", "left_elbow"),
            ("left_elbow", "left_wrist"),
            ("right_shoulder", "right_elbow"),
            ("right_elbow", "right_wrist"),
            ("left_shoulder", "right_shoulder"),
        ];

        for (from, to) in connections {
            if let (Some(from_joint), Some(to_joint)) = (
                tracking_result.joints.get(from),
                tracking_result.joints.get(to),
            ) {
                let from_x = (from_joint.position.x as f32 * width) as i32;
                let from_y = (from_joint.position.y as f32 * height) as i32;
                let to_x = (to_joint.position.x as f32 * width) as i32;
                let to_y = (to_joint.position.y as f32 * height) as i32;

                // Draw thick line (5 pixels wide for better visibility)
                let green = Rgba([0u8, 255u8, 0u8, 255u8]);
                for offset in -2..=2 {
                    draw_line_segment_mut(
                        &mut img,
                        (from_x as f32 + offset as f32, from_y as f32),
                        (to_x as f32 + offset as f32, to_y as f32),
                        green,
                    );
                    draw_line_segment_mut(
                        &mut img,
                        (from_x as f32, from_y as f32 + offset as f32),
                        (to_x as f32, to_y as f32 + offset as f32),
                        green,
                    );
                }
            }
        }

        // Draw joints with larger circles for better visibility
        for (name, joint) in &tracking_result.joints {
            let x = (joint.position.x as f32 * width) as i32;
            let y = (joint.position.y as f32 * height) as i32;

            let color = if name.contains("left") {
                Rgba([255u8, 0u8, 0u8, 255u8])
            } else {
                Rgba([0u8, 0u8, 255u8, 255u8])
            };

            draw_filled_circle_mut(&mut img, (x, y), 12, color);
            draw_hollow_circle_mut(&mut img, (x, y), 14, Rgba([255u8, 255u8, 255u8, 255u8]));
        }

        // Draw hand landmarks and connections
        for (side, hand) in &tracking_result.hands {
            if !hand.is_tracked {
                continue;
            }

            let hand_color = if side == "left" {
                Rgba([255u8, 100u8, 100u8, 255u8])
            } else {
                Rgba([100u8, 100u8, 255u8, 255u8])
            };

            // Draw hand landmarks
            for (i, landmark) in hand.landmarks.iter().enumerate() {
                let x = (landmark.x as f32 * width) as i32;
                let y = (landmark.y as f32 * height) as i32;

                let radius = if i == 0 { 8 } else { 5 };
                draw_filled_circle_mut(&mut img, (x, y), radius, hand_color);
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
                    let from_x = (hand.landmarks[*from].x as f32 * width) as i32;
                    let from_y = (hand.landmarks[*from].y as f32 * height) as i32;
                    let to_x = (hand.landmarks[*to].x as f32 * width) as i32;
                    let to_y = (hand.landmarks[*to].y as f32 * height) as i32;

                    // Draw thicker lines for fingers
                    for offset in -1..=1 {
                        draw_line_segment_mut(
                            &mut img,
                            (from_x as f32 + offset as f32, from_y as f32),
                            (to_x as f32 + offset as f32, to_y as f32),
                            hand_color,
                        );
                    }
                }
            }
        }

        DynamicImage::ImageRgba8(img)
    }
    
    fn render_settings_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Settings")
            .open(&mut self.show_settings)
            .resizable(true)
            .default_size([500.0, 300.0])
            .show(ctx, |ui| {
                ui.heading("Directory Settings");
                ui.add_space(10.0);
                
                ui.group(|ui| {
                    ui.label("Working Directory (for processing videos):");
                    ui.horizontal(|ui| {
                        ui.label(self.settings.working_directory.display().to_string());
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                self.settings.working_directory = path;
                                let _ = std::fs::create_dir_all(&self.settings.working_directory);
                            }
                        }
                    });
                });
                
                ui.add_space(10.0);
                
                ui.group(|ui| {
                    ui.label("Output Directory (for saving recordings):");
                    ui.horizontal(|ui| {
                        ui.label(self.settings.output_directory.display().to_string());
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                self.settings.output_directory = path.clone();
                                let _ = std::fs::create_dir_all(&self.settings.output_directory);
                                // Update gallery to scan from new directory
                                self.video_gallery = VideoGallery::new(&self.settings.output_directory);
                                let _ = self.video_gallery.scan_videos();
                            }
                        }
                    });
                });

                ui.add_space(10.0);

                if ui.button("Save Settings").clicked() {
                    // Settings are already applied immediately, just show confirmation
                    ui.label("Settings saved!");
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
                    ui.heading("SuPro");
                    ui.label("Version 1.0.0");
                    ui.add_space(12.0);
                    ui.label("A sophisticated motion tracking application");
                    ui.label("for analyzing arm rotation patterns.");
                    ui.add_space(16.0);
                    ui.hyperlink("https://github.com/Juliorodrigo23/Supro");
                });
            });
    }

    fn render_video_playback_controls(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Video Playback");
            ui.add_space(10.0);

            // Get total frames if available
            let total_frames = if let Some(VideoSource::File(reader)) = &self.video_source {
                reader.get_total_frames()
            } else {
                0
            };

            if total_frames > 0 {
                // Frame scrubber
                ui.horizontal(|ui| {
                    ui.label("Frame:");

                    let mut frame_f32 = self.current_video_frame as f32;
                    let slider = egui::Slider::new(&mut frame_f32, 0.0..=(total_frames - 1) as f32)
                        .show_value(false);

                    if ui.add(slider).changed() {
                        self.current_video_frame = frame_f32 as usize;
                        self.is_playing = false; // Pause when scrubbing
                    }

                    ui.label(format!("{} / {}", self.current_video_frame + 1, total_frames));
                });

                ui.add_space(10.0);

                // Playback controls
                ui.horizontal(|ui| {
                    // Previous frame
                    if ui.button("⏮ Prev").clicked() && self.current_video_frame > 0 {
                        self.current_video_frame -= 1;
                        self.is_playing = false;
                    }

                    // Play/Pause
                    let play_pause_text = if self.is_playing { "⏸ Pause" } else { "▶ Play" };
                    if ui.button(play_pause_text).clicked() {
                        self.is_playing = !self.is_playing;
                    }

                    // Next frame
                    if ui.button("Next ⏭").clicked() && self.current_video_frame < total_frames - 1 {
                        self.current_video_frame += 1;
                        self.is_playing = false;
                    }

                    ui.separator();

                    // Speed control
                    ui.label("Speed:");
                    ui.add(egui::Slider::new(&mut self.video_playback_speed, 0.25..=2.0)
                        .text("x")
                        .suffix("x"));
                });

                ui.add_space(5.0);

                // Quick seek buttons
                ui.horizontal(|ui| {
                    ui.label("Quick Seek:");

                    if ui.button("-10 frames").clicked() {
                        self.current_video_frame = self.current_video_frame.saturating_sub(10);
                        self.is_playing = false;
                    }

                    if ui.button("+10 frames").clicked() {
                        self.current_video_frame = (self.current_video_frame + 10).min(total_frames - 1);
                        self.is_playing = false;
                    }

                    if ui.button("Reset to Start").clicked() {
                        self.current_video_frame = 0;
                        self.is_playing = false;
                    }
                });
            } else {
                ui.label("No video loaded");
            }
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
        
        // Handle save message timer
        if self.show_save_message {
            self.save_message_timer -= ctx.input(|i| i.unstable_dt);
            if self.save_message_timer <= 0.0 {
                self.show_save_message = false;
            }
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
        
        // Process video frames
        if self.is_playback_mode {
            // Playback mode - read from both raw and overlay sources
            if self.is_playing {
                // Get video info for FPS
                let fps = if let Some(info) = self.video_source.as_ref().and_then(|s| s.get_info()) {
                    info.fps as f64
                } else {
                    30.0 // Default FPS
                };

                // Calculate time per frame based on speed
                let frame_interval = 1.0 / (fps * self.video_playback_speed as f64);

                // Check if enough time has passed to advance frame
                let time_since_last_frame = self.sim_time - self.last_frame_time;

                if time_since_last_frame >= frame_interval {
                    // Get total frames to check bounds
                    let total_frames = if let Some(VideoSource::File(reader)) = &self.video_source {
                        reader.get_total_frames()
                    } else {
                        0
                    };

                    // Check if we've reached the end
                    if self.current_video_frame >= total_frames.saturating_sub(1) {
                        self.is_playing = false;
                        self.current_video_frame = total_frames.saturating_sub(1);
                    } else {
                        self.current_video_frame += 1;
                        self.last_frame_time = self.sim_time;
                    }
                }

                // Load and display current frame (always update texture even if frame didn't advance)
                if let Some(VideoSource::File(reader)) = &mut self.video_source {
                    if let Some(frame) = reader.get_frame(self.current_video_frame) {
                        let size = [frame.width() as usize, frame.height() as usize];
                        let rgba = frame.to_rgba8();
                        let pixels = rgba.as_flat_samples();

                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                            size,
                            pixels.as_slice(),
                        );

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
                }

                // Load overlay frame
                if let Some(VideoSource::File(reader)) = &mut self.overlay_video_source {
                    if let Some(overlay_frame) = reader.get_frame(self.current_video_frame) {
                        let size = [overlay_frame.width() as usize, overlay_frame.height() as usize];
                        let rgba = overlay_frame.to_rgba8();
                        let pixels = rgba.as_flat_samples();

                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                            size,
                            pixels.as_slice(),
                        );

                        if let Some(texture) = &mut self.overlay_frame_texture {
                            texture.set(color_image, Default::default());
                        } else {
                            self.overlay_frame_texture = Some(ctx.load_texture(
                                "overlay_frame",
                                color_image,
                                Default::default(),
                            ));
                        }
                    }
                }
            } else {
                // When paused or scrubbing, load the current frame
                if let Some(VideoSource::File(reader)) = &mut self.video_source {
                    if let Some(frame) = reader.get_frame(self.current_video_frame) {
                        let size = [frame.width() as usize, frame.height() as usize];
                        let rgba = frame.to_rgba8();
                        let pixels = rgba.as_flat_samples();

                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                            size,
                            pixels.as_slice(),
                        );

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
                }

                if let Some(VideoSource::File(reader)) = &mut self.overlay_video_source {
                    if let Some(overlay_frame) = reader.get_frame(self.current_video_frame) {
                        let size = [overlay_frame.width() as usize, overlay_frame.height() as usize];
                        let rgba = overlay_frame.to_rgba8();
                        let pixels = rgba.as_flat_samples();

                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                            size,
                            pixels.as_slice(),
                        );

                        if let Some(texture) = &mut self.overlay_frame_texture {
                            texture.set(color_image, Default::default());
                        } else {
                            self.overlay_frame_texture = Some(ctx.load_texture(
                                "overlay_frame",
                                color_image,
                                Default::default(),
                            ));
                        }
                    }
                }
            }
        } else if let Some(video_source) = self.video_source.as_mut() {
            // Normal processing mode
            match video_source.read_frame() {
                Ok(frame) => {
                    // Create overlay frame with tracking
                    let overlay_frame = frame.clone();

                    // Process with tracker
                    if let Ok(mut tracker) = self.tracker.lock() {
                        match tracker.process_frame(&frame) {
                            Ok(tracking_result) => {
                                self.current_result = tracking_result.clone();

                                if tracking_result.left_gesture.is_some() || tracking_result.right_gesture.is_some() {
                                    self.last_valid_result = Some(tracking_result.clone());
                                }

                                self.tracking_history.push(tracking_result.clone());

                                if self.tracking_history.len() > 1000 {
                                    self.tracking_history.remove(0);
                                }

                                // Update progress for video files
                                if self.mode == AppMode::VideoFile {
                                    self.video_progress = video_source.get_progress();
                                }

                                // Add to data exporter and recorder
                                if self.is_recording || (self.mode == AppMode::VideoFile && self.is_processing) {
                                    if let Some(exporter) = &mut self.data_exporter {
                                        exporter.add_frame(tracking_result.clone(), self.sim_time);
                                    }

                                    // Draw overlay directly onto the frame for video file processing
                                    let final_overlay_frame = if self.mode == AppMode::VideoFile {
                                        self.draw_overlay_on_image(&frame, &tracking_result)
                                    } else {
                                        overlay_frame.clone()
                                    };

                                    if let Some(recorder) = &mut self.recorder {
                                        recorder.add_frame(&frame, Some(&final_overlay_frame));
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Tracking error: {}", e);
                            }
                        }
                    }

                    // Update texture
                    let size = [frame.width() as usize, frame.height() as usize];
                    let rgba = frame.to_rgba8();
                    let pixels = rgba.as_flat_samples();

                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        pixels.as_slice(),
                    );

                    if let Some(texture) = &mut self.current_frame_texture {
                        texture.set(color_image, Default::default());
                    } else {
                        self.current_frame_texture = Some(ctx.load_texture(
                            "video_frame",
                            color_image,
                            Default::default(),
                        ));
                    }

                    // Check if video processing is complete
                    if self.mode == AppMode::VideoFile && self.video_progress >= 0.99 {
                        self.processing_complete = true;
                        self.is_processing = false;
                        if self.recorder.is_some() {
                            self.save_processed_video();
                        }
                    }
                }
                Err(_) => {
                    // End of video or error
                    if self.mode == AppMode::VideoFile && !self.is_playback_mode {
                        self.processing_complete = true;
                        self.is_processing = false;
                        if self.recorder.is_some() {
                            self.save_processed_video();
                        }
                    }
                }
            }
        }
        
        // Update time
        self.sim_time += ctx.input(|i| i.unstable_dt) as f64;
        
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