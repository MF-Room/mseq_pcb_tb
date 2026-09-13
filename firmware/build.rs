fn main() {
    println!(
        "cargo:rustc-link-search={}",
        std::env::var("CARGO_MANIFEST_DIR").unwrap()
    );
    println!("cargo:rerun-if-env-changed=MODE");
    println!("cargo:rerun-if-env-changed=COUNT");
    println!("cargo::rustc-check-cfg=cfg(receive2_mode)");
    println!("cargo::rustc-check-cfg=cfg(send_mode)");
    println!("cargo::rustc-check-cfg=cfg(display_mode)");
    println!("cargo::rustc-check-cfg=cfg(bpm_mode)");
    println!("cargo::rustc-check-cfg=cfg(nor_mode)");
    println!("cargo::rustc-check-cfg=cfg(fram_mode)");
    println!("cargo::rustc-check-cfg=cfg(switch_mode)");
    if std::env::var("MODE").as_deref() == Ok("receive2") {
        println!("cargo:rustc-cfg=receive2_mode");
    }
    if std::env::var("MODE").as_deref() == Ok("send") {
        println!("cargo:rustc-cfg=send_mode");
    }
    if std::env::var("MODE").as_deref() == Ok("display") {
        println!("cargo:rustc-cfg=display_mode");
    }
    if std::env::var("MODE").as_deref() == Ok("bpm") {
        println!("cargo:rustc-cfg=bpm_mode");
    }
    if std::env::var("MODE").as_deref() == Ok("nor") {
        println!("cargo:rustc-cfg=nor_mode");
    }
    if std::env::var("MODE").as_deref() == Ok("fram") {
        println!("cargo:rustc-cfg=fram_mode");
    }
    if std::env::var("MODE").as_deref() == Ok("switch") {
        println!("cargo:rustc-cfg=switch_mode");
    }
}
