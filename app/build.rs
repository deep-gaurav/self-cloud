use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Get the current UTC timestamp
    // We can use chrono if available in build-dependencies, or just use shell date.
    // To avoid adding build deps (since I don't see [build-dependencies] in app Cargo.toml yet),
    // I will try to use `date` command or just `SystemTime`.

    // Using SystemTime
    let now = std::time::SystemTime::now();
    let dt: chrono::DateTime<chrono::Utc> = now.into();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", dt.to_rfc3339());

    // note: add error checking yourself.
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
}
