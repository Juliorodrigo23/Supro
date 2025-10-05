// src/data.rs
use crate::tracking::{TrackingResult, GestureType};
use csv::Writer;
use std::path::{Path, PathBuf};
use std::fs::File;
use anyhow::Result;
use chrono::Local;
use serde::Serialize;
use nalgebra::Vector3;

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

    // Hand landmarks - Finger angles
    // Left hand
    left_thumb_angle: Option<f64>,
    left_index_angle: Option<f64>,
    left_middle_angle: Option<f64>,
    left_ring_angle: Option<f64>,
    left_pinky_angle: Option<f64>,
    left_wrist_flexion: Option<f64>,

    // Right hand
    right_thumb_angle: Option<f64>,
    right_index_angle: Option<f64>,
    right_middle_angle: Option<f64>,
    right_ring_angle: Option<f64>,
    right_pinky_angle: Option<f64>,
    right_wrist_flexion: Option<f64>,
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
            left_thumb_angle: None,
            left_index_angle: None,
            left_middle_angle: None,
            left_ring_angle: None,
            left_pinky_angle: None,
            left_wrist_flexion: None,
            right_thumb_angle: None,
            right_index_angle: None,
            right_middle_angle: None,
            right_ring_angle: None,
            right_pinky_angle: None,
            right_wrist_flexion: None,
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

        // Calculate finger angles for left hand
        if let Some(left_hand) = result.hands.get("left") {
            if left_hand.is_tracked && left_hand.landmarks.len() >= 21 {
                record.left_thumb_angle = Some(Self::calculate_finger_angle(&left_hand.landmarks, 1, 2, 3, 4));
                record.left_index_angle = Some(Self::calculate_finger_angle(&left_hand.landmarks, 5, 6, 7, 8));
                record.left_middle_angle = Some(Self::calculate_finger_angle(&left_hand.landmarks, 9, 10, 11, 12));
                record.left_ring_angle = Some(Self::calculate_finger_angle(&left_hand.landmarks, 13, 14, 15, 16));
                record.left_pinky_angle = Some(Self::calculate_finger_angle(&left_hand.landmarks, 17, 18, 19, 20));
                record.left_wrist_flexion = Some(Self::calculate_wrist_angle(&left_hand.landmarks));
            }
        }

        // Calculate finger angles for right hand
        if let Some(right_hand) = result.hands.get("right") {
            if right_hand.is_tracked && right_hand.landmarks.len() >= 21 {
                record.right_thumb_angle = Some(Self::calculate_finger_angle(&right_hand.landmarks, 1, 2, 3, 4));
                record.right_index_angle = Some(Self::calculate_finger_angle(&right_hand.landmarks, 5, 6, 7, 8));
                record.right_middle_angle = Some(Self::calculate_finger_angle(&right_hand.landmarks, 9, 10, 11, 12));
                record.right_ring_angle = Some(Self::calculate_finger_angle(&right_hand.landmarks, 13, 14, 15, 16));
                record.right_pinky_angle = Some(Self::calculate_finger_angle(&right_hand.landmarks, 17, 18, 19, 20));
                record.right_wrist_flexion = Some(Self::calculate_wrist_angle(&right_hand.landmarks));
            }
        }

        record
    }

    // Calculate finger angle based on landmarks (MCP, PIP, DIP, TIP)
    fn calculate_finger_angle(landmarks: &[Vector3<f64>], mcp: usize, pip: usize, dip: usize, tip: usize) -> f64 {
        if landmarks.len() <= tip {
            return 0.0;
        }

        // Calculate vectors
        let v1 = landmarks[pip] - landmarks[mcp];
        let v2 = landmarks[dip] - landmarks[pip];
        let v3 = landmarks[tip] - landmarks[dip];

        // Calculate angles between consecutive segments
        let angle1 = Self::angle_between_vectors(&v1, &v2);
        let angle2 = Self::angle_between_vectors(&v2, &v3);

        // Return average angle (in degrees)
        ((angle1 + angle2) / 2.0).to_degrees()
    }

    // Calculate wrist flexion angle
    fn calculate_wrist_angle(landmarks: &[Vector3<f64>]) -> f64 {
        if landmarks.len() < 21 {
            return 0.0;
        }

        // Use wrist (0), middle finger MCP (9), and middle finger tip (12)
        let wrist = landmarks[0];
        let mcp = landmarks[9];
        let tip = landmarks[12];

        let v1 = mcp - wrist;
        let v2 = tip - mcp;

        Self::angle_between_vectors(&v1, &v2).to_degrees()
    }

    // Helper function to calculate angle between two vectors
    fn angle_between_vectors(v1: &Vector3<f64>, v2: &Vector3<f64>) -> f64 {
        let dot = v1.dot(v2);
        let mag1 = v1.norm();
        let mag2 = v2.norm();

        if mag1 == 0.0 || mag2 == 0.0 {
            return 0.0;
        }

        let cos_angle = (dot / (mag1 * mag2)).clamp(-1.0, 1.0);
        cos_angle.acos()
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