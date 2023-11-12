mod honeycomb;
mod semconv;

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::{Context, Ok};
use chrono::Utc;
use clap::Parser;
use honeycomb::{Column, HoneyComb};
use semconv::{SemanticConventions, Suggestion};

// For each dataset get all the columns and put them in a map of column_name -> ColumnUsage
// ColumnUsage contains a column and a mapping of datasets where this column is used
#[derive(Debug)]
struct ColumnUsage {
    column: Column,
    datasets: Vec<bool>,
    suggestion: Suggestion,
}

impl ColumnUsage {
    fn new(
        column: Column,
        suggestion: Suggestion,
        dataset_len: usize,
        initial_true: usize,
    ) -> Self {
        let mut datasets = vec![false; dataset_len];
        datasets[initial_true] = true;
        Self {
            column,
            datasets,
            suggestion,
        }
    }

    fn datasets_as_string(&self) -> String {
        let mut bools = vec![];
        let mut total = 0usize;
        for d in &self.datasets {
            if *d {
                bools.push("x,");
                total += 1;
            } else {
                bools.push(",");
            }
        }
        format!("{},{}", total, bools.join(""))
    }
}

#[derive(Debug)]
struct ColumnUsageMap {
    map: HashMap<String, ColumnUsage>,
    datasets: Vec<String>,
    semconv: SemanticConventions,
}

impl ColumnUsageMap {
    fn new(
        root_dirs: &[String],
        include_datasets: Option<HashSet<String>>,
    ) -> anyhow::Result<Self> {
        let sc = SemanticConventions::new(root_dirs)?;

        let mut cm = ColumnUsageMap {
            map: HashMap::new(),
            datasets: vec![],
            semconv: sc,
        };
        let hc = HoneyComb::new();
        let now = Utc::now();
        let inc_datasets = match include_datasets {
            Some(d) => d,
            None => HashSet::new(),
        };
        let mut datasets = hc
            .list_all_datasets()?
            .iter()
            .filter_map(|d| {
                if (now - d.last_written_at).num_days() < 60 {
                    if inc_datasets.is_empty() || inc_datasets.contains(&d.slug) {
                        Some(d.slug.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        datasets.sort();
        cm.datasets = datasets;
        for (dataset_num, dataset_slug) in cm.datasets.iter().enumerate() {
            //println!("Reading dataset: {}", dataset_slug);
            let columns = hc.list_all_columns(dataset_slug)?;

            for column in columns {
                let duration = now - column.last_written;
                if duration.num_days() < 60 {
                    if let Some(cu) = cm.map.get_mut(&column.key_name) {
                        cu.datasets[dataset_num] = true;
                    } else {
                        let key_name = column.key_name.clone();
                        let suggestion = cm.semconv.get_suggestion(&key_name);
                        let cu =
                            ColumnUsage::new(column, suggestion, cm.datasets.len(), dataset_num);
                        cm.map.insert(key_name, cu);
                    }
                }
            }
        }
        Ok(cm)
    }

    fn to_csv(&self, path: &str) -> anyhow::Result<()> {
        let file = File::create(path)?;
        let mut file = BufWriter::new(file);
        let mut columns = self.map.values().collect::<Vec<_>>();
        columns.sort_by(|a, b| a.column.key_name.cmp(&b.column.key_name));
        writeln!(file, "Name,Type,SemConv,Usage,{},", self.datasets.join(","))?;
        for c in columns {
            writeln!(
                file,
                "{},{},{},{}",
                c.column.key_name,
                c.column.r#type,
                c.suggestion,
                c.datasets_as_string()
            )?;
        }
        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(author, version)]
/// Honey Health
///
/// Generates reports on the health of your Honeycomb datasets' attribute names.
/// Provide OpenTelemetry Semantic Convention compatible files to find mismatches
/// and suggestions.
struct Args {
    /// Model paths
    ///
    /// Provide one or more paths to the root of semantic convention
    /// model directories.
    #[arg(short, long, required = true, num_args(1..))]
    model: Vec<String>,

    /// Datasets
    ///
    /// Provide zero or more dataset names to limit the report, otherwise
    /// all datasets are included.
    #[arg(short, long, required = false, num_args(0..))]
    dataset: Option<Vec<String>>,

    /// Output file path
    #[arg(short, long, default_value_t = String::from("hh_report.csv"))]
    output: String,
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let args = Args::parse();
    let mut root_dirs = vec![];
    for path in args.model {
        let p = Path::new(&path);
        if !p.is_dir() {
            anyhow::bail!("{} is not directory", path);
        }
        root_dirs.push(
            p.canonicalize()?
                .to_str()
                .context("invalid path")?
                .to_owned(),
        );
    }
    let include_datasets = args.dataset.map(HashSet::from_iter);
    let cm = ColumnUsageMap::new(&root_dirs, include_datasets)?;
    cm.to_csv(&args.output)
}
