use crate::error::{ErrorKind, GsyncError};
use regex::Regex;
use serde::{Deserialize, Deserializer};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(deserialize_with = "to_dirmap")]
    pub dir_map: Vec<(Regex, PathBuf)>,
    #[serde(deserialize_with = "to_ignored")]
    pub ignored: Vec<Regex>,
}

impl Config {
    pub fn parse_config<P: AsRef<Path>>(path: P) -> Result<Self, GsyncError> {
        match path.as_ref().exists() {
            true => {
                let contents = fs::read_to_string(path.as_ref())?;
                let config = serde_json::from_str(&contents)?;
                Ok(config)
            }
            false => {
                eprintln!(
                    "Config file {} does not exist",
                    path.as_ref().to_string_lossy()
                );
                Err(ErrorKind::ConfigNotExist.into())
            }
        }
    }
}

fn to_dirmap<'de, D>(deserializer: D) -> Result<Vec<(Regex, PathBuf)>, D::Error>
where
    D: Deserializer<'de>,
{
    let dir_map: Vec<(String, PathBuf)> = Deserialize::deserialize(deserializer)?;
    Ok(dir_map
        .into_iter()
        .map(|map| (Regex::new(&map.0).unwrap(), map.1))
        .collect())
}

fn to_ignored<'de, D>(deserializer: D) -> Result<Vec<Regex>, D::Error>
where
    D: Deserializer<'de>,
{
    let ignored: Vec<String> = Deserialize::deserialize(deserializer)?;
    Ok(ignored
        .into_iter()
        .map(|i| Regex::new(&i).unwrap())
        .collect())
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json;
    use std::cmp::PartialEq;

    impl PartialEq for Config {
        fn eq(&self, other: &Config) -> bool {
            for (index, (regex1, path1)) in self.dir_map.iter().enumerate() {
                let (regex2, path2) = &other.dir_map[index];
                if regex1.as_str() != regex2.as_str() || path1 != path2 {
                    return false;
                }
            }

            for (index, reg1) in self.ignored.iter().enumerate() {
                let reg2 = &other.ignored[index];
                if reg1.as_str() != reg2.as_str() {
                    return false;
                }
            }

            return true;
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
            ignored: vec![Regex::new("ccc/ddd").unwrap()],
        };
        assert_eq!(config, expected);
    }
}
