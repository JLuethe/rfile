fn main() {
    println!("cargo:rerun-if-env-changed=RFILE_MAGIC_PATH");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,librfile_magic.so.1");
    }
}
