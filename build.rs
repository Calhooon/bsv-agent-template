// Keep the keeper crate's capture, adapting the engine-free fleet's policy.
use std::process::Command;

fn main() {
    bsv_worker_build_info::emit();
    println!("cargo:rerun-if-changed=build.rs");

    // The keeper includes untracked files; fleet provenance counts tracked edits.
    let status = Command::new("git")
        .args([
            "--no-optional-locks",
            "status",
            "--porcelain",
            "--untracked-files=no",
        ])
        .output()
        .expect("git status for tracked build changes");
    assert!(status.status.success(), "build requires a git checkout");
    println!(
        "cargo:rustc-env=OVERLAY_DIRTY={}",
        u8::from(!status.stdout.is_empty())
    );

    // Watch every tracked input, including edits outside this Worker's src.
    let files = Command::new("git")
        .args(["ls-files", "--full-name"])
        .output()
        .expect("tracked build inputs");
    assert!(files.status.success());
    for file in String::from_utf8(files.stdout)
        .expect("UTF-8 paths")
        .lines()
    {
        println!("cargo:rerun-if-changed={file}");
    }
    println!("cargo:rerun-if-env-changed=BUILD_HOUR");
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("UTC build time")
        .as_secs();
    println!(
        "cargo:rustc-env=OVERLAY_BUILT_AT={}",
        bsv_worker_build_info::capture::rfc3339_utc(seconds / 3600 * 3600)
    );
}
