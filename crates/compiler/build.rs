fn main() {
    // Rendering assets are browser-owned. Cargo must not synthesize an atlas or
    // read the separate assets checkout while compiling the Rust application.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../web/assets/models");
    println!("cargo:warning=Direct GLB rendering enabled; no image atlas is generated");
}
