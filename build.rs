fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let _= embed_resource::compile("build_resources/windows/build.rc", embed_resource::NONE);
    }
}