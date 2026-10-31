//! The files a check reads: what git tracks, and anything new that git does not ignore.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

pub struct Repo {
    pub root: PathBuf,
    /// Paths relative to the root, with `/` separators, sorted.
    pub files: Vec<String>,
    /// Every directory that holds a file, as a relative path.
    pub dirs: BTreeSet<String>,
}

impl Repo {
    pub fn open(root: &Path) -> Result<Self> {
        let files = git_files(root).or_else(|_| walk(root))?;
        let mut dirs = BTreeSet::new();
        for f in &files {
            let mut p = Path::new(f).parent();
            while let Some(dir) = p.filter(|d| !d.as_os_str().is_empty()) {
                dirs.insert(slash(dir));
                p = dir.parent();
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            files,
            dirs,
        })
    }

    /// The text of a file, or `None` for a binary file or a file the working tree no longer has.
    pub fn read(&self, path: &str) -> Option<String> {
        let bytes = std::fs::read(self.root.join(path)).ok()?;
        if bytes.contains(&0) {
            return None;
        }
        String::from_utf8(bytes).ok()
    }

    pub fn has(&self, path: &str) -> bool {
        self.files
            .binary_search_by(|f| f.as_str().cmp(path))
            .is_ok()
            || self.dirs.contains(path)
    }
}

fn git_files(root: &Path) -> Result<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .output()
        .context("run git ls-files")?;
    if !out.status.success() {
        bail!(
            "git ls-files: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let mut files: Vec<String> = out
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .filter(|p| root.join(p).is_file())
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

/// The fallback outside a git work tree: every file except the `.git` directory.
fn walk(root: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
            let path = entry?.path();
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(rel) = path.strip_prefix(root) {
                files.push(slash(rel));
            }
        }
    }
    files.sort();
    Ok(files)
}

fn slash(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
