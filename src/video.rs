// src/video.rs - Enhanced with video file processing and overlay capabilities
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use image::{DynamicImage, ImageBuffer};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;
use std::sync::{Arc, Mutex};
use std::process::Command;
use std::fs;

pub enum VideoSource {
    Camera(Arc<Mutex<Camera>>),
    File(VideoFileReader),
}

pub struct VideoFileReader {
    path: PathBuf,
    current_frame: usize,
    total_frames: usize,
    width: u32,
    height: u32,
    fps: f32,
    frames_cache: Vec<DynamicImage>,
    is_loaded: bool,
    loading_progress: f32,
    loading_message: String,
}

impl VideoFileReader {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Check if file exists
        if !path.exists() {
            return Err(anyhow::anyhow!("Video file does not exist: {}", path.display()));
        }

        // Check if we have read permissions
        if let Err(e) = std::fs::File::open(&path) {
            return Err(anyhow::anyhow!("Cannot read video file (permission denied): {}", e));
        }

        // Check if ffprobe is available
        if Command::new("ffprobe").arg("-version").output().is_err() {
            return Err(anyhow::anyhow!("FFmpeg is not installed or not in PATH. Please install FFmpeg to process videos."));
        }

        // Get video info using ffprobe
        let output = Command::new("ffprobe")
            .args(&[
                "-v", "error",
                "-select_streams", "v:0",
                "-count_frames",
                "-show_entries", "stream=width,height,r_frame_rate,nb_frames",
                "-of", "csv=p=0",
                path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to run ffprobe")?;
        
        let info = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = info.trim().split(',').collect();

        if parts.len() < 4 {
            return Err(anyhow::anyhow!("Invalid video format or corrupted file"));
        }

        let width = parts[0].parse()
            .map_err(|_| anyhow::anyhow!("Invalid video width"))?;
        let height = parts[1].parse()
            .map_err(|_| anyhow::anyhow!("Invalid video height"))?;
        let fps_str = parts[2];
        let fps = if fps_str.contains('/') {
            let fps_parts: Vec<&str> = fps_str.split('/').collect();
            if fps_parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid frame rate format"));
            }
            fps_parts[0].parse::<f32>().unwrap_or(30.0) / fps_parts[1].parse::<f32>().unwrap_or(1.0)
        } else {
            fps_str.parse().unwrap_or(30.0)
        };
        let total_frames: usize = parts[3].parse()
            .map_err(|_| anyhow::anyhow!("Invalid frame count"))?;

        if total_frames == 0 {
            return Err(anyhow::anyhow!("Video has no frames"));
        }

        Ok(Self {
            path,
            current_frame: 0,
            total_frames,
            width,
            height,
            fps,
            frames_cache: Vec::new(),
            is_loaded: false,
            loading_progress: 0.0,
            loading_message: String::from("Initializing..."),
        })
    }

    pub fn get_loading_progress(&self) -> f32 {
        self.loading_progress
    }

    pub fn get_loading_message(&self) -> &str {
        &self.loading_message
    }

    pub fn get_total_frames(&self) -> usize {
        self.total_frames
    }
    
    pub fn load_all_frames(&mut self) -> Result<()> {
        if self.is_loaded {
            return Ok(());
        }

        eprintln!("Loading video frames from: {}", self.path.display());
        self.loading_message = "Extracting frames...".to_string();
        self.loading_progress = 0.0;

        // Check if ffmpeg is available
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            return Err(anyhow::anyhow!("FFmpeg is not installed. Please install FFmpeg to process videos."));
        }

        // Check available disk space
        let temp_dir = std::env::temp_dir().join(format!("supro_{}", uuid::Uuid::new_v4()));

        // Estimate required space (rough estimate: frames * 0.5MB per frame)
        let estimated_space_mb = (self.total_frames as f64 * 0.5) as u64;
        eprintln!("Estimated disk space needed: {} MB", estimated_space_mb);

        if let Err(e) = fs::create_dir_all(&temp_dir) {
            return Err(anyhow::anyhow!("Cannot create temporary directory: {}", e));
        }

