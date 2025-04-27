fn main() {
    cc::Build::new().file("activate_raw_mode.c").compile("activate_raw_mode");

    println!("cargo:rustc-link-lib=dylib=activate_raw_mode");
    println!("cargo:rustc-link-search=native=.");
}
