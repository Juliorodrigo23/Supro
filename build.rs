fn main() {
    // For now, we're not using PyO3 or OpenCV
    println!("cargo:rerun-if-changed=build.rs");
}