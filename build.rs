fn main() {
    // Link required macOS frameworks for notify-rust's mac-notification-sys dependency
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
}
