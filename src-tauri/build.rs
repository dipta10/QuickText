fn main() {
    println!("cargo:rerun-if-env-changed=QUICKTEXT_BUILD_ID");
    println!("cargo:rerun-if-env-changed=QUICKTEXT_SOURCE_REVISION");

    let build_id = std::env::var("QUICKTEXT_BUILD_ID")
        .ok()
        .filter(|build_id| !build_id.trim().is_empty())
        .unwrap_or_else(|| "Development".to_string());
    let source_revision = std::env::var("QUICKTEXT_SOURCE_REVISION")
        .ok()
        .filter(|source_revision| !source_revision.trim().is_empty())
        .unwrap_or_else(|| "Development".to_string());
    println!("cargo:rustc-env=QUICKTEXT_BUILD_ID={build_id}");
    println!("cargo:rustc-env=QUICKTEXT_SOURCE_REVISION={source_revision}");

    tauri_build::build()
}
