// src/tracking.rs - Fixed version with lazy MediaPipe initialization
use nalgebra::{Vector3, Vector6, Matrix3, Matrix6, Matrix3x6};
use std::collections::{HashMap, VecDeque};
use anyhow::Result;
use image::DynamicImage;
use crate::mediapipe_bridge::MediaPipeWrapper;
use std::time::Instant;

#[derive(Clone)]
pub struct PerformanceMetrics {
    pub avg_fps: f32,
    pub avg_processing_time: f32,
    pub tracking_confidence: f32,
    frame_times: VecDeque<f32>,
}

pub struct KalmanFilter {
    state: Vector6<f64>,  // [x, y, z, vx, vy, vz]
    covariance: Matrix6<f64>,
    process_noise: Matrix6<f64>,
    measurement_noise: Matrix3<f64>,    
    dt: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestureType {
    Pronation,
    Supination,
    None,
}

#[derive(Debug, Clone)]
pub struct GestureState {
    pub gesture_type: GestureType,
    pub confidence: f64,
    pub angle: f64,
}

#[derive(Debug, Clone)]
pub struct JointState {
    pub position: Vector3<f64>,
    pub velocity: Vector3<f64>,
    pub confidence: f64,
    pub pixel_pos: (i32, i32),
}

#[derive(Debug, Clone)]
pub struct HandState {
    pub landmarks: Vec<Vector3<f64>>,
    pub confidences: Vec<f64>,
    pub is_tracked: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TrackingResult {
    pub tracking_lost: bool,
    pub joints: HashMap<String, JointState>,
    pub hands: HashMap<String, HandState>,
    pub left_gesture: Option<GestureState>,
    pub right_gesture: Option<GestureState>,
    pub timestamp: f64,
}

pub struct ArmTracker {
    active_arms: HashMap<String, bool>,
    active_fingers: HashMap<String, bool>,
    palm_history: HashMap<String, VecDeque<Vector3<f64>>>,
    rotation_history: HashMap<String, VecDeque<f64>>,
    last_valid_gestures: HashMap<String, GestureState>,
    config: TrackerConfig,
    // Simulation state for demo
    sim_time: f64,
    mediapipe: Option<MediaPipeWrapper>,
    mediapipe_initialized: bool,
    init_attempts: u32,
    metrics: PerformanceMetrics,
    frame_counter: u32,
    adaptive_skip_rate: usize,
    last_confidence: f64,
    joint_filters: HashMap<String, KalmanFilter>,
    hand_state_cache: HashMap<String, (HandState, u32)>,
    hand_filters: HashMap<String, Vec<KalmanFilter>>,
}

#[derive(Debug, Clone)]
pub struct TrackerConfig {
    pub history_size: usize,
    pub confidence_threshold: f64,
    pub gesture_angle_threshold: f64,
    pub min_rotation_threshold: f64,
    pub rotation_smoothing_factor: f64,
    pub min_stable_frames: usize,
    pub enable_kalman: bool,          // Add this
    pub downsample_width: u32,        // Add this
    pub adaptive_frame_skip: bool,    // Add this
    pub max_frame_skip: usize,        // Add this
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            history_size: 10,
            confidence_threshold: 0.6,
            gesture_angle_threshold: 0.05,  // Lowered from 0.1
            min_rotation_threshold: 0.03,   // Lowered from 0.05
            rotation_smoothing_factor: 0.5,  // Lowered from 0.6 for faster response
            min_stable_frames: 2,
            enable_kalman: true,
            downsample_width: 640,
            adaptive_frame_skip: false,  // Disable adaptive skipping
            max_frame_skip: 1,
        }
    }
}
impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            avg_fps: 0.0,
            avg_processing_time: 0.0,
            tracking_confidence: 0.0,
            frame_times: VecDeque::with_capacity(30),
        }
    }
}


impl KalmanFilter {
    pub fn new() -> Self {
        let mut process_noise = Matrix6::identity() * 0.1;
        process_noise.fixed_view_mut::<3, 3>(3, 3).fill_diagonal(0.2);
        
        Self {
            state: Vector6::zeros(),
            covariance: Matrix6::identity(),
            process_noise,
            measurement_noise: Matrix3::identity() * 0.1,
            dt: 1.0 / 30.0,
        }
    }
    
