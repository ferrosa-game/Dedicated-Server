use std::process::Command;

fn get_git_hash() {
        let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    println!("cargo::rustc-env=GIT_HASH={}", git_hash);
    println!("cargo::rerun-if-changed=.git/HEAD");
}

fn main() {
    get_git_hash()
}
