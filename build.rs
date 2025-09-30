use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let target_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    
    // Copy Info.plist to target directory
    let info_plist_src = Path::new(&manifest_dir).join("Info.plist");
    let info_plist_dst = Path::new(&target_dir).join("../../Info.plist");
    
    if info_plist_src.exists() {
        fs::copy(info_plist_src, info_plist_dst).expect("Failed to copy Info.plist");
    }
}