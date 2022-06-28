use log::error;
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::process::Command;
use std::str;

pub fn parse_commit<'a>(raw_commit: &'a str) -> Vec<&'a str> {
    let commits: Vec<&str> = raw_commit.split("..").collect();
    match commits.len() {
        1 => vec![raw_commit],
        2 => {
            if commits[0] == "" && commits[1] == "" {
                return vec![];
            }

            let (c1, c2) = sort_commits(commits[0], commits[1]);
            vec![c1, c2]
        }
        _ => {
            eprintln!("Commit format is invalid, {}", raw_commit);
            vec![]
        }
    }
}

pub fn sort_commits<'a>(c1: &'a str, c2: &'a str) -> (&'a str, &'a str) {
    match Command::new("git")
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

pub fn find_changes(raw_commit: &str) -> HashSet<String> {
    let commits = parse_commit(raw_commit);
    let output = match commits.len() {
        1 => Command::new("git")
            .arg("diff")
            .arg("--name-status")
            .arg(format!("{}~", commits[0]))
            .arg(commits[0])
            .output(),
        2 => Command::new("git")
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
        Ok(output) => match output.status.success() {
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
        },
    }
}

pub fn parse_changes(raw_changes: &str) -> HashSet<String> {
    raw_changes
        .lines()
        .filter_map(|line| {
            let parts: Vec<_> = line.split_whitespace().collect();
            match parts[0] {
                "D" => None,
                "R100" => None, // only rename files, no content change, ignore
                s if s.starts_with('R') => Some(parts[2].to_owned()), // contains content change, need to sync
                _ => Some(parts[1].to_owned()),
            }
        })
        .collect()
}
