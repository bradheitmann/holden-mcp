//! Variant: escapes. The build script records the environment it was given,
//! so a test can show what reached a build that holden started.

fn main() {
    let mut vars: Vec<(String, String)> = std::env::vars().collect();
    vars.sort();
    let mut text = String::new();
    for (k, v) in vars {
        text.push_str(&k);
        text.push('=');
        text.push_str(&v);
        text.push('\n');
    }
    let out = std::env::var("OUT_DIR").unwrap_or_else(|_| ".".to_owned());
    if std::fs::write(std::path::Path::new(&out).join("build-env.txt"), text).is_err() {
        std::process::exit(1);
    }
}