    pub fn predict(&mut self) {
        let mut f = Matrix6::identity();
        f.fixed_view_mut::<3, 3>(0, 3).fill_diagonal(self.dt);
        
        self.state = f * self.state;
        self.covariance = f * self.covariance * f.transpose() + self.process_noise;
    }
    
    pub fn update(&mut self, measurement: Vector3<f64>) {
        // H is 3x6 matrix (observes position, not velocity)
        let mut h = Matrix3x6::<f64>::zeros();
        h[(0, 0)] = 1.0;
        h[(1, 1)] = 1.0;
        h[(2, 2)] = 1.0;
        
        // Innovation
        let y = measurement - (h * self.state);
        
        // Innovation covariance
        let s = h * self.covariance * h.transpose() + self.measurement_noise;
        
        // Kalman gain
        let k = self.covariance * h.transpose() * s.try_inverse().unwrap();
        
        // Update state and covariance
        self.state = self.state + k * y;
        let i = Matrix6::identity();
        self.covariance = (i - k * h) * self.covariance;
    }
    
    pub fn position(&self) -> Vector3<f64> {
        Vector3::new(self.state[0], self.state[1], self.state[2])
    }
}

impl ArmTracker {
    pub fn new() -> Result<Self> {
        let mut tracker = Self {
            active_arms: HashMap::new(),
            active_fingers: HashMap::new(),
            palm_history: HashMap::new(),
            rotation_history: HashMap::new(),
            last_valid_gestures: HashMap::new(),
            config: TrackerConfig::default(),
            sim_time: 0.0,
            mediapipe: None,
            mediapipe_initialized: false,
            init_attempts: 0,
            metrics: PerformanceMetrics::new(),
            frame_counter: 0,
            adaptive_skip_rate: 1,
            last_confidence: 0.0,
            joint_filters: HashMap::new(),
            hand_state_cache: HashMap::new(),
            hand_filters: HashMap::new(),
        };
        
        // Initialize tracking flags
        tracker.active_arms.insert("left".to_string(), true);
        tracker.active_arms.insert("right".to_string(), true);
        tracker.active_fingers.insert("left".to_string(), true);
        tracker.active_fingers.insert("right".to_string(), true);
        
        // Initialize history buffers
        for side in &["left", "right"] {
            tracker.palm_history.insert(
                side.to_string(),
                VecDeque::with_capacity(tracker.config.history_size)
            );
            tracker.rotation_history.insert(
                side.to_string(),
                VecDeque::with_capacity(tracker.config.history_size)
            );
            tracker.last_valid_gestures.insert(
                side.to_string(),
                GestureState {
                    gesture_type: GestureType::None,
                    confidence: 0.0,
                    angle: 0.0,
                }
            );
        }
        
        Ok(tracker)
    }

     pub fn initialize_mediapipe(&mut self) {
        if self.mediapipe_initialized {
            eprintln!("MediaPipe already initialized");
            return;
        }
        
        eprintln!("Initializing MediaPipe for camera tracking...");
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        match MediaPipeWrapper::new() {
            Ok(mp) => {
                eprintln!("✓ MediaPipe initialized successfully");
                self.mediapipe = Some(mp);
                self.mediapipe_initialized = true;
                self.init_attempts = 0;
            }
            Err(e) => {
                eprintln!("✗ MediaPipe initialization failed: {}", e);
                eprintln!("  Will use simulation mode for tracking");
            }
        }
    }
    
    pub fn shutdown_mediapipe(&mut self) {
        if self.mediapipe.is_some() {
            eprintln!("Shutting down MediaPipe...");
            self.mediapipe = None;
            self.mediapipe_initialized = false;
            self.init_attempts = 0;
            eprintln!("✓ MediaPipe shutdown complete");
        }
    }
    
