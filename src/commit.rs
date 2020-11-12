use log::error;
use std::char;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

pub struct Commits {
    raw_commits: Vec<String>,
    repo: PathBuf,
}

impl Commits {
    pub fn new(raw_commits: Vec<String>, repo: PathBuf) -> Self {
        Commits { raw_commits, repo }
    }

    pub fn changes(&self) -> Vec<String> {
        self.raw_commits
            .iter()
            .map(|raw_commit| find_changes(raw_commit, &self.repo))
            .fold(HashSet::new(), |acc, changes| {
                acc.union(&changes).map(|c| c.to_owned()).collect()
            })
            .into_iter()
            .collect()
    }
}

pub fn parse_commit<'a, P: AsRef<Path>>(raw_commit: &'a str, repo: P) -> (&'a str, &'a str) {
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
    let mut changes: HashSet<String> = HashSet::new();
    for commit in commits {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo.as_ref())
            .args(&["show", "--name-status", "--pretty=tformat:", &commit])
            .output();
        let files = match output {
            Err(e) => {
                eprintln!("Error while find changes between commits: {:?}", e);
                vec![]
            }
            Ok(output) => {
                if output.status.success() {
                    // println!("output: {}", str::from_utf8(&output.stdout).unwrap());
                    str::from_utf8(&output.stdout)
                        .unwrap()
                        .lines()
                        .filter_map(|line| {
                            // println!("line: {}", line);
                            let l: Vec<_> = line.split(char::is_whitespace).collect();
                            // println!("line split: {:?}", l);
                            match l[0] {
                                "D" => None,
                                _ => Some(l[1].to_owned()),
                            }
                        })
                        .collect()
                } else {
                    eprintln!(
                        "git show failed, stdout:\n{}stderr:\n{}",
                        str::from_utf8(&output.stdout).unwrap(),
                        str::from_utf8(&output.stderr).unwrap(),
                    );
                    vec![]
                }
            }
        };
        changes.extend(files);
    }
    changes
}
