fn main() {
    println!("cargo:rerun-if-env-changed=QUICKTEXT_BUILD_ID");

    let build_id = std::env::var("QUICKTEXT_BUILD_ID")
        .ok()
        .filter(|build_id| !build_id.trim().is_empty())
        .unwrap_or_else(|| "Development".to_string());
    println!("cargo:rustc-env=QUICKTEXT_BUILD_ID={build_id}");

    tauri_build::build()
}
