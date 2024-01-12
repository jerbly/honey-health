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
use colored::Colorize;
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
struct DatasetHealth {
    matching: usize,
    missing: usize,
    bad: usize,
}

impl DatasetHealth {
    fn new() -> Self {
        Self {
            matching: 0,
            missing: 0,
            bad: 0,
        }
    }
}

#[derive(Debug)]
struct ColumnUsageMap {
    map: HashMap<String, ColumnUsage>,
    datasets: Vec<String>,
    dataset_health: Vec<DatasetHealth>,
    semconv: SemanticConventions,
}

impl ColumnUsageMap {
    fn new(
        root_dirs: &[String],
        include_datasets: Option<HashSet<String>>,
        max_last_written_days: usize,
    ) -> anyhow::Result<Self> {
        let sc = SemanticConventions::new(root_dirs)?;

        let mut cm = ColumnUsageMap {
            map: HashMap::new(),
            datasets: vec![],
            dataset_health: vec![],
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
                if (now - d.last_written_at.unwrap_or(Utc::now())).num_days()
                    < max_last_written_days as i64
                {
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
        eprint!("Reading {} datasets ", cm.datasets.len());
        for (dataset_num, dataset_slug) in cm.datasets.iter().enumerate() {
            //println!("Reading dataset: {}", dataset_slug);
            eprint!(".");
            let columns = hc.list_all_columns(dataset_slug)?;
            let mut dataset_health = DatasetHealth::new();
            for column in columns {
                let duration = now - column.last_written;
                if duration.num_days() < max_last_written_days as i64 {
                    let health: Suggestion;
                    if let Some(cu) = cm.map.get_mut(&column.key_name) {
                        cu.datasets[dataset_num] = true;
                        health = cu.suggestion.clone();
                    } else {
                        let key_name = column.key_name.clone();
                        let suggestion = cm.semconv.get_suggestion(&key_name);
                        let cu = ColumnUsage::new(
                            column,
                            suggestion.clone(),
                            cm.datasets.len(),
                            dataset_num,
                        );
                        cm.map.insert(key_name, cu);
                        health = suggestion;
                    }
                    match health {
                        Suggestion::Matching => dataset_health.matching += 1,
                        Suggestion::Missing(_) => dataset_health.missing += 1,
                        _ => dataset_health.bad += 1,
                    }
                }
            }
            cm.dataset_health.push(dataset_health);
        }
        eprintln!();
        Ok(cm)
    }

    fn to_csv(&self, path: &str) -> anyhow::Result<()> {
        let file = File::create(path)?;
        let mut file = BufWriter::new(file);
        let mut columns = self.map.values().collect::<Vec<_>>();
        columns.sort_by(|a, b| a.column.key_name.cmp(&b.column.key_name));
        writeln!(
            file,
            "Name,Type,SemConv,Hint,Usage,{},",
            self.datasets.join(",")
        )?;
        for c in columns {
            writeln!(
                file,
                "{},{},{},{},{}",
                c.column.key_name,
                c.column.r#type,
                c.suggestion.get_name(),
                c.suggestion.get_comments_string(),
                c.datasets_as_string()
            )?;
        }
        Ok(())
    }

    fn print_health(&self) {
        // find the length of the longest dataset name
        let longest = "Dataset".len().max(
            self.datasets
                .iter()
                .map(|dataset_slug| dataset_slug.len())
                .max()
                .unwrap_or(0),
        );

        println!(
            "{:>width$} {} {}  {}  {}",
            "Dataset".bold(),
            "Match".bold().green(),
            "Miss".bold().yellow(),
            "Bad".bold().red(),
            "Score".bold().blue(),
            width = longest
        );
        for (dataset_num, dataset_slug) in self.datasets.iter().enumerate() {
            let dataset_health = &self.dataset_health[dataset_num];
            let total = dataset_health.matching + dataset_health.missing + dataset_health.bad;
            let score = if total == 0 {
                0.0
            } else {
                (dataset_health.matching as f64 / total as f64) * 100.0
            };

            println!(
                "{:>width$}  {:4} {:4} {:4} {:>5.1}%",
                dataset_slug,
                dataset_health.matching,
                dataset_health.missing,
                dataset_health.bad,
                score,
                width = longest
            );
        }
    }

    fn print_dataset_report(&self) {
        // If there's only one dataset, print the columns that are not matching
        if self.datasets.len() == 1 {
            let mut columns = self.map.values().collect::<Vec<_>>();
            let longest = "Column".len().max(
                columns
                    .iter()
                    .map(|c| c.column.key_name.len())
                    .max()
                    .unwrap_or(0),
            );
            columns.sort_by(|a, b| a.column.key_name.cmp(&b.column.key_name));
            println!(
                "\n{:>width$} {}",
                "Column".bold(),
                "Suggestion".bold(),
                width = longest
            );
            for c in columns {
                match c.suggestion {
                    Suggestion::Matching => {}
                    Suggestion::Missing(_) => {
                        println!(
                            "{:>width$} {}",
                            c.column.key_name.yellow(),
                            c.suggestion,
                            width = longest
                        );
                    }
                    _ => {
                        println!(
                            "{:>width$} {}",
                            c.column.key_name.red(),
                            c.suggestion,
                            width = longest
                        );
                    }
                }
            }
        }
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
    /// Provide zero or more dataset names to limit the report. Omitting this
    /// means all datasets are included. A single dataset will print a report
    /// rather than a CSV file.
    #[arg(short, long, required = false, num_args(0..))]
    dataset: Option<Vec<String>>,

    /// Output file path
    ///
    /// Provide a path to the CSV dataset comparison report. This is only
    /// used when more than one dataset is included.
    #[arg(short, long, default_value_t = String::from("hh_report.csv"))]
    output: String,

    /// Max last written days
    ///
    /// The maximum number of days since a dataset was last written to. This
    /// defaults to 30 days.
    #[arg(short, long, default_value_t = 30)]
    last_written_days: usize,
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
    let cm = ColumnUsageMap::new(&root_dirs, include_datasets, args.last_written_days)?;
    if cm.datasets.is_empty() {
        println!("No datasets found");
        return Ok(());
    }
    if cm.datasets.len() > 1 {
        cm.to_csv(&args.output)?;
    }
    cm.print_health();
    cm.print_dataset_report();
    Ok(())
}
