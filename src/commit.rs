use log::error;
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::process::Command;
use std::str;

pub fn parse_commit<'a, P: AsRef<Path>>(raw_commit: &'a str, repo: P) -> Vec<&'a str> {
    let commits: Vec<&str> = raw_commit.split("..").collect();
    match commits.len() {
        1 => vec![raw_commit],
        2 => {
            if commits[0] == "" && commits[1] == "" {
                return vec![];
            }

            let (c1, c2) = sort_commits(commits[0], commits[1], repo);
            vec![c1, c2]
        }
        _ => {
            eprintln!("Commit format is invalid, {}", raw_commit);
            vec![]
        }
    }
}

pub fn sort_commits<'a, P: AsRef<Path>>(c1: &'a str, c2: &'a str, repo: P) -> (&'a str, &'a str) {
    match Command::new("git")
        .arg("-C")
        .arg(repo.as_ref())
        .arg("merge-base")
        .arg("--is-ancestor")
        .arg(c1)
        .arg(c2)
        .status()
    {
        Err(e) => {
            error!("git merge-base failed: {:?}", e);
            (c1, c2)
        }
        Ok(exit_status) => match exit_status.success() {
            true => (c1, c2),
            false => (c2, c1),
        },
    }
}

pub fn find_changes<P: AsRef<Path>>(raw_commit: &str, repo: P) -> HashSet<String> {
    let commits = parse_commit(raw_commit, &repo);
    let output = match commits.len() {
        1 => Command::new("git")
            .arg("-C")
            .arg(repo.as_ref())
            .arg("diff")
            .arg("--name-status")
            .arg(format!("{}~", commits[0]))
            .arg(commits[0])
            .output(),
        2 => Command::new("git")
            .arg("-C")
            .arg(repo.as_ref())
            .arg("diff")
            .arg("--name-status")
            .arg(commits[0])
            .arg(commits[1])
            .output(),
        _ => Err(Error::from(ErrorKind::NotFound)),
    };
    match output {
        Err(e) => {
            eprintln!("Error while find changes between commits: {:?}", e);
            HashSet::new()
        }
        Ok(output) => {
            match output.status.success() {
                true => {
                    let changes = str::from_utf8(&output.stdout).unwrap();
                    parse_changes(&changes)
                }
                false => {
                    eprintln!(
                        "git show failed, stdout:\n{}stderr:\n{}",
                        str::from_utf8(&output.stdout).unwrap(),
                        str::from_utf8(&output.stderr).unwrap(),
                    );
                    HashSet::new()
                }
            }
        }
    }
}

pub fn parse_changes(raw_changes: &str) -> HashSet<String> {
    raw_changes
        .lines()
        .filter_map(|line| {
            let l: Vec<_> = line.split_whitespace().collect();
            match l[0] {
                "D" => None,
                _ => Some(l[1].to_owned()),
            }
        })
        .collect()
}

