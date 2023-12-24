use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Listener host & port
    let lproto = env::var_os("LPROTO").unwrap();
    let lhost = env::var_os("LHOST").unwrap();
    let lport = env::var_os("LPORT").unwrap();

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("config.rs");

    fs::write(
        &dest_path,
        format!("pub fn config() -> (&'static str, &'static str, u16) {}
            (\"{}\", \"{}\", {})
        {}
        ", "{", lproto.into_string().unwrap(), lhost.into_string().unwrap(), lport.into_string().unwrap(), "}")
    ).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}