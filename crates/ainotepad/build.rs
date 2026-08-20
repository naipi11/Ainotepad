fn main() {
    let icon = std::path::Path::new("assets/ainotepad.ico");
    if icon.exists() {
        println!("cargo:rerun-if-changed=assets/ainotepad.ico");
        let _ = embed_resource::compile("assets/ainotepad-icon.rc", embed_resource::NONE);
    }
}
