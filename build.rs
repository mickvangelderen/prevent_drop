use std::env;

fn main() {
    println!("rerun-if-changed=\"Cargo.toml\"");
    println!("rerun-if-env-changed=\"OPT_LEVEL\"");

    let opt_level = env::var("OPT_LEVEL").unwrap();

    match opt_level.parse::<u32>() {
        Ok(opt_level) => {
            if opt_level > 0 {
                println!("cargo:rustc-cfg=opt_level_gt_0");
            }
        }
        _ => {}
    }
}
