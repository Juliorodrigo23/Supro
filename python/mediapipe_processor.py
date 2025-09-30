import mediapipe as mp
import numpy as np
import json
import cv2

class MediaPipeProcessor:
    def __init__(self):
        self.mp_pose = mp.solutions.pose
        self.mp_hands = mp.solutions.hands
        self.mp_drawing = mp.solutions.drawing_utils
        
        self.pose = self.mp_pose.Pose(
            static_image_mode=False,
            model_complexity=2,
            smooth_landmarks=True,
            enable_segmentation=False,
            min_detection_confidence=0.5,
            min_tracking_confidence=0.5
        )
        
        self.hands = self.mp_hands.Hands(
            static_image_mode=False,
            max_num_hands=2,
            model_complexity=1,
            min_detection_confidence=0.5,
            min_tracking_confidence=0.5
        )
    
    def process_frame(self, frame_data):
        """Process a frame and return landmarks"""
        # Convert frame data to numpy array
        frame = np.frombuffer(frame_data, dtype=np.uint8)
        frame = frame.reshape((720, 1280, 3))  # Adjust dimensions as needed
        
        # Convert BGR to RGB
        rgb_frame = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        
        # Process with MediaPipe
        pose_results = self.pose.process(rgb_frame)
        hands_results = self.hands.process(rgb_frame)
        
        result = {
            'pose_landmarks': [],
            'hand_landmarks': []
        }
        
        if pose_results.pose_landmarks:
            result['pose_landmarks'] = [
                [lm.x, lm.y, lm.z] 
                for lm in pose_results.pose_landmarks.landmark
            ]
        
        if hands_results.multi_hand_landmarks:
            for hand_landmarks in hands_results.multi_hand_landmarks:
                hand_data = [
                    [lm.x, lm.y, lm.z] 
                    for lm in hand_landmarks.landmark
                ]
                result['hand_landmarks'].append(hand_data)
        
        return json.dumps(result)
    
    def cleanup(self):
        self.pose.close()
        self.hands.close()