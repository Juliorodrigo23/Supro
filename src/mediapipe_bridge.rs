// src/mediapipe_bridge.rs
use anyhow::{Result, Context};
use nalgebra::Vector3;
use std::process::{Command, Stdio, Child};
use std::io::{Write, BufRead, BufReader};
use serde::{Deserialize, Serialize};
use image::DynamicImage;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize, Deserialize)]
struct MediaPipeFrame {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct MediaPipeResult {
    pub pose_landmarks: Vec<[f64; 3]>,    // Make public
    pub hand_landmarks: Vec<Vec<[f64; 3]>>, // Make public
}

pub struct MediaPipeWrapper {
    python_process: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl MediaPipeWrapper {
    pub fn new() -> Result<Self> {
        eprintln!("=== MediaPipe Initialization ===");
        
        // Try multiple paths to find the Python script
        let possible_paths = vec![
            // For bundled app
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|p| p.join("../Resources/python/mediapipe_service.py"))),
            // For development
            std::env::current_dir().ok().map(|d| d.join("python/mediapipe_service.py")),
        ];
        
        let script_path = possible_paths
            .into_iter()
            .flatten()
            .find(|p| p.exists())
            .ok_or_else(|| anyhow::anyhow!("Could not find mediapipe_service.py"))?;
        
        eprintln!("Found Python script at: {}", script_path.display());
        
        let mut child = Command::new("python3")
            .arg(&script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn Python MediaPipe process - is Python3 installed?")?;
        
        eprintln!("Python process spawned with PID: {:?}", child.id());
        
        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?;
        let mut stdout = BufReader::new(
            child.stdout.take()
                .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?
        );
        
        // Wait for ready signal with timeout
        eprintln!("Waiting for MediaPipe service to be ready...");
        let mut ready_line = String::new();
        let start = Instant::now();
        let timeout = Duration::from_secs(15); // Increased timeout
        
        loop {
            if start.elapsed() > timeout {
                eprintln!("Timeout after {:?}", start.elapsed());
                return Err(anyhow::anyhow!("Timeout waiting for MediaPipe service"));
            }
            
            match stdout.read_line(&mut ready_line) {
                Ok(0) => {
                    eprintln!("Python process closed unexpectedly");
                    return Err(anyhow::anyhow!("Python process terminated"));
                }
                Ok(_) => {
                    eprintln!("Received from Python: {}", ready_line.trim());
                    if ready_line.trim() == "READY" {
                        eprintln!("✓ MediaPipe service is ready!");
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from Python: {}", e);
                    return Err(anyhow::anyhow!("Failed to read from Python process: {}", e));
                }
            }
        }
        
        eprintln!("=== MediaPipe Initialized Successfully ===");
        
        Ok(Self {
            python_process: child,
            stdin,
            stdout,
        })
    }
    
    pub fn process_image(&mut self, image: &DynamicImage) -> Result<MediaPipeResult> {
        // Convert image to RGB bytes
        let rgb = image.to_rgb8();
        let frame_data = MediaPipeFrame {
            width: rgb.width(),
            height: rgb.height(),
            data: rgb.into_raw(),
        };
        
        eprintln!("Sending frame: {}x{} ({} bytes)", 
                 frame_data.width, frame_data.height, frame_data.data.len());
        
        // Send frame to Python
        let json_data = serde_json::to_string(&frame_data)?;
        writeln!(self.stdin, "{}", json_data)?;
        self.stdin.flush()?;
        
        // Read response
        let mut response = String::new();
        self.stdout.read_line(&mut response)
            .context("Failed to read response from MediaPipe")?;
        
        if response.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty response from MediaPipe"));
        }
        
        // Parse result
        let result: MediaPipeResult = serde_json::from_str(&response)
            .context("Failed to parse MediaPipe response")?;
        
        if !result.pose_landmarks.is_empty() {
            eprintln!("✓ Received {} pose landmarks", result.pose_landmarks.len());
        } else {
            eprintln!("✗ No pose landmarks detected");
        }
        
        Ok(result)
    }
    
    pub fn get_pose_landmarks(&mut self, image: &DynamicImage) -> Result<Vec<Vector3<f64>>> {
        let result = self.process_image(image)?;
        Ok(result.pose_landmarks.into_iter()
            .map(|[x, y, z]| Vector3::new(x, y, z))
            .collect())
    }
    
    pub fn get_hand_landmarks(&mut self, image: &DynamicImage) -> Result<Vec<Vec<Vector3<f64>>>> {
        let result = self.process_image(image)?;
        Ok(result.hand_landmarks.into_iter()
            .map(|hand| hand.into_iter()
                .map(|[x, y, z]| Vector3::new(x, y, z))
                .collect())
            .collect())
    }
}

impl Drop for MediaPipeWrapper {
    fn drop(&mut self) {
        eprintln!("Shutting down MediaPipe service");
        let _ = self.python_process.kill();
    }
}