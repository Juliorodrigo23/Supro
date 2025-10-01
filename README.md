# Supro Arm Tracker

![Rust](https://img.shields.io/badge/language-Rust-orange)
![Python](https://img.shields.io/badge/backend-Python-blue)
![Platform](https://img.shields.io/badge/platform-macOS-lightgrey)
![Status](https://img.shields.io/badge/status-active-success)

**Supro Arm Tracker** is a real-time motion tracking system for analyzing forearm rotation (**supination** and **pronation**).  
Built in **Rust** with a **MediaPipe + Python backend**, it combines Kalman filtering and custom algorithms for smooth and accurate gesture recognition.  
Developed under Dr. Ortiz’s Research Lab by Julio Contreras.  

![Supro Logo](SuproLogo.gif)

---

## ✨ Features

- **Operation Modes**
  - 📹 **Live Camera Mode** – track via webcam in real-time
  - 🎞 **Video File Mode** – analyze pre-recorded sessions
  - 📊 **Analysis Mode** – review, chart, and export past results

- **Tracking & Visualization**
  - Real-time skeleton overlay (shoulders, elbows, wrists)
  - Hand & finger landmark tracking
  - Dual-view UI for raw vs tracked comparison

- **Gesture Analysis**
  - Detects **Supination** and **Pronation**
  - Displays live confidence scores & rotation angles

- **Data Export**
  - Save sessions to **CSV**
  - Auto-generate interactive **HTML reports**

---

## 📸 Demo




![Demo Screenshot](docs/demo.gif)


---

## 🚀 Quickstart

### Prerequisites

* **Rust toolchain** (`rustup`)
* **Python 3** with required dependencies:

  ```bash
  pip3 install mediapipe opencv-python numpy
  ```
* **macOS build tools**: Xcode Command Line Tools

### Clone & Run

```bash
git clone https://github.com/<your-repo>/supro-arm-tracker.git
cd supo-arm-tracker
cargo run
```

### Build macOS `.app` Bundle

Use the included build script:

```bash
./build.sh
```

This automates bundling, copies Python scripts, sets `Info.plist` values, and installs the app into `/Applications`.

---

## 🔧 How It Works

```
Rust (UI & Core Logic)
   │
   ├── Capture Frame → JSON → Python (MediaPipe)
   │
   └── Landmarks ← JSON ← MediaPipe
        │
        └── Kalman Filter + Gesture Algorithms → Rendered in egui UI
```

1. **UI & State (Rust)** – `eframe`/`egui` manages the app state and GUI.
2. **Python Service** – `mediapipe_service.py` processes frames with Google MediaPipe.
3. **Data Exchange** – Frames are serialized into JSON and sent over stdin/stdout.
4. **Analysis & Rendering (Rust)** – Landmarks are smoothed with Kalman filters, gestures classified, and results drawn on-screen.

---

## 📂 Project Structure

```bash
.
├── Cargo.toml              # Rust dependencies & project metadata
├── build.sh                # macOS bundling script
├── assets/                 # Icons and static assets
├── python/
│   └── mediapipe_service.py   # Python MediaPipe backend
└── src/
    ├── main.rs             # Application entry point
    ├── app.rs              # UI + application state
    ├── tracking.rs         # Gesture + Kalman filter logic
    ├── mediapipe_bridge.rs # IPC bridge to Python service
    ├── data.rs             # CSV + HTML data exporting
    ├── video.rs            # Camera + video input
    └── ui.rs               # Theming + custom UI components
```

---

## 🛠 Tech Stack

* **Rust** (core application logic & UI)
* **egui / eframe** (immediate-mode GUI)
* **MediaPipe** (pose & hand tracking, via Python)
* **Kalman Filters** (`nalgebra`) for smoothing joint positions
* **nokhwa** (camera capture)
* **serde / CSV / HTML** (data export)

---

## 🛣 Roadmap

* [ ] Windows & Linux builds
* [ ] Improved gesture classification (more than supination/pronation)
* [ ] Real-time performance metrics dashboard
* [ ] Mobile/embedded integration

---

## 🤝 Contributing

Pull requests are welcome! For major changes, please open an issue first to discuss what you’d like to change.

---

## 📄 License

> Proprietary research software. Contact author for usage permissions.
Julio Contreras | ECE @ Rutgers | JRC397@scarletmail.rutgers.edu