use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

fn main() {
    println!("Testing camera access...\n");

    let index = CameraIndex::Index(0);
    let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);

    match Camera::new(index, format) {
        Ok(mut camera) => {
            println!("✓ Camera opened");
            
            match camera.open_stream() {
                Ok(_) => {
                    println!("✓ Stream opened - CAMERA ACCESS WORKING!");
                    match camera.frame() {
                        Ok(_) => println!("✓ Frame captured successfully"),
                        Err(e) => println!("✗ Failed to capture frame: {}", e),
                    }
                }
                Err(e) => println!("✗ Failed to open stream: {}", e),
            }
        }
        Err(e) => {
            println!("✗ Failed to open camera: {}", e);
            println!("\nPossible causes:");
            println!("1. Camera is being used by another app");
            println!("2. Camera permissions not granted");
            println!("3. No camera connected");
        }
    }
}
