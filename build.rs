fn main() {
    println!("cargo:rustc-check-cfg=cfg(no_global_oom_handling)");
}
