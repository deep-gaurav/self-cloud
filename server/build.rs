use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Get the current UTC timestamp
    let now = chrono::Utc::now();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", now.to_rfc3339());

    // Get git hash
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let git_hash = String::from_utf8(output.stdout).unwrap();
                println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
            } else {
                println!("cargo:rustc-env=GIT_HASH=unknown");
            }
        }
        Err(_) => {
            println!("cargo:rustc-env=GIT_HASH=unknown");
        }
    }
}
