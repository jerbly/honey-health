use glob::glob;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::File,
    path::PathBuf,
};
use strsim::jaro;

#[derive(Debug, Clone, PartialEq)]
pub enum Suggestion {
    Matching,
    Missing(Vec<SuggestionComment>),
    Bad(Vec<SuggestionComment>),
}

impl Suggestion {
    pub fn get_name(&self) -> String {
        match self {
            Suggestion::Matching => "Matching".to_string(),
            Suggestion::Missing(_) => "Missing".to_string(),
            Suggestion::Bad(_) => "Bad".to_string(),
        }
    }
    pub fn get_comments_string(&self) -> String {
        match self {
            Suggestion::Matching => "".to_string(),
            Suggestion::Missing(comments) | Suggestion::Bad(comments) => comments
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join("; "),
        }
    }
}

impl Display for Suggestion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Suggestion::Matching => write!(f, "Matching"),
            _ => write!(f, "{:7}  {}", self.get_name(), self.get_comments_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionComment {
    WrongCase,
    Similar(Vec<String>),
    Extends(String),
    //DeepNamespace, // TODO if the namespace is deep, it could indicate encoding a code path
    NoNamespace,
}

impl Display for SuggestionComment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuggestionComment::WrongCase => write!(f, "WrongCase"),
            SuggestionComment::NoNamespace => write!(f, "NoNamespace"),
            SuggestionComment::Similar(v) => write!(f, "Similar to {}", v.join(" ")),
            SuggestionComment::Extends(s) => write!(f, "Extends {}", s),
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
    pub prefixes: HashSet<String>,
}

impl SemanticConventions {
    pub fn new(root_dirs: &[String]) -> anyhow::Result<Self> {
        let mut sc = SemanticConventions {
            attribute_map: HashMap::new(),
            prefixes: HashSet::new(),
        };
        sc.populate_builtins();
        for root_dir in root_dirs {
            let yml = format!("{root_dir}/**/*.yml");
            let yaml = format!("{root_dir}/**/*.yaml");
            for entry in glob(yml.as_str())?.chain(glob(yaml.as_str())?) {
                sc.read_file(entry?)?;
            }
        }
        Ok(sc)
    }

    fn populate_builtins(&mut self) {
        let builtins = [
            "duration_ms",
            "type",
            "meta.signal_type",
            "name",
            "span.kind",
            "span.num_events",
            "span.num_links",
            "trace.parent_id",
            "trace.span_id",
            "trace.trace_id",
            "meta.annotation_type",
            "parent_name",
            "status_code",
            "error",
        ];
        for builtin in builtins {
            self.attribute_map.insert(builtin.to_owned(), "".to_owned());
        }
        let builtin_prefixes = ["meta", "span", "trace"];
        for builtin_prefix in builtin_prefixes {
            self.prefixes.insert(builtin_prefix.to_owned());
        }
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
                        self.insert_prefixes(&prefix);
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

    // Break up the namespace a.b.c into a, a.b, a.b.c and store them in the set
    fn insert_prefixes(&mut self, input: &str) {
        let mut prefix = String::new();
        for c in input.chars() {
            if c == '.' {
                self.prefixes.insert(prefix.clone());
            }
            prefix.push(c);
        }
        self.prefixes.insert(prefix.clone());
    }

    // Split the input at the final dot, and see if the prefix exists
    fn prefix_exists(&self, input: &str) -> Option<String> {
        for (i, c) in input.chars().rev().enumerate() {
            if c == '.' {
                let s = input[..input.len() - (i + 1)].to_string();
                if self.prefixes.contains(&s) {
                    return Some(s);
                }
            }
        }
        None
    }

    fn has_namespace(&self, input: &str) -> bool {
        input.contains('.')
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
        } else {
            let mut bad = false;
            // get all the suggestion comments
            let mut comments = Vec::new();
            if Self::contains_uppercase(name) {
                comments.push(SuggestionComment::WrongCase);
                bad = true;
            }
            if let Some(s) = self.prefix_exists(name) {
                comments.push(SuggestionComment::Extends(s));
            }
            if !self.has_namespace(name) {
                comments.push(SuggestionComment::NoNamespace);
                bad = true;
            }
            if let Some(similar_names) = self.similar(name) {
                comments.push(SuggestionComment::Similar(similar_names));
            }
            if bad {
                Suggestion::Bad(comments)
            } else {
                Suggestion::Missing(comments)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_populate_builtins() {
        let mut sc = SemanticConventions {
            attribute_map: HashMap::new(),
            prefixes: HashSet::new(),
        };
        sc.populate_builtins();
        assert!(sc.attribute_map.contains_key("duration_ms"));
        assert!(sc.prefixes.contains("meta"));
    }

    #[test]
    fn test_contains_uppercase() {
        assert!(SemanticConventions::contains_uppercase("Test"));
        assert!(!SemanticConventions::contains_uppercase("test"));
    }

    #[test]
    fn test_insert_prefixes() {
        let mut sc = SemanticConventions {
            attribute_map: HashMap::new(),
            prefixes: HashSet::new(),
        };
        sc.insert_prefixes("a.b.c");
        assert!(sc.prefixes.contains("a"));
        assert!(sc.prefixes.contains("a.b"));
        assert!(sc.prefixes.contains("a.b.c"));
    }

    #[test]
    fn test_prefix_exists() {
        let mut sc = SemanticConventions {
            attribute_map: HashMap::new(),
            prefixes: HashSet::new(),
        };
        sc.insert_prefixes("a.b.c");
        assert_eq!(sc.prefix_exists("a.b.c.d"), Some("a.b.c".to_string()));
        assert_eq!(sc.prefix_exists("a.d"), Some("a".to_string()));
        assert_eq!(sc.prefix_exists("x.y.z"), None);
    }

    #[test]
    fn test_similar() {
        let mut sc = SemanticConventions {
            attribute_map: HashMap::new(),
            prefixes: HashSet::new(),
        };
        sc.attribute_map.insert("test".to_string(), "".to_string());
        assert_eq!(sc.similar("test"), Some(vec!["test".to_string()]));
        assert_eq!(sc.similar("x"), None);
    }
}
