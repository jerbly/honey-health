use glob::glob;
use serde::Deserialize;
use std::{collections::HashMap, fmt::Display, fs::File, path::PathBuf};
use strsim::jaro;

#[derive(Debug)]
pub enum Suggestion {
    Matching,
    Missing,
    WrongCase,
    Similar(Vec<String>),
}

impl Display for Suggestion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Suggestion::Matching => write!(f, "Matching"),
            Suggestion::Missing => write!(f, "Missing"),
            Suggestion::WrongCase => write!(f, "WrongCase"),
            Suggestion::Similar(v) => write!(f, "Similar to {}", v.join(" ")),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Attribute {
    id: Option<String>,
    brief: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Group {
    prefix: Option<String>,
    attributes: Option<Vec<Attribute>>,
}

#[derive(Debug, Deserialize)]
struct Groups {
    groups: Vec<Group>,
}

#[derive(Debug)]
pub struct SemanticConventions {
    // Have a map of constructed-attribute-name as key, to, brief as value
    pub attribute_map: HashMap<String, String>,
}

impl SemanticConventions {
    pub fn new(root_dirs: &[String]) -> anyhow::Result<Self> {
        let mut sc = SemanticConventions {
            attribute_map: HashMap::new(),
        };
        for root_dir in root_dirs {
            let yml = format!("{root_dir}/**/*.yml");
            let yaml = format!("{root_dir}/**/*.yaml");
            for entry in glob(yml.as_str())?.chain(glob(yaml.as_str())?) {
                sc.read_file(entry?)?;
            }
        }
        Ok(sc)
    }

    pub fn read_file(&mut self, path: PathBuf) -> anyhow::Result<()> {
        //println!("{:?}", path.as_os_str());
        let groups: Groups = serde_yaml::from_reader(&File::open(path)?)?;
        for group in groups.groups {
            if let (Some(prefix), Some(attributes)) = (group.prefix, group.attributes) {
                for attribute in attributes {
                    if let (Some(id), Some(brief)) = (attribute.id, attribute.brief) {
                        self.attribute_map
                            .insert(format!("{}.{}", prefix, id), brief.trim().to_owned());
                    }
                }
            }
        }
        Ok(())
    }

    fn contains_uppercase(input: &str) -> bool {
        for c in input.chars() {
            if c.is_ascii_uppercase() {
                return true;
            }
        }
        false
    }

    fn similar(&self, input: &str) -> Option<Vec<String>> {
        // See if there are some obvious similarities
        let similars: Vec<String> = self
            .attribute_map
            .keys()
            .filter(|&key| jaro(input, key) > 0.85)
            .cloned()
            .collect();
        if !similars.is_empty() {
            Some(similars)
        } else {
            None
        }
    }

    /// Given the input attribute name, make an improvement suggestion.
    pub fn get_suggestion(&self, name: &str) -> Suggestion {
        // Is this already a semantic convention
        if self.attribute_map.contains_key(name) {
            Suggestion::Matching
        } else if Self::contains_uppercase(name) {
            Suggestion::WrongCase
        } else if let Some(similar_name) = self.similar(name) {
            Suggestion::Similar(similar_name)
        } else {
            Suggestion::Missing
        }
    }
}