        self.loading_progress = 0.1;
        self.loading_message = format!("Extracting {} frames...", self.total_frames);

        // Extract frames as images
        let status = Command::new("ffmpeg")
            .args(&[
                "-i", self.path.to_str().unwrap(),
                "-vf", "scale=640:480",
                &format!("{}/frame_%04d.png", temp_dir.display()),
            ])
            .status()
            .context("Failed to extract frames with ffmpeg")?;

        if !status.success() {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(anyhow::anyhow!("FFmpeg frame extraction failed. The video format may be unsupported."));
        }

        self.loading_progress = 0.5;
        self.loading_message = "Loading frames into memory...".to_string();

        // Load extracted frames
        self.frames_cache.clear();
        for i in 1..=self.total_frames {
            let frame_path = temp_dir.join(format!("frame_{:04}.png", i));
            if frame_path.exists() {
                match image::open(&frame_path) {
                    Ok(img) => {
                        self.frames_cache.push(img);
                        self.loading_progress = 0.5 + (0.5 * (i as f32 / self.total_frames as f32));
                        self.loading_message = format!("Loading frame {}/{}", i, self.total_frames);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load frame {}: {}", i, e);
                    }
                }
            }
        }

        // Clean up temp files
        let _ = fs::remove_dir_all(&temp_dir);

        if self.frames_cache.is_empty() {
            return Err(anyhow::anyhow!("No frames could be loaded from the video"));
        }

        self.is_loaded = true;
        self.loading_progress = 1.0;
        self.loading_message = format!("Loaded {} frames successfully", self.frames_cache.len());
        eprintln!("Loaded {} frames", self.frames_cache.len());
        Ok(())
    }
    
    pub fn get_frame(&mut self, index: usize) -> Option<DynamicImage> {
        if !self.is_loaded {
            let _ = self.load_all_frames();
        }
        self.frames_cache.get(index).cloned()
    }
    
    pub fn next_frame(&mut self) -> Option<DynamicImage> {
        let frame = self.get_frame(self.current_frame);
        if frame.is_some() {
            self.current_frame = (self.current_frame + 1) % self.total_frames;
        }
        frame
    }
    
    pub fn seek(&mut self, frame_index: usize) {
        self.current_frame = frame_index.min(self.total_frames - 1);
    }
    
    pub fn get_progress(&self) -> f32 {
        if self.total_frames == 0 {
            0.0
        } else {
            self.current_frame as f32 / self.total_frames as f32
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub path: PathBuf,
    pub fps: f64,
    pub frame_count: i32,
    pub width: i32,
    pub height: i32,
    pub current_frame: i32,
}

impl VideoSource {
    pub fn new_camera(index: i32) -> Result<Self> {
        eprintln!("DEBUG: Attempting to open camera index {}", index);
        
        let camera_index = CameraIndex::Index(index as u32);
        
        use nokhwa::utils::{CameraFormat, FrameFormat, Resolution};
        
        let format = CameraFormat::new(
            Resolution::new(640, 480),
            FrameFormat::MJPEG,
            30,
        );
        
        let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(format));
        
        eprintln!("DEBUG: Creating camera object...");
        let camera = Camera::new(camera_index, requested)
            .map_err(|e| {
                eprintln!("DEBUG: Failed to create camera: {}", e);
                anyhow::anyhow!("Failed to open camera: {}", e)
            })?;
        
        eprintln!("DEBUG: Camera created successfully");
        Ok(VideoSource::Camera(Arc::new(Mutex::new(camera))))
    }
    
    pub fn new_file(path: impl AsRef<Path>) -> Result<Self> {
        let reader = VideoFileReader::new(path)?;
        Ok(VideoSource::File(reader))
    }
    
    pub fn read_frame(&mut self) -> Result<DynamicImage> {
        match self {
            VideoSource::Camera(camera) => {
                let mut cam = camera.lock().unwrap();
                
                if !cam.is_stream_open() {
                    cam.open_stream()
                        .map_err(|e| anyhow::anyhow!("Failed to open camera stream: {}", e))?;
                }
                
                let frame = cam.frame()
                    .map_err(|e| anyhow::anyhow!("Failed to capture frame: {}", e))?;
                
                let decoded = frame.decode_image::<RgbFormat>()
                    .map_err(|e| anyhow::anyhow!("Failed to decode frame: {}", e))?;
                
                let width = decoded.width();
                let height = decoded.height();
                let rgb_data = decoded.into_vec();
                
                let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
                for chunk in rgb_data.chunks(3) {
                    rgba_data.push(chunk[0]);
                    rgba_data.push(chunk[1]);
                    rgba_data.push(chunk[2]);
                    rgba_data.push(255);
                }
                
                let img = ImageBuffer::from_raw(width, height, rgba_data)
                    .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;
                
                let flipped = image::imageops::flip_horizontal(&img);
                Ok(DynamicImage::ImageRgba8(flipped))
            }
            VideoSource::File(reader) => {
                reader.next_frame()
                    .ok_or_else(|| anyhow::anyhow!("No more frames in video"))
            }
        }
    }
    
    pub fn get_info(&self) -> Option<VideoInfo> {
        match self {
            VideoSource::Camera(camera) => {
                let cam = camera.lock().unwrap();
                let resolution = cam.resolution();
                Some(VideoInfo {
                    path: PathBuf::from("camera://0"),
                    fps: cam.frame_rate() as f64,
                    frame_count: -1,
                    width: resolution.width() as i32,
                    height: resolution.height() as i32,
                    current_frame: 0,
                })
            }
            VideoSource::File(reader) => Some(VideoInfo {
                path: reader.path.clone(),
                fps: reader.fps as f64,
                frame_count: reader.total_frames as i32,
                width: reader.width as i32,
                height: reader.height as i32,
                current_frame: reader.current_frame as i32,
            }),
        }
    }
    
    pub fn seek(&mut self, frame_number: i32) -> Result<()> {
        if let VideoSource::File(reader) = self {
            reader.seek(frame_number as usize);
        }
        Ok(())
    }
    
    pub fn get_progress(&self) -> f32 {
        match self {
            VideoSource::Camera(_) => 0.0,
            VideoSource::File(reader) => reader.get_progress(),
        }
    }
}

