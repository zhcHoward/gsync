use crate::error::{ErrorKind, GsyncError};
use regex::Regex;
use serde::{Deserialize, Deserializer};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(deserialize_with = "from_list")]
    pub dir_map: Vec<(Regex, PathBuf)>,
    pub ignored: Vec<String>,
}

impl Config {
    pub fn parse_config<P: AsRef<Path>>(path: P) -> Result<Self, GsyncError> {
        match path.as_ref().exists() {
            true => {
                let contents = fs::read_to_string(path.as_ref()).unwrap();
                Ok(serde_json::from_str(&contents).unwrap())
            }
            false => {
                eprintln!(
                    "Config file {} does not exist",
                    path.as_ref().to_string_lossy()
                );
                Err(GsyncError::Custom(ErrorKind::ConfigNotExist))
            }
        }
    }
}

fn from_list<'de, D>(deserializer: D) -> Result<Vec<(Regex, PathBuf)>, D::Error>
where
    D: Deserializer<'de>,
{
    let dir_map: Vec<(String, PathBuf)> = Deserialize::deserialize(deserializer)?;
    Ok(dir_map
        .into_iter()
        .map(|map| (Regex::new(&map.0).unwrap(), map.1))
        .collect())
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json;
    use std::cmp::PartialEq;

    impl PartialEq for Config {
        fn eq(&self, other: &Config) -> bool {
            self.ignored == other.ignored
        }
    }

    #[test]
    fn test_deserialize_config() {
        let json = r#"
            {
                "dir_map": [
                    ["aaa/bbb", "/usr/local/bin/aaa/bbb"]
                ],
                "ignored": [
                    "ccc/ddd"
                ]
            }
        "#;
        let config: Config = serde_json::from_str(json).unwrap();
        let expected = Config {
            dir_map: vec![(
                Regex::new("aaa/bbb").unwrap(),
                PathBuf::from("/usr/local/bin/aaa/bbb"),
            )],
            ignored: vec![String::from("ccc/ddd")],
        };
        assert_eq!(config, expected);
    }
}
