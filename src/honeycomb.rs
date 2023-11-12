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
    pub last_written_at: DateTime<Utc>,
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
        let response = reqwest::blocking::Client::new()
            .get(format!("{}datasets", URL))
            .header("X-Honeycomb-Team", &self.api_key)
            .send()?
            .json::<Vec<Dataset>>()?;
        Ok(response)
    }
    pub fn list_all_columns(&self, dataset_slug: &str) -> anyhow::Result<Vec<Column>> {
        let response = reqwest::blocking::Client::new()
            .get(format!("{}columns/{}", URL, dataset_slug))
            .header("X-Honeycomb-Team", &self.api_key)
            .send()?
            .json::<Vec<Column>>()?;
        Ok(response)
    }
}
