// src/main.rs
mod app;
mod tracking;
mod ui;
mod video;
mod data;

use eframe::egui;
use usvg::TreeParsing;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    if let Ok(p) = std::env::current_exe() {
        eprintln!("Running from: {}", p.display());
    }

    // DEBUG: List all available cameras
    println!("=== Camera Detection Debug ===");
    match nokhwa::query(nokhwa::utils::ApiBackend::Auto) {
        Ok(cameras) => {
            println!("Found {} camera(s):", cameras.len());
            for (i, camera) in cameras.iter().enumerate() {
                println!("  [{}] {}", i, camera.human_name());
            }
        }
        Err(e) => {
            println!("Failed to query cameras: {}", e);
        }
    }
    println!("============================\n");



    // Set up GUI options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([1200.0, 800.0]),
        centered: true,
        ..Default::default()
    };

    // Run the application
    let result = eframe::run_native(
        "Arm Rotation Tracking System",
        options,
        Box::new(|cc| {
            // Configure fonts and visuals
            configure_fonts(&cc.egui_ctx);
            cc.egui_ctx.set_visuals(create_visuals());
            
            Box::new(app::ArmTrackerApp::new(cc))
        }),
    );

    // Handle the error if needed
    if let Err(e) = result {
        eprintln!("Error running application: {:?}", e);
    }
}



fn load_svg_as_rgba(path: &str, size: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let svg_data = std::fs::read_to_string(path)?;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg_data, &opt)?;
    
    // Use resvg's re-exported tiny_skia types
    let pixmap_size = tree.size.to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).unwrap();
    
    let scale = size as f32 / pixmap_size.width().max(pixmap_size.height()) as f32;
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    
    // Use the Tree's render method directly with consistent types
    resvg::Tree::from_usvg(&tree).render(transform, &mut pixmap.as_mut());
    
    Ok(pixmap.data().to_vec())
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    
    // Load Montserrat font
    let font_path = "/Users/JulioContreras/Desktop/School/Research/Baseball SuPro /SuPro Rewritten/fonts/Montserrat-VariableFont_wght.ttf";
    if let Ok(font_data) = std::fs::read(font_path) {
        fonts.font_data.insert(
            "Montserrat".to_owned(),
            egui::FontData::from_owned(font_data),
        );
        
        // Set Montserrat as the primary font
        fonts.families.entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Montserrat".to_owned());
            
        fonts.families.entry(egui::FontFamily::Monospace)
            .or_default()
            .push("Montserrat".to_owned());
    }
    
    ctx.set_fonts(fonts);
}

fn create_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    
    // Customize colors for a modern, professional look
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 35);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 45, 52);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(55, 55, 65);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(70, 130, 240);
    
    // Adjust rounding for modern appearance
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    visuals.widgets.active.rounding = egui::Rounding::same(8.0);
    
    visuals.window_rounding = egui::Rounding::same(12.0);
    visuals.menu_rounding = egui::Rounding::same(8.0);
    
    visuals
}