impl Drop for VideoSource {
    fn drop(&mut self) {
        if let VideoSource::Camera(camera) = self {
            if let Ok(mut cam) = camera.lock() {
                let _ = cam.stop_stream();
            }
        }
    }
}

pub struct VideoRecorder {
    output_dir: PathBuf,
    session_id: String,
    fps: f64,
    frame_count: i32,
    frames: Vec<DynamicImage>,
    overlay_frames: Vec<DynamicImage>,
    width: u32,
    height: u32,
}

impl VideoRecorder {
    pub fn new(
        output_dir: impl AsRef<Path>,
        width: u32,
        height: u32,
        fps: f64,
    ) -> Result<Self> {
        let session_id = format!("recording_{}", chrono::Local::now().format("%Y%m%d_%H%M%S"));
        let output_dir = output_dir.as_ref().join(&session_id);
        
        // Create output directory
        std::fs::create_dir_all(&output_dir)?;
        
        Ok(Self {
            output_dir,
            session_id,
            fps,
            frame_count: 0,
            frames: Vec::new(),
            overlay_frames: Vec::new(),
            width,
            height,
        })
    }
    
    pub fn add_frame(&mut self, frame: &DynamicImage, overlay_frame: Option<&DynamicImage>) {
        self.frames.push(frame.clone());
        if let Some(overlay) = overlay_frame {
            self.overlay_frames.push(overlay.clone());
        } else {
            self.overlay_frames.push(frame.clone());
        }
        self.frame_count += 1;
    }
    
    pub fn save_videos(&self) -> Result<(PathBuf, PathBuf)> {
        let raw_video_path = self.output_dir.join("raw_video.mp4");
        let overlay_video_path = self.output_dir.join("overlay_video.mp4");
        
        // Save raw video
        self.save_video_from_frames(&self.frames, &raw_video_path)?;
        
        // Save overlay video
        self.save_video_from_frames(&self.overlay_frames, &overlay_video_path)?;
        
        Ok((raw_video_path, overlay_video_path))
    }
    
