fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon_with_id("assets/icon.ico", "1");
        res.set("ProductName", "WhatsApp Browser");
        res.set(
            "FileDescription",
            "WhatsApp Browser - Multi-aba com cookies independentes",
        );
        res.compile().expect("falha ao compilar recursos Windows");
    }
}
