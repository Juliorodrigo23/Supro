# Supro Arm Tracker

An Arm Rotation Tracking System by Julio Contreras — Under Dr. Ortiz's Research Lab 

![Supro Logo](SuproLogo.gif)

Supro Arm Tracker is a sophisticated motion tracking application built in Rust for analyzing forearm rotation patterns (supination and pronation) in real-time from a webcam or pre-recorded video files. It uses Google's MediaPipe for robust skeleton and hand tracking, combined with custom algorithms and Kalman filters for smooth and accurate gesture analysis.

---

## Core Features

* **Multi-Mode Operation**:
    * 🎥 **Live Camera Mode**: Analyze arm movements directly from a webcam feed[cite: 3, 39].
    * 📁 **Video File Mode**: Process and analyze pre-recorded video files[cite: 3, 41, 65].
    * 📊 **Analysis Mode**: Review, chart, and export data from previous sessions[cite: 57, 102].
* **Advanced Tracking & Visualization**:
    * Real-time skeleton tracking overlay showing shoulders, elbows, and wrists[cite: 145, 150].
    * Detailed hand and finger landmark tracking[cite: 154, 158].
    * **Dual View** UI to compare raw video feed against the tracking overlay side-by-side[cite: 59, 82].
* **Gesture Recognition**:
    * Detects and classifies forearm movements into **Supination** or **Pronation**[cite: 319, 412].
    * Displays real-time gesture classification, confidence levels, and rotation angles[cite: 111, 117, 131, 132].
* **Data Recording & Export**:
    * Record tracking sessions for later analysis[cite: 45, 171, 175].
    * Export detailed, frame-by-frame joint and gesture data to a **CSV file**[cite: 105, 228].
    * Generate a summary **HTML report** with session statistics[cite: 106, 246].
* **Configurable Settings**:
    * Adjust tracking parameters like confidence threshold and smoothing factor[cite: 186, 187].
    * Toggle tracking for the left arm, right arm, and fingers independently[cite: 180].

---

## Technology Stack

* **Primary Language**: **Rust**
* **GUI Framework**: **`eframe` / `egui`** for the immediate-mode graphical user interface[cite: 528].
* **Computer Vision Backend**: **Google MediaPipe** (Pose and Hand models) running in a separate Python process[cite: 551, 552, 564].
* **Camera & Video Input**: The `nokhwa` crate for camera capture in Rust[cite: 502].
* **Mathematical Filtering**: A custom **Kalman Filter** implementation for smoothing joint positions, written using the `nalgebra` crate[cite: 318, 325, 441].
* **Inter-Process Communication**: A lightweight bridge between Rust and Python using standard input/output (stdin/stdout) to exchange JSON data[cite: 286, 293, 307, 309].

---

## How It Works

The application's architecture is designed to leverage the strengths of both Rust and Python:

1.  **UI & Main Logic (Rust)**: The core application is a native `eframe` GUI that manages the user interface, state, and controls[cite: 197].
2.  **Python Subprocess**: On starting the camera, the Rust application spawns a Python script (`python/mediapipe_service.py`) as a background process[cite: 293].
3.  **Data Exchange**:
    * Rust captures a frame from the camera[cite: 511].
    * The frame is converted to RGB data, serialized into a JSON object, and sent to the Python script's `stdin`[cite: 305, 307].
    * The Python script reads the JSON, processes the frame using the MediaPipe library, and extracts pose and hand landmarks[cite: 554, 561].
    * The resulting landmark data is serialized back into a JSON string and printed to `stdout`[cite: 561].
4.  **Analysis & Rendering (Rust)**:
    * The Rust application reads the JSON response from the Python process's `stdout`[cite: 309].
    * The raw landmark data is smoothed using Kalman filters[cite: 442].
    * Custom algorithms in `tracking.rs` use the smoothed joint and hand positions to calculate forearm rotation angles and classify gestures[cite: 391].
    * The final tracking data and gestures are rendered as overlays on the video feed in the `egui` UI[cite: 144].

---

## Building and Running

This project is configured for macOS, including a build script for creating a `.app` bundle.

### Prerequisites

1.  **Rust Toolchain**: Install via `rustup`.
2.  **Python 3**: Ensure `python3` is available in your PATH.
3.  **Python Dependencies**: Install the required computer vision libraries.
    ```bash
    pip3 install mediapipe opencv-python numpy
    ```
4.  **macOS Build Tools**: `Xcode Command Line Tools` are needed for the bundling script.

### Instructions

1.  **Clone the repository:**
    ```bash
    git clone <your-repo-url>
    cd Supro-Rewritten
    ```

2.  **Run in Debug Mode:**
    ```bash
    cargo run
    ```

3.  **Build a Release `.app` Bundle (macOS):**
    The `build.sh` script automates the process of bundling the application, copying the Python scripts, setting the correct `Info.plist` values, and moving the final app to the `/Applications` folder[cite: 530, 540, 549, 550].
    ```bash
    ./build.sh
    ```

---

## Project Structure

````

.
├── Cargo.toml              \# Rust project manifest and dependencies
├── build.sh                \# macOS application bundling script
├── assets/                 \# Icons and other static assets
├── python/
│   └── mediapipe\_service.py  \# The Python backend for MediaPipe processing
└── src/
├── main.rs               \# Application entry point and setup
├── app.rs                \# Core application struct, UI layout, and event handling
├── tracking.rs           \# All tracking logic, gesture algorithms, and Kalman filters
├── mediapipe\_bridge.rs   \# Manages communication with the Python subprocess
├── data.rs               \# Handles CSV and HTML data exporting
├── video.rs              \# Manages camera and video file sources via `nokhwa`
└── ui.rs                 \# Theming and custom UI components

```

---

## License

```