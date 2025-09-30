// src/data.rs
use crate::tracking::{TrackingResult, GestureType};
use csv::Writer;
use std::path::{Path, PathBuf};
use std::fs::File;
use anyhow::{Result, Context};
use chrono::{DateTime, Local};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct TrackingRecord {
    timestamp: f64,
    frame: i32,
    tracking_lost: bool,
    
    // Joint positions
    left_shoulder_x: Option<f64>,
    left_shoulder_y: Option<f64>,
    left_shoulder_z: Option<f64>,
    left_shoulder_confidence: Option<f64>,
    
    right_shoulder_x: Option<f64>,
    right_shoulder_y: Option<f64>,
    right_shoulder_z: Option<f64>,
    right_shoulder_confidence: Option<f64>,
    
    left_elbow_x: Option<f64>,
    left_elbow_y: Option<f64>,
    left_elbow_z: Option<f64>,
    left_elbow_confidence: Option<f64>,
    
    right_elbow_x: Option<f64>,
    right_elbow_y: Option<f64>,
    right_elbow_z: Option<f64>,
    right_elbow_confidence: Option<f64>,
    
    left_wrist_x: Option<f64>,
    left_wrist_y: Option<f64>,
    left_wrist_z: Option<f64>,
    left_wrist_confidence: Option<f64>,
    
    right_wrist_x: Option<f64>,
    right_wrist_y: Option<f64>,
    right_wrist_z: Option<f64>,
    right_wrist_confidence: Option<f64>,
    
    // Gestures
    left_gesture: Option<String>,
    left_gesture_confidence: Option<f64>,
    left_gesture_angle: Option<f64>,
    
    right_gesture: Option<String>,
    right_gesture_confidence: Option<f64>,
    right_gesture_angle: Option<f64>,
}

pub struct DataExporter {
    output_dir: PathBuf,
    session_name: String,
    tracking_data: Vec<TrackingResult>,
    timestamps: Vec<f64>,
}

