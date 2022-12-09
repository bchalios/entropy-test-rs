use std::{path::PathBuf, str::FromStr};

extern crate bindgen;

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = bindgen::Builder::default()
        .header("random.h")
        .allowlist_var("RNDGETENTCNT")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.gB
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the src/bindings.rs file.
    let out_path = PathBuf::from_str("src/bindings.rs").unwrap();
    bindings
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
}
