// this file is used to be able to use serde (https://github.com/serde-rs/serde) in rust stable
// cf serde readme for explanations

#[cfg(not(feature = "serde_macros"))]
mod inner {
    extern crate syntex;
    extern crate serde_codegen;

    use std::env;
    use std::path::Path;

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();

        let src = Path::new("src/objects.rs.in");
        let dst = Path::new(&out_dir).join("objects.rs");

        let mut registry = syntex::Registry::new();

        serde_codegen::register(&mut registry);
        registry.expand("", &src, &dst).unwrap();
    }
}

#[cfg(feature = "serde_macros")]
mod inner {
    pub fn main() {}
}

fn main() {
    inner::main();
}
