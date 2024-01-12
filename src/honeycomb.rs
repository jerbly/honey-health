use std::env;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub struct HoneyComb {
    pub api_key: String,
}
const URL: &str = "https://api.honeycomb.io/1/";
const HONEYCOMB_API_KEY: &str = "HONEYCOMB_API_KEY";

#[derive(Debug, Deserialize)]
pub struct Dataset {
    pub slug: String,
    pub last_written_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Column {
    pub id: String,
    pub key_name: String,
    pub r#type: String,
    pub description: String,
    pub hidden: bool,
    pub last_written: DateTime<Utc>,
}

impl HoneyComb {
    pub fn new() -> Self {
        Self {
            api_key: env::var(HONEYCOMB_API_KEY)
                .unwrap_or_else(|_| panic!("Environment variable {} not found", HONEYCOMB_API_KEY)),
        }
    }
    pub fn list_all_datasets(&self) -> anyhow::Result<Vec<Dataset>> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(format!("{}datasets", URL))
            .header("X-Honeycomb-Team", &self.api_key)
            .send()?;

        let text = response.text()?;

        match serde_json::from_str::<Vec<Dataset>>(&text) {
            Ok(datasets) => Ok(datasets),
            Err(_) => {
                println!("Invalid JSON data: {}", text);
                Err(anyhow::anyhow!("Invalid JSON data"))
            }
        }
    }
    pub fn list_all_columns(&self, dataset_slug: &str) -> anyhow::Result<Vec<Column>> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(format!("{}columns/{}", URL, dataset_slug))
            .header("X-Honeycomb-Team", &self.api_key)
            .send()?;

        let text = response.text()?;

        match serde_json::from_str::<Vec<Column>>(&text) {
            Ok(columns) => Ok(columns),
            Err(_) => {
                println!("Invalid JSON data: {}", text);
                Err(anyhow::anyhow!("Invalid JSON data"))
            }
        }
    }
}