impl DataExporter {
    pub fn new(output_dir: impl AsRef<Path>, session_name: Option<String>) -> Self {
        let session_name = session_name.unwrap_or_else(|| {
            format!("session_{}", Local::now().format("%Y%m%d_%H%M%S"))
        });
        
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
            session_name,
            tracking_data: Vec::new(),
            timestamps: Vec::new(),
        }
    }
    
    pub fn add_frame(&mut self, result: TrackingResult, timestamp: f64) {
        self.tracking_data.push(result);
        self.timestamps.push(timestamp);
    }
    
    pub fn export_csv(&self) -> Result<PathBuf> {
        let csv_path = self.output_dir
            .join(&self.session_name)
            .join("tracking_data.csv");
        
        // Create directory if it doesn't exist
        if let Some(parent) = csv_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = File::create(&csv_path)?;
        let mut writer = Writer::from_writer(file);
        
        for (i, (result, timestamp)) in self.tracking_data.iter()
            .zip(self.timestamps.iter())
            .enumerate() 
        {
            let record = self.create_record(i as i32, *timestamp, result);
            writer.serialize(record)?;
        }
        
        writer.flush()?;
        Ok(csv_path)
    }
    
    fn create_record(&self, frame: i32, timestamp: f64, result: &TrackingResult) -> TrackingRecord {
        let mut record = TrackingRecord {
            timestamp,
            frame,
            tracking_lost: result.tracking_lost,
            left_shoulder_x: None,
            left_shoulder_y: None,
            left_shoulder_z: None,
            left_shoulder_confidence: None,
            right_shoulder_x: None,
            right_shoulder_y: None,
            right_shoulder_z: None,
            right_shoulder_confidence: None,
            left_elbow_x: None,
            left_elbow_y: None,
            left_elbow_z: None,
            left_elbow_confidence: None,
            right_elbow_x: None,
            right_elbow_y: None,
            right_elbow_z: None,
            right_elbow_confidence: None,
            left_wrist_x: None,
            left_wrist_y: None,
            left_wrist_z: None,
            left_wrist_confidence: None,
            right_wrist_x: None,
            right_wrist_y: None,
            right_wrist_z: None,
            right_wrist_confidence: None,
            left_gesture: None,
            left_gesture_confidence: None,
            left_gesture_angle: None,
            right_gesture: None,
            right_gesture_confidence: None,
            right_gesture_angle: None,
        };
        
        // Fill in joint data
        for (name, joint) in &result.joints {
            match name.as_str() {
                "left_shoulder" => {
                    record.left_shoulder_x = Some(joint.position.x);
                    record.left_shoulder_y = Some(joint.position.y);
                    record.left_shoulder_z = Some(joint.position.z);
                    record.left_shoulder_confidence = Some(joint.confidence);
                }
                "right_shoulder" => {
                    record.right_shoulder_x = Some(joint.position.x);
                    record.right_shoulder_y = Some(joint.position.y);
                    record.right_shoulder_z = Some(joint.position.z);
                    record.right_shoulder_confidence = Some(joint.confidence);
                }
                "left_elbow" => {
                    record.left_elbow_x = Some(joint.position.x);
                    record.left_elbow_y = Some(joint.position.y);
                    record.left_elbow_z = Some(joint.position.z);
                    record.left_elbow_confidence = Some(joint.confidence);
                }
                "right_elbow" => {
                    record.right_elbow_x = Some(joint.position.x);
                    record.right_elbow_y = Some(joint.position.y);
                    record.right_elbow_z = Some(joint.position.z);
                    record.right_elbow_confidence = Some(joint.confidence);
                }
                "left_wrist" => {
                    record.left_wrist_x = Some(joint.position.x);
                    record.left_wrist_y = Some(joint.position.y);
                    record.left_wrist_z = Some(joint.position.z);
                    record.left_wrist_confidence = Some(joint.confidence);
                }
                "right_wrist" => {
                    record.right_wrist_x = Some(joint.position.x);
                    record.right_wrist_y = Some(joint.position.y);
                    record.right_wrist_z = Some(joint.position.z);
                    record.right_wrist_confidence = Some(joint.confidence);
                }
                _ => {}
            }
        }
        
        // Fill in gesture data
        if let Some(left_gesture) = &result.left_gesture {
            record.left_gesture = Some(format!("{:?}", left_gesture.gesture_type));
            record.left_gesture_confidence = Some(left_gesture.confidence);
            record.left_gesture_angle = Some(left_gesture.angle);
        }
        
        if let Some(right_gesture) = &result.right_gesture {
            record.right_gesture = Some(format!("{:?}", right_gesture.gesture_type));
            record.right_gesture_confidence = Some(right_gesture.confidence);
            record.right_gesture_angle = Some(right_gesture.angle);
        }
        
        record
    }
    
    pub fn generate_report(&self) -> Result<PathBuf> {
        let report_path = self.output_dir
            .join(&self.session_name)
            .join("report.html");
        
        // Create directory if it doesn't exist
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let html_content = self.create_html_report()?;
        std::fs::write(&report_path, html_content)?;
        
        Ok(report_path)
    }
    
    fn create_html_report(&self) -> Result<String> {
        let total_frames = self.tracking_data.len();
        let tracking_lost_count = self.tracking_data.iter()
            .filter(|r| r.tracking_lost)
            .count();
        
        let left_supination_count = self.tracking_data.iter()
            .filter(|r| r.left_gesture.as_ref()
                .map(|g| g.gesture_type == GestureType::Supination)
                .unwrap_or(false))
            .count();
        
        let left_pronation_count = self.tracking_data.iter()
            .filter(|r| r.left_gesture.as_ref()
                .map(|g| g.gesture_type == GestureType::Pronation)
                .unwrap_or(false))
            .count();
        
        let html = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Arm Tracking Report - {}</title>
    <style>
        body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; margin: 40px; background: #f5f5f5; }}
        h1 {{ color: #333; }}
        .stats {{ background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .stat-item {{ margin: 10px 0; }}
        .stat-label {{ font-weight: bold; color: #666; }}
        .stat-value {{ color: #4682EA; font-size: 1.2em; }}
    </style>
</head>
<body>
    <h1>Arm Tracking Session Report</h1>
    <div class="stats">
        <h2>Session: {}</h2>
        <div class="stat-item">
            <span class="stat-label">Total Frames:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-item">
            <span class="stat-label">Tracking Success Rate:</span>
            <span class="stat-value">{:.1}%</span>
        </div>
        <div class="stat-item">
            <span class="stat-label">Left Arm Supination:</span>
            <span class="stat-value">{} frames</span>
        </div>
        <div class="stat-item">
            <span class="stat-label">Left Arm Pronation:</span>
            <span class="stat-value">{} frames</span>
        </div>
    </div>
</body>
</html>
        "#,
            self.session_name,
            self.session_name,
            total_frames,
            (1.0 - tracking_lost_count as f64 / total_frames as f64) * 100.0,
            left_supination_count,
            left_pronation_count
        );
        
        Ok(html)
    }
}