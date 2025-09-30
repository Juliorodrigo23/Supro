// src/main.rs
mod app;
mod tracking;
mod ui;
mod video;
mod data;

use eframe::egui;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Set up GUI options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([1200.0, 800.0])
            .with_icon(load_icon()),
        centered: true,
        ..Default::default()
    };

    // Run the application (don't use ? operator with eframe)
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

fn load_icon() -> egui::IconData {
    // Create a default icon if no icon file exists
    let icon_data = vec![255u8; 64 * 64 * 4]; // White 64x64 RGBA image
    
    egui::IconData {
        rgba: icon_data,
        width: 64,
        height: 64,
    }
}

fn configure_fonts(ctx: &egui::Context) {
    let fonts = egui::FontDefinitions::default();
    
    // Use the default fonts for now
    // You can add custom fonts later
    
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