use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if env::var_os("CARGO_FEATURE_RISCV").is_some() {
        let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set"));
        fs::write(out.join("embassy_rp_irq.x"), include_bytes!("embassy_rp_irq.x"))
            .expect("failed to write embassy_rp_irq.x");
        println!("cargo:rustc-link-search={}", out.display());
    }
    println!("cargo:rerun-if-changed=embassy_rp_irq.x");
}