    fn generate_simulation_data(&mut self, result: &mut TrackingResult) {
        let t = self.sim_time;
        
        if *self.active_arms.get("left").unwrap_or(&false) {
            result.joints.insert("left_shoulder".to_string(), JointState {
                position: Vector3::new(0.3, 0.4, 0.0),
                velocity: Vector3::zeros(),
                confidence: 0.95,
                pixel_pos: (300, 200),
            });
            
            result.joints.insert("left_elbow".to_string(), JointState {
                position: Vector3::new(0.35, 0.5 + 0.05 * t.sin(), 0.0),
                velocity: Vector3::new(0.0, 0.05 * t.cos(), 0.0),
                confidence: 0.9,
                pixel_pos: (350, 300),
            });
            
            result.joints.insert("left_wrist".to_string(), JointState {
                position: Vector3::new(0.4 + 0.1 * (t * 0.5).cos(), 0.6 + 0.1 * t.sin(), 0.0),
                velocity: Vector3::new(-0.05 * (t * 0.5).sin(), 0.1 * t.cos(), 0.0),
                confidence: 0.85,
                pixel_pos: (400, 400),
            });
            
            let gesture_type = if (t * 0.3).sin() > 0.3 {
                GestureType::Supination
            } else if (t * 0.3).sin() < -0.3 {
                GestureType::Pronation
            } else {
                GestureType::None
            };
            
            if gesture_type != GestureType::None {
                result.left_gesture = Some(GestureState {
                    gesture_type,
                    confidence: 0.7 + 0.2 * (t * 2.0).sin().abs(),
                    angle: 45.0_f64.to_radians() * (t * 0.3).sin(),
                });
            }
        }
        
        if *self.active_arms.get("right").unwrap_or(&false) {
            result.joints.insert("right_shoulder".to_string(), JointState {
                position: Vector3::new(0.7, 0.4, 0.0),
                velocity: Vector3::zeros(),
                confidence: 0.95,
                pixel_pos: (700, 200),
            });
            
            result.joints.insert("right_elbow".to_string(), JointState {
                position: Vector3::new(0.65, 0.5 + 0.05 * (t + 1.5).sin(), 0.0),
                velocity: Vector3::new(0.0, 0.05 * (t + 1.5).cos(), 0.0),
                confidence: 0.9,
                pixel_pos: (650, 300),
            });
            
            result.joints.insert("right_wrist".to_string(), JointState {
                position: Vector3::new(0.6 - 0.1 * (t * 0.5 + 1.0).cos(), 0.6 + 0.1 * (t + 1.5).sin(), 0.0),
                velocity: Vector3::new(0.05 * (t * 0.5 + 1.0).sin(), 0.1 * (t + 1.5).cos(), 0.0),
                confidence: 0.85,
                pixel_pos: (600, 400),
            });
            
            let gesture_type = if (t * 0.25 + 1.0).sin() > 0.3 {
                GestureType::Pronation
            } else if (t * 0.25 + 1.0).sin() < -0.3 {
                GestureType::Supination
            } else {
                GestureType::None
            };
            
            if gesture_type != GestureType::None {
                result.right_gesture = Some(GestureState {
                    gesture_type,
                    confidence: 0.65 + 0.25 * (t * 1.5).cos().abs(),
                    angle: 50.0_f64.to_radians() * (t * 0.25 + 1.0).sin(),
                });
            }
        }
    }
    
    pub fn toggle_arm(&mut self, side: &str) {
        if let Some(active) = self.active_arms.get_mut(side) {
            *active = !*active;
        }
    }
    
    pub fn toggle_fingers(&mut self, side: &str) {
        if let Some(active) = self.active_fingers.get_mut(side) {
            *active = !*active;
        }
    }
    
    pub fn is_using_mediapipe(&self) -> bool {
        self.mediapipe.is_some() && self.mediapipe_initialized
    }
    
    pub fn is_initializing(&self) -> bool {
        false
    }
    
    pub fn reset_mediapipe(&mut self) {
        self.shutdown_mediapipe();
        eprintln!("MediaPipe reset - call initialize_mediapipe() to retry");
    }

