#!/usr/bin/env python3
import sys
import json
import numpy as np
import traceback

try:
    import mediapipe as mp
    import cv2
except ImportError as e:
    print(f"Error: Missing required packages: {e}", file=sys.stderr)
    print("Install with: pip3 install mediapipe opencv-python numpy", file=sys.stderr)
    sys.exit(1)

class MediaPipeService:
    def __init__(self):
        # Initialize pose tracking
        self.mp_pose = mp.solutions.pose
        self.pose = self.mp_pose.Pose(
            static_image_mode=False,
            model_complexity=1,  # Changed from 0 to 1 for better accuracy
            smooth_landmarks=True,
            enable_segmentation=False,
            min_detection_confidence=0.5,
            min_tracking_confidence=0.5  # Increased from 0.3
        )
        
        # Initialize hand tracking - ENABLED
        self.mp_hands = mp.solutions.hands
        self.hands = self.mp_hands.Hands(
            static_image_mode=False,
            max_num_hands=2,
            model_complexity=1,  # Changed from 0 to 1
            min_detection_confidence=0.4,  # Lowered from 0.5
            min_tracking_confidence=0.4
        )
        
        print("MediaPipe service initialized with hands enabled", file=sys.stderr)
    
    def process_frame(self, frame_data):
        try:
            width = frame_data['width']
            height = frame_data['height']
            data = np.array(frame_data['data'], dtype=np.uint8)
            
            # Reshape frame
            frame = data.reshape((height, width, 3))
            
            # Process with MediaPipe
            pose_results = self.pose.process(frame)
            hands_results = self.hands.process(frame)
            
            result = {
                'pose_landmarks': [],
                'hand_landmarks': []
            }
            
            if pose_results.pose_landmarks:
                result['pose_landmarks'] = [
                    [lm.x, lm.y, lm.z] 
                    for lm in pose_results.pose_landmarks.landmark
                ]
            
            # PROCESS HANDS - ENABLED
            if hands_results.multi_hand_landmarks:
                for hand_landmarks in hands_results.multi_hand_landmarks:
                    hand_data = [
                        [lm.x, lm.y, lm.z] 
                        for lm in hand_landmarks.landmark
                    ]
                    result['hand_landmarks'].append(hand_data)
            
            return result
            
        except Exception as e:
            print(f"Error processing frame: {e}", file=sys.stderr)
            traceback.print_exc(file=sys.stderr)
            return {'pose_landmarks': [], 'hand_landmarks': []}
    
    def run(self):
        print("READY", file=sys.stdout)
        sys.stdout.flush()
        print("MediaPipe service ready with hands tracking", file=sys.stderr)
        
        while True:
            try:
                line = sys.stdin.readline()
                if not line:
                    print("End of input stream", file=sys.stderr)
                    break
                
                frame_data = json.loads(line)
                result = self.process_frame(frame_data)
                print(json.dumps(result))
                sys.stdout.flush()
                
            except json.JSONDecodeError as e:
                print(f"JSON decode error: {e}", file=sys.stderr)
                print(json.dumps({'pose_landmarks': [], 'hand_landmarks': []}))
                sys.stdout.flush()
            except Exception as e:
                print(f"Service error: {e}", file=sys.stderr)
                traceback.print_exc(file=sys.stderr)
                print(json.dumps({'pose_landmarks': [], 'hand_landmarks': []}))
                sys.stdout.flush()

if __name__ == '__main__':
    try:
        service = MediaPipeService()
        service.run()
    except Exception as e:
        print(f"Failed to start service: {e}", file=sys.stderr)
        sys.exit(1)