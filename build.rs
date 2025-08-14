fn main() {
    // This build script is for linking to libxdo
    // It's a C library for simulating keyboard input
    
    // Use pkg-config to find libxdo
    if let Err(e) = pkg_config::probe_library("libxdo") {
        // If pkg-config fails, print a helpful error message
        eprintln!("Failed to find libxdo using pkg-config: {}", e);
        eprintln!("Please ensure libxdo is installed and configured correctly.");
        eprintln!("On Debian/Ubuntu, you can install it with: sudo apt-get install libxdo-dev");
        std::process::exit(1);
    }
    
    println!("cargo:rustc-link-lib=xdo");
}