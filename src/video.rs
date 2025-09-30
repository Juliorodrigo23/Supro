// src/video.rs - Fixed version
use std::path::{Path, PathBuf};
use anyhow::Result;
use image::{DynamicImage, ImageBuffer};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;
use std::sync::{Arc, Mutex};

pub enum VideoSource {
    Camera(Arc<Mutex<Camera>>),
    File(PathBuf, VideoInfo),
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
        
        // Create a CameraFormat with the desired settings
        use nokhwa::utils::{CameraFormat, FrameFormat, Resolution};
        
        let format = CameraFormat::new(
            Resolution::new(640, 480),
            FrameFormat::MJPEG,  // or FrameFormat::YUYV
            30,  // frame rate
        );
        
        // Use the Exact variant with the CameraFormat
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
        let path = path.as_ref().to_path_buf();
        let info = VideoInfo {
            path: path.clone(),
            fps: 30.0,
            frame_count: 1000,
            width: 1280,
            height: 720,
            current_frame: 0,
        };
        Ok(VideoSource::File(path, info))
    }
    
    pub fn read_frame(&mut self) -> Result<DynamicImage> {
        match self {
            VideoSource::Camera(camera) => {
                let mut cam = camera.lock().unwrap();
                
                // Open stream if not already open
                if !cam.is_stream_open() {
                    cam.open_stream()
                        .map_err(|e| anyhow::anyhow!("Failed to open camera stream: {}", e))?;
                }
                
                // Capture frame
                let frame = cam.frame()
                    .map_err(|e| anyhow::anyhow!("Failed to capture frame: {}", e))?;
                
                // Get frame data
                let decoded = frame.decode_image::<RgbFormat>()
                    .map_err(|e| anyhow::anyhow!("Failed to decode frame: {}", e))?;
                
                // Convert to DynamicImage
                let width = decoded.width();
                let height = decoded.height();
                let rgb_data = decoded.into_vec();  // Changed from into_flat_vec()
                
                // Convert RGB to RGBA
                let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
                for chunk in rgb_data.chunks(3) {
                    rgba_data.push(chunk[0]);
                    rgba_data.push(chunk[1]);
                    rgba_data.push(chunk[2]);
                    rgba_data.push(255);
                }
                
                let img = ImageBuffer::from_raw(width, height, rgba_data)
                    .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;
                
                Ok(DynamicImage::ImageRgba8(img))
            }
            VideoSource::File(_, _) => {
                // For file playback, return a placeholder for now
                Ok(DynamicImage::new_rgba8(1280, 720))
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
                    frame_count: -1, // Infinite for camera
                    width: resolution.width() as i32,
                    height: resolution.height() as i32,
                    current_frame: 0,
                })
            }
            VideoSource::File(_, info) => Some(info.clone()),
        }
    }
    
    pub fn seek(&mut self, frame_number: i32) -> Result<()> {
        if let VideoSource::File(_, ref mut info) = self {
            info.current_frame = frame_number;
        }
        Ok(())
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
    output_path: PathBuf,
    fps: f64,
    frame_count: i32,
    frames: Vec<DynamicImage>,
}

impl VideoRecorder {
    pub fn new(
        output_path: impl AsRef<Path>,
        _width: i32,
        _height: i32,
        fps: f64,
    ) -> Result<Self> {
        let path = output_path.as_ref().to_path_buf();
        
        // Ensure output directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        Ok(Self {
            output_path: path,
            fps,
            frame_count: 0,
            frames: Vec::new(),
        })
    }
    
    pub fn write_frame(&mut self, frame: &DynamicImage) -> Result<()> {
        self.frames.push(frame.clone());
        self.frame_count += 1;
        Ok(())
    }
    
    pub fn finalize(self) -> Result<PathBuf> {
        // For demo, save frames as individual images
        if let Some(parent) = self.output_path.parent() {
            for (i, frame) in self.frames.iter().enumerate() {
                let path = parent.join(format!("frame_{:04}.png", i));
                frame.save(&path)?;
            }
        }
        Ok(self.output_path)
    }
    
    pub fn get_frame_count(&self) -> i32 {
        self.frame_count
    }
    
    pub fn get_duration(&self) -> f64 {
        self.frame_count as f64 / self.fps
    }
}