    // Add the missing process_hand_landmarks method
    // Add the missing process_hand_landmarks method
fn process_hand_landmarks(&mut self, hand_landmarks: &[[f64; 3]], hand_index: usize, result: &mut TrackingResult) {
    if hand_landmarks.len() < 21 {
        return;
    }
    
    let landmarks: Vec<Vector3<f64>> = hand_landmarks.iter()
        .map(|lm| Vector3::new(lm[0], lm[1], lm[2]))
        .collect();
    
    let wrist_pos = landmarks[0];
    
    // Try to match to wrist joints first (strictest)
    let side = if result.joints.contains_key("left_wrist") && 
                result.joints.contains_key("right_wrist") {
        let left_wrist = &result.joints["left_wrist"].position;
        let right_wrist = &result.joints["right_wrist"].position;
        
        let dist_left = (wrist_pos - left_wrist).norm();
        let dist_right = (wrist_pos - right_wrist).norm();
        
        eprintln!("Hand {} distances - left: {:.3}, right: {:.3}", hand_index, dist_left, dist_right);
        
        // Increased threshold - MediaPipe coordinates are 0-1 range
        const MAX_HAND_ARM_DISTANCE: f64 = 0.3; // DOUBLED from 0.15
        
        if dist_left.min(dist_right) > MAX_HAND_ARM_DISTANCE {
            eprintln!("Hand {} too far from wrists (min dist: {:.3}), trying position fallback", 
                     hand_index, dist_left.min(dist_right));
            
            // FALLBACK: Use x-position if distances too large
            if wrist_pos.x < 0.5 { "right" } else { "left" }
        } else {
            if dist_left < dist_right { "left" } else { "right" }
        }
    } else {
        eprintln!("Missing wrist joints - left: {}, right: {}", 
                 result.joints.contains_key("left_wrist"),
                 result.joints.contains_key("right_wrist"));
        
        // FALLBACK: Use x-position
        if wrist_pos.x < 0.5 { "right" } else { "left" }
    };
    
    eprintln!("Hand {} assigned to {} side", hand_index, side);
    
    // Rest of your code unchanged...
    let filters = self.get_or_create_hand_filters(side);
    let mut smoothed_landmarks = Vec::new();
    
    for (i, lm) in hand_landmarks.iter().enumerate() {
        let measurement = Vector3::new(lm[0], lm[1], lm[2]);
        filters[i].predict();
        filters[i].update(measurement);
        smoothed_landmarks.push(filters[i].position());
    }

    let hand_state = HandState {
        landmarks: smoothed_landmarks.clone(),
        confidences: vec![1.0; smoothed_landmarks.len()],
        is_tracked: true,
    };
    
    self.hand_state_cache.insert(side.to_string(), (hand_state.clone(), 0));
    result.hands.insert(side.to_string(), hand_state);
    
    // Calculate gesture if we have arm joints
    if result.joints.contains_key(&format!("{}_shoulder", side)) &&
       result.joints.contains_key(&format!("{}_elbow", side)) &&
       result.joints.contains_key(&format!("{}_wrist", side)) {
        
        let shoulder = &result.joints[&format!("{}_shoulder", side)].position;
        let elbow = &result.joints[&format!("{}_elbow", side)].position;
        let wrist = &result.joints[&format!("{}_wrist", side)].position;
        
        if let Some(gesture) = self.calculate_arm_rotation_enhanced(
            side,
            shoulder,
            elbow,
            wrist,
            Some(&smoothed_landmarks)
        ) {
            if side == "left" {
                result.left_gesture = Some(gesture);
            } else {
                result.right_gesture = Some(gesture);
            }
        }
    }
}

    pub fn process_frame_with_metrics(&mut self, frame: &DynamicImage) -> Result<(TrackingResult, PerformanceMetrics)> {
        let start = Instant::now();
        let result = self.process_frame(frame)?;
        let elapsed = start.elapsed().as_secs_f32();
        
        self.metrics.frame_times.push_front(elapsed);
        if self.metrics.frame_times.len() > 30 {
            self.metrics.frame_times.pop_back();
        }
        
        self.metrics.avg_processing_time = self.metrics.frame_times.iter().sum::<f32>() 
            / self.metrics.frame_times.len() as f32;
        self.metrics.avg_fps = 1.0 / self.metrics.avg_processing_time;
        
        // Fix: Convert f64 to f32
        self.metrics.tracking_confidence = if result.joints.is_empty() {
            0.0
        } else {
            (result.joints.values()
                .map(|j| j.confidence)
                .sum::<f64>() / result.joints.len() as f64) as f32
        };
        
        Ok((result, self.metrics.clone()))
    }

    fn get_or_create_hand_filters(&mut self, side: &str) -> &mut Vec<KalmanFilter> {
        self.hand_filters.entry(side.to_string())
            .or_insert_with(|| {
                (0..21).map(|_| KalmanFilter::new()).collect()
            })
    }

