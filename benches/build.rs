fn main() {
    // used for documentation in readme generation
    println!("cargo:rustc-env=TARGET={}", std::env::var("TARGET").unwrap());
}
