fn main() {
    let icon = std::path::Path::new("assets/aitext.ico");
    if icon.exists() {
        println!("cargo:rerun-if-changed=assets/aitext.ico");
        let _ = embed_resource::compile("assets/aitext-icon.rc", embed_resource::NONE);
    }
}