    fn calculate_arm_rotation_enhanced(
        &mut self, 
        side: &str,
        shoulder: &Vector3<f64>, 
        elbow: &Vector3<f64>, 
        wrist: &Vector3<f64>,
        hand_landmarks: Option<&Vec<Vector3<f64>>>
    ) -> Option<GestureState> {
        // Calculate forearm vector
        let forearm = (wrist - elbow).normalize();
        
        // Get palm normal if hand landmarks available
        let palm_normal = hand_landmarks.and_then(|landmarks| {
            if landmarks.len() >= 21 {
                Some(self.calculate_palm_normal(landmarks))
            } else {
                None
            }
        })?;  // Early return if no palm normal
        
        // Calculate rotation axis and angle relative to anatomical reference
        let rotation_axis = palm_normal.cross(&forearm);
        let _rotation_angle = palm_normal.dot(&forearm).clamp(-1.0, 1.0).acos();
        
        // Update palm history with anatomically aware normal
        let history = self.palm_history.get_mut(side).unwrap();
        history.push_front(palm_normal);
        if history.len() > self.config.history_size {
            history.pop_back();
        }
        
        // Need at least MIN_STABLE_FRAMES for stable detection
        if history.len() < self.config.min_stable_frames {
            return None;
        }

        // Calculate smoothed rotation angle from palm history - MATCHING C++ LOGIC
        let mut cumulative_angle = 0.0;
        let mut cumulative_axis = Vector3::zeros();
        let mut valid_samples = 0;

        for i in 1..history.len() {
            let curr_normal = history[i-1];
            let prev_normal = history[i];
            
            // Calculate rotation angle between consecutive frames
            let angle = curr_normal.dot(&prev_normal).clamp(-1.0, 1.0).acos();
            
            // Only count significant rotations - MATCHING C++
            if angle > self.config.min_rotation_threshold {
                cumulative_angle += angle;
                cumulative_axis += curr_normal.cross(&prev_normal);
                valid_samples += 1;
            }
        }

        // If we don't have enough valid samples, no significant rotation
        if valid_samples < (self.config.min_stable_frames - 1) {
            return None;
        }

        let avg_angle = cumulative_angle / valid_samples as f64;
        let _avg_axis = cumulative_axis.normalize();

        // Apply exponential smoothing to rotation history
        let rotation_history = self.rotation_history.get_mut(side).unwrap();
        rotation_history.push_front(avg_angle);
        if rotation_history.len() > self.config.history_size {
            rotation_history.pop_back();
        }

        // Calculate smoothed rotation with exponential moving average - MATCHING C++
        let mut smoothed_rotation = 0.0;
        let mut weight_sum = 0.0;
        let mut weight = 1.0;

        for rot in rotation_history.iter() {
            smoothed_rotation += rot * weight;
            weight_sum += weight;
            weight *= self.config.rotation_smoothing_factor;
        }
        smoothed_rotation /= weight_sum;

        // Only detect rotation if it's significant
        if smoothed_rotation > self.config.gesture_angle_threshold {
            // Determine rotation direction - MATCHING C++ LOGIC
            let is_supination = if side == "left" {
                // For left arm, positive rotation around forearm axis is supination
                rotation_axis.dot(&Vector3::y()) < 0.0
            } else {
                // For right arm, negative rotation around forearm axis is supination
                rotation_axis.dot(&Vector3::y()) < 0.0
            };
            
            Some(GestureState {
                gesture_type: if is_supination { 
                    GestureType::Supination 
                } else { 
                    GestureType::Pronation 
                },
                confidence: (smoothed_rotation / (self.config.gesture_angle_threshold * 2.0)).min(1.0),
                angle: smoothed_rotation,
            })
        } else {
            None
        }
    }

