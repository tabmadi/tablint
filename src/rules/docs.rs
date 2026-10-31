//! The layout of the docs/ set and the links between files, per ADR-0001.

use std::collections::BTreeMap;
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::Ctx;
use crate::config::Paths;
use crate::finding::Finding;

pub fn check(ctx: &Ctx<'_>) -> Result<Vec<Finding>> {
    let mut out = layout(ctx);
    out.extend(links(ctx)?);
    out.extend(tasks(ctx)?);
    Ok(out)
}

fn layout(ctx: &Ctx<'_>) -> Vec<Finding> {
    let d = &ctx.cfg.docs;
    let mut out = Vec::new();
    let in_docs: Vec<&str> = ctx
        .docs
        .keys()
        .copied()
        .filter(|p| p.starts_with("docs/"))
        .collect();
    for &path in &in_docs {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() > 2
            && !d.genre_dirs.is_empty()
            && !d.genre_dirs.iter().any(|g| g == parts[1])
        {
            out.push(Finding::new(
                path,
                0,
                "docs/genre",
                format!("`docs/{}` is not a genre directory", parts[1]),
            ));
        }
        if parts.len() == 2
            && !d.root_files.is_empty()
            && !d.root_files.iter().any(|r| r == parts[1])
        {
            out.push(Finding::new(
                path,
                0,
                "docs/root",
                "the docs/ root holds only the entry documents and the registries",
            ));
        }
    }
    let mut names: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for &path in &in_docs {
        let name = path.rsplit('/').next().unwrap_or(path);
        if !d.shared_names.iter().any(|s| s == name) {
            names.entry(name).or_default().push(path);
        }
    }
    for (name, paths) in names.into_iter().filter(|(_, p)| p.len() > 1) {
        for path in paths {
            out.push(Finding::new(
                path,
                0,
                "docs/filename",
                format!("another document under docs/ is also `{name}`"),
            ));
        }
    }
    for dir in ctx.repo.dirs.iter().filter(|d| d.starts_with("docs/")) {
        let files = ctx
            .repo
            .files
            .iter()
            .filter(|f| f.rsplit_once('/').is_some_and(|(p, _)| p == dir))
            .count();
        let subdirs = ctx
            .repo
            .dirs
            .iter()
            .filter(|s| s.rsplit_once('/').is_some_and(|(p, _)| p == dir))
            .count();
        if files == 1 && subdirs == 0 {
            out.push(Finding::new(
                dir,
                0,
                "docs/single-file-dir",
                "a directory that holds a single file groups nothing",
            ));
        }
    }
    out
}

fn links(ctx: &Ctx<'_>) -> Result<Vec<Finding>> {
    let forbidden: Vec<(Paths, &str, &str)> = ctx
        .cfg
        .docs
        .forbidden_links
        .iter()
        .map(|f| {
            Ok((
                Paths::new(std::slice::from_ref(&f.from))?,
                f.to.as_str(),
                f.reason.as_str(),
            ))
        })
        .collect::<Result<_>>()?;
    let mut out = Vec::new();
    for (&path, doc) in ctx.docs.iter().filter(|(p, _)| !ctx.skipped.matches(p)) {
        for l in doc
            .links
            .iter()
            .filter(|l| !l.resolved.is_empty() && !l.resolved.starts_with(".."))
        {
            if !ctx.repo.has(&l.resolved) {
                out.push(Finding::new(
                    path,
                    l.line,
                    "docs/untracked-link",
                    format!("`{}` is not a file git tracks", l.target),
                ));
            }
            for (from, to, reason) in &forbidden {
                if from.matches(path) && l.resolved == *to {
                    out.push(Finding::new(
                        path,
                        l.line,
                        "docs/forbidden-link",
                        format!("links to `{to}`: {reason}"),
                    ));
                }
            }
        }
    }
    Ok(out)
}

/// A `(CI: <task>)` annotation names a task, never the workflow file that calls it.
fn tasks(ctx: &Ctx<'_>) -> Result<Vec<Finding>> {
    let kinds = &ctx.cfg.annotations.task_kinds;
    let named: Vec<(&str, usize, &str)> = ctx
        .docs
        .iter()
        .filter(|(p, _)| !ctx.skipped.matches(p))
        .flat_map(|(&path, doc)| doc.annotations.iter().map(move |a| (path, a)))
        .filter(|(_, a)| kinds.contains(&a.kind))
        .flat_map(|(path, a)| a.values.iter().map(move |v| (path, a.line, v.as_str())))
        .filter(|(_, _, v)| *v != "<task>")
        .collect();
    if named.is_empty() || !ctx.cfg.enabled("annotation/task") {
        return Ok(Vec::new());
    }
    let tasks = mise_tasks(&ctx.repo.root)?;
    Ok(named
        .into_iter()
        .filter(|(_, _, task)| !tasks.iter().any(|t| t == task))
        .map(|(path, line, task)| {
            Finding::new(
                path,
                line,
                "annotation/task",
                format!("`{task}` is not a mise task"),
            )
        })
        .collect())
}

fn mise_tasks(root: &std::path::Path) -> Result<Vec<String>> {
    let out = Command::new("mise")
        .args(["tasks", "ls", "--json"])
        .current_dir(root)
        .output()
        .context("run mise tasks ls")?;
    if !out.status.success() {
        bail!(
            "mise tasks ls: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let tasks: Vec<serde_json::Value> =
        serde_json::from_slice(&out.stdout).context("parse mise tasks ls")?;
    Ok(tasks
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_owned))
        .collect())
}