    fn save_video_from_frames(&self, frames: &[DynamicImage], output_path: &Path) -> Result<()> {
        // Create temp directory for frames
        let temp_dir = self.output_dir.join("temp_frames");
        std::fs::create_dir_all(&temp_dir)?;
        
        // Save frames as images
        for (i, frame) in frames.iter().enumerate() {
            let frame_path = temp_dir.join(format!("frame_{:05}.png", i));
            frame.save(&frame_path)?;
        }
        
        // Use ffmpeg to create video
        let status = Command::new("ffmpeg")
            .args(&[
                "-y",
                "-r", &self.fps.to_string(),
                "-i", &format!("{}/frame_%05d.png", temp_dir.display()),
                "-c:v", "libx264",
                "-preset", "medium",
                "-crf", "23",
                "-pix_fmt", "yuv420p",
                output_path.to_str().unwrap(),
            ])
            .status()
            .context("Failed to run ffmpeg")?;
        
        // Clean up temp frames
        let _ = std::fs::remove_dir_all(&temp_dir);
        
        if !status.success() {
            return Err(anyhow::anyhow!("FFmpeg video encoding failed"));
        }
        
        Ok(())
    }
    
    pub fn get_output_dir(&self) -> &Path {
        &self.output_dir
    }
}

// Video gallery management
pub struct VideoGallery {
    videos_dir: PathBuf,
    videos: Vec<VideoEntry>,
}

#[derive(Clone)]
pub struct VideoEntry {
    pub path: PathBuf,
    pub thumbnail: Option<DynamicImage>,
    pub name: String,
    pub date: chrono::DateTime<chrono::Local>,
    pub has_overlay: bool,
    pub has_csv: bool,
}

impl VideoGallery {
    pub fn new(videos_dir: impl AsRef<Path>) -> Self {
        Self {
            videos_dir: videos_dir.as_ref().to_path_buf(),
            videos: Vec::new(),
        }
    }
    
    pub fn scan_videos(&mut self) -> Result<()> {
        self.videos.clear();
        
        if !self.videos_dir.exists() {
            std::fs::create_dir_all(&self.videos_dir)?;
        }
        
        // Scan for video directories
        for entry in std::fs::read_dir(&self.videos_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Check for raw video
                let raw_video = path.join("raw_video.mp4");
                if raw_video.exists() {
                    let overlay_exists = path.join("overlay_video.mp4").exists();
                    let csv_exists = path.join("tracking_data.csv").exists();
                    
                    // Generate thumbnail from first frame
                    let thumbnail = self.extract_thumbnail(&raw_video).ok();
                    
                    let metadata = std::fs::metadata(&raw_video)?;
                    let modified = metadata.modified()?;
                    let datetime = chrono::DateTime::<chrono::Local>::from(modified);
                    
                    self.videos.push(VideoEntry {
                        path: raw_video,
                        thumbnail,
                        name: path.file_name().unwrap().to_string_lossy().to_string(),
                        date: datetime,
                        has_overlay: overlay_exists,
                        has_csv: csv_exists,
                    });
                }
            }
        }
        
        // Sort by date (newest first)
        self.videos.sort_by(|a, b| b.date.cmp(&a.date));
        
        Ok(())
    }
    
    fn extract_thumbnail(&self, video_path: &Path) -> Result<DynamicImage> {
        // Extract first frame as thumbnail
        let temp_thumb = std::env::temp_dir().join("thumb.png");
        
        let status = Command::new("ffmpeg")
            .args(&[
                "-i", video_path.to_str().unwrap(),
                "-vf", "scale=320:240",
                "-vframes", "1",
                "-y",
                temp_thumb.to_str().unwrap(),
            ])
            .status()?;
        
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to extract thumbnail"));
        }
        
        let thumb = image::open(&temp_thumb)?;
        let _ = std::fs::remove_file(&temp_thumb);
        
        Ok(thumb)
    }
    
    pub fn get_videos(&self) -> &[VideoEntry] {
        &self.videos
    }
}