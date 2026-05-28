fn main() {
    println!("cargo:rustc-link-search={}", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:rerun-if-env-changed=MODE");
    println!("cargo::rustc-check-cfg=cfg(send_mode)");
    if std::env::var("MODE").as_deref() == Ok("send") {
        println!("cargo:rustc-cfg=send_mode");
    }
}
