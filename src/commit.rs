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
    pub fn new<P: AsRef<Path>>(raw_commits: Vec<String>, repo: P) -> Self {
        Commits {
            raw_commits,
            repo: repo.as_ref().to_path_buf(),
        }
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

pub fn parse_commit<P: AsRef<Path>>(raw_commit: &str, repo: P) -> Vec<String> {
    let commits: Vec<&str> = raw_commit.split("..").collect();
    match commits.len() {
        1 => vec![raw_commit.to_owned()],
        2 => {
            if commits[0] == "" && commits[1] == "" {
                return vec![];
            }

            let output = Command::new("git")
                .arg("-C")
                .arg(repo.as_ref())
                .args(&["rev-list", raw_commit])
                .output()
                .unwrap();
            str::from_utf8(&output.stdout)
                .unwrap()
                .lines()
                .map(|line| line.to_string())
                .collect()
        }
        _ => {
            eprintln!("Commit format is invalid, {}", raw_commit);
            vec![]
        }
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
