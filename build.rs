use std::{fs, path::Path, process::Command};

fn git_directory() -> Option<std::path::PathBuf> {
  let directory = Path::new(".git");

  if directory.is_dir() {
    return Some(directory.to_path_buf());
  }

  let content = fs::read_to_string(directory).ok()?;
  let path = Path::new(content.trim().strip_prefix("gitdir: ")?);

  if path.is_absolute() {
    Some(path.to_path_buf())
  } else {
    Some(directory.parent()?.join(path))
  }
}

fn git_sha() -> Option<String> {
  let output = Command::new("git").args(["rev-parse", "HEAD"]).output().ok()?;

  if !output.status.success() {
    return None;
  }

  let sha = String::from_utf8(output.stdout).ok()?.trim().to_string();

  if sha.is_empty() { None } else { Some(sha) }
}

fn main() {
  if let Some(git_dir) = git_directory() {
    let head_path = git_dir.join("HEAD");

    println!("cargo:rerun-if-changed={}", head_path.display());

    if let Ok(head_contents) = fs::read_to_string(&head_path) {
      if let Some(reference) = head_contents.trim().strip_prefix("ref: ") {
        println!(
          "cargo:rerun-if-changed={}",
          git_dir.join(reference).display()
        );
      }
    }
  }

  let sha = git_sha().unwrap_or_else(|| "UNKNOWN".to_string());

  println!("cargo:rustc-env=VERGEN_GIT_SHA={sha}");
}