    fn calculate_palm_normal(&self, landmarks: &[Vector3<f64>]) -> Vector3<f64> {
        // MediaPipe hand landmark indices - matching C++ exactly
        const WRIST: usize = 0;
        const THUMB_CMC: usize = 1;
        const INDEX_MCP: usize = 5;
        const MIDDLE_MCP: usize = 9;
        const RING_MCP: usize = 13;
        const PINKY_MCP: usize = 17;
        const MIDDLE_PIP: usize = 10;
        const MIDDLE_TIP: usize = 12;

        // Get key points
        let wrist = landmarks[WRIST];
        let thumb_cmc = landmarks[THUMB_CMC];
        let index_mcp = landmarks[INDEX_MCP];
        let middle_mcp = landmarks[MIDDLE_MCP];
        let ring_mcp = landmarks[RING_MCP];
        let pinky_mcp = landmarks[PINKY_MCP];
        let middle_pip = landmarks[MIDDLE_PIP];
        let middle_tip = landmarks[MIDDLE_TIP];

        // Calculate robust palm direction vectors
        let palm_center = (index_mcp + middle_mcp + ring_mcp + pinky_mcp) / 4.0;
        let palm_direction = (palm_center - wrist).normalize();
        
        // Calculate palm width vector (perpendicular to thumb-pinky line)
        let thumb_pinky = (pinky_mcp - thumb_cmc).normalize();
        
        // Calculate finger direction (using middle finger as reference)
        let finger_direction = (middle_tip - middle_mcp).normalize();
        
        // Calculate palm normal using multiple reference vectors - THIS IS THE KEY DIFFERENCE
        let normal1 = thumb_pinky.cross(&palm_direction);
        let normal2 = thumb_pinky.cross(&finger_direction);
        
        // Combine normals with weights (equal weighting like C++)
        let weighted_normal = (normal1 + normal2).normalize();
        
        weighted_normal
    }


// In tracking.rs, update the process_frame method around line 500:
pub fn process_frame(&mut self, frame: &DynamicImage) -> Result<TrackingResult> {
    let mut result = TrackingResult::default();
    result.timestamp = self.sim_time;
    self.sim_time += 0.033;
    self.frame_counter += 1;
    
    if let Some(ref mut mp) = self.mediapipe {
        match mp.process_image(frame) {
            Ok(mp_result) => {
                if mp_result.pose_landmarks.len() > 16 {
                    self.process_pose_with_kalman(&mp_result.pose_landmarks, &mut result);
                    
                    for (i, hand_lms) in mp_result.hand_landmarks.iter().enumerate() {
                        self.process_hand_landmarks(hand_lms, i, &mut result);
                    }
                    
                    // Keep gestures from last_valid_gestures if not detected this frame
                    if result.left_gesture.is_none() {
                        if let Some(last_gesture) = self.last_valid_gestures.get("left") {
                            if last_gesture.gesture_type != GestureType::None {
                                result.left_gesture = Some(last_gesture.clone());
                            }
                        }
                    } else if let Some(gesture) = &result.left_gesture {
                        self.last_valid_gestures.insert("left".to_string(), gesture.clone());
                    }
                    
                    if result.right_gesture.is_none() {
                        if let Some(last_gesture) = self.last_valid_gestures.get("right") {
                            if last_gesture.gesture_type != GestureType::None {
                                result.right_gesture = Some(last_gesture.clone());
                            }
                        }
                    } else if let Some(gesture) = &result.right_gesture {
                        self.last_valid_gestures.insert("right".to_string(), gesture.clone());
                    }
                    
                    result.tracking_lost = false;
                }
            }
            Err(e) => {
                eprintln!("MediaPipe error: {}", e);
                result.tracking_lost = true;
            }
        }
    } else {
        self.generate_simulation_data(&mut result);
    }
    
    Ok(result)
}

    fn process_pose_with_kalman(&mut self, landmarks: &[[f64; 3]], result: &mut TrackingResult) {
        const LEFT_SHOULDER: usize = 11;
        const RIGHT_SHOULDER: usize = 12;
        const LEFT_ELBOW: usize = 13;
        const RIGHT_ELBOW: usize = 14;
        const LEFT_WRIST: usize = 15;
        const RIGHT_WRIST: usize = 16;
        
        let joint_indices = [
            ("left_shoulder", LEFT_SHOULDER),
            ("right_shoulder", RIGHT_SHOULDER),
            ("left_elbow", LEFT_ELBOW),
            ("right_elbow", RIGHT_ELBOW),
            ("left_wrist", LEFT_WRIST),
            ("right_wrist", RIGHT_WRIST),
        ];
        
        for (name, idx) in joint_indices.iter() {
            if *idx < landmarks.len() {
                let measurement = Vector3::new(
                    landmarks[*idx][0],
                    landmarks[*idx][1],
                    landmarks[*idx][2],
                );
                
                // Use or create Kalman filter for this joint
                let kalman = self.joint_filters
                    .entry(name.to_string())
                    .or_insert_with(KalmanFilter::new);
                
                kalman.predict();
                kalman.update(measurement);
                
                let smoothed_pos = kalman.position();
                
                result.joints.insert(name.to_string(), JointState {
                    position: smoothed_pos,
                    velocity: Vector3::zeros(), // Could calculate from Kalman state
                    confidence: 0.9,
                    pixel_pos: (
                        (smoothed_pos.x * 640.0) as i32,
                        (smoothed_pos.y * 480.0) as i32
                    ),
                });
            }
        }
    }



}
