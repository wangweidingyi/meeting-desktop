fn main() {
    #[cfg(target_os = "macos")]
    {
        cc::Build::new()
            .cpp(true)
            .file("native/macos/system_audio_bridge.mm")
            .flag("-std=c++17")
            .flag("-fobjc-arc")
            .flag("-Wno-deprecated-declarations")
            .compile("meeting_system_audio_bridge");

        println!("cargo:rerun-if-changed=native/macos/system_audio_bridge.h");
        println!("cargo:rerun-if-changed=native/macos/system_audio_bridge.mm");
        println!("cargo:rustc-link-lib=framework=CoreAudio");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }

    tauri_build::build()
}
