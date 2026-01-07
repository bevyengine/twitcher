use std::{fs, path::Path};

use git2::{Repository, Sort};
use regex::Regex;
use serde::Serialize;
use tera::Tera;
use twitcher::stats::find_stats_files;

#[derive(Serialize)]
enum Status {
    Unknown,
    Done,
    Queued,
}

#[derive(Serialize)]
struct Commit {
    id: String,
    summary: String,
    pr: u16,
    timestamp: i64,
    status: Status,
}

fn main() {
    let repo = match Repository::open("bevy") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };

    let commits_done: Vec<String> = find_stats_files(Path::new("results"))
        .iter()
        .map(|path| {
            path.parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();
    let commits_queued: Vec<String> = fs::read_dir("queue")
        .unwrap()
        .filter_map(|f| f.ok())
        .filter(|entry| entry.file_type().unwrap().is_file())
        .map(|entry| entry.file_name().to_str().unwrap().to_string())
        .collect();

    let summary_regex = Regex::new("(.*) \\(#([0-9]+)\\)").unwrap();

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.set_sorting(Sort::TIME).unwrap();
    revwalk.push_head().unwrap();
    let commits = revwalk
        .filter_map(|c| repo.find_commit(*c.as_ref().unwrap()).ok())
        .take(500)
        .flat_map(|commit| {
            let captures = summary_regex.captures(commit.summary().unwrap())?;
            let id = commit.id().to_string();
            Some(Commit {
                status: if commits_done.contains(&id) {
                    Status::Done
                } else if commits_queued.contains(&id) {
                    Status::Queued
                } else {
                    Status::Unknown
                },
                id,
                timestamp: commit.time().seconds(),
                summary: captures.get(1).unwrap().as_str().to_string(),
                pr: captures.get(2).unwrap().as_str().parse().unwrap(),
            })
        })
        .collect::<Vec<_>>();

    let tera = Tera::new("templates/*").unwrap();
    // Prepare the context with some data
    let mut context = tera::Context::new();
    context.insert("commits", &commits);
    context.insert(
        "missing",
        &(commits
            .iter()
            .filter(|c| !matches!(c.status, Status::Done))
            .count()),
    );
    context.insert("updated", &(chrono::Utc::now().timestamp()));

    // Render the template with the given context
    let rendered = tera.render("landing-page.html", &context).unwrap();
    std::fs::write("./index.html", &rendered).unwrap();
}
