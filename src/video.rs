// src/video.rs - Working version without OpenCV
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use image::{DynamicImage, RgbaImage};

pub enum VideoSource {
    Camera(usize),
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
        // For now, just store the camera index
        // In production, you'd integrate with a camera library
        Ok(VideoSource::Camera(index as usize))
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
        // Create a test pattern for demo
        let mut img = RgbaImage::new(1280, 720);
        
        // Generate a gradient pattern
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let r = (x as f32 / 1280.0 * 255.0) as u8;
            let g = (y as f32 / 720.0 * 255.0) as u8;
            let b = 128;
            *pixel = image::Rgba([r, g, b, 255]);
        }
        
        Ok(DynamicImage::ImageRgba8(img))
    }
    
    pub fn get_info(&self) -> Option<&VideoInfo> {
        match self {
            VideoSource::File(_, info) => Some(info),
            _ => None,
        }
    }
    
    pub fn seek(&mut self, frame_number: i32) -> Result<()> {
        if let VideoSource::File(_, ref mut info) = self {
            info.current_frame = frame_number;
        }
        Ok(())
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