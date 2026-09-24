fn main() {
    println!("cargo:rerun-if-changed=src/text_input.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/text_input.m")
            .flag("-fobjc-arc")
            .compile("keygen_text_input");
    }
}
