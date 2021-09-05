use std::process::Command;
fn main() {
    // does this need error checking?
    let output = Command::new("git").args(&["rev-parse", "HEAD"]).output().unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    let output = Command::new("git").args(&["status", "--porcelain"]).output().unwrap();
    let is_dirty = output.stdout.len() != 0;
    if is_dirty {
        println!("cargo:rustc-env=GIT_HASH={}+dirty", &git_hash[..7]);
    } else {
        println!("cargo:rustc-env=GIT_HASH={}", &git_hash[..7]);
    }
}
