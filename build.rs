fn main() {
    slint_build::compile("ui/app.slint").expect("Failed to compile ui/app.slint");

    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        let wasi_sdk = std::env::var("WASI_SDK_PATH")
            .unwrap_or_else(|_| "C:/tools/wasi-sdk-34.0-x86_64-windows".to_string());
        println!(
            "cargo:rustc-link-search=native={}/share/wasi-sysroot/lib/wasm32-wasip1",
            wasi_sdk
        );
        println!("cargo:rustc-link-lib=static=m");
        println!("cargo:rustc-link-lib=static=c");
    }
}
