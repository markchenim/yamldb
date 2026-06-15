use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum YamlDbError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Record not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    #[serde(flatten)]
    pub data: HashMap<String, serde_yaml::Value>,
}

pub struct YamlDb {
    path: Option<PathBuf>,
    records: HashMap<String, Record>,
}

impl YamlDb {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: Some(path.as_ref().to_path_buf()),
            records: HashMap::new(),
        }
    }

    pub fn memory() -> Self {
        Self {
            path: None,
            records: HashMap::new(),
        }
    }

    pub fn load(&mut self) -> Result<(), YamlDbError> {
        let path = match &self.path {
            Some(p) => p,
            None => return Ok(()),
        };
        if !path.exists() {
            self.records = HashMap::new();
            return Ok(());
        }
        let content = fs::read_to_string(path)?;
        if content.trim().is_empty() {
            self.records = HashMap::new();
            return Ok(());
        }
        let records: Vec<Record> = serde_yaml::from_str(&content)?;
        self.records = records.into_iter().map(|r| (r.id.clone(), r)).collect();
        Ok(())
    }

    pub fn save(&self) -> Result<(), YamlDbError> {
        let path = match &self.path {
            Some(p) => p,
            None => return Ok(()),
        };
        let records: Vec<&Record> = self.records.values().collect();
        let content = serde_yaml::to_string(&records)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn create(&mut self, record: Record) -> Result<(), YamlDbError> {
        if self.records.contains_key(&record.id) {
            return Err(YamlDbError::NotFound(format!(
                "Record '{}' already exists",
                record.id
            )));
        }
        self.records.insert(record.id.clone(), record);
        self.save()
    }

    pub fn read(&self, id: &str) -> Result<&Record, YamlDbError> {
        self.records
            .get(id)
            .ok_or_else(|| YamlDbError::NotFound(id.to_string()))
    }

    pub fn read_all(&self) -> Vec<&Record> {
        self.records.values().collect()
    }

    pub fn update(&mut self, id: &str, data: HashMap<String, serde_yaml::Value>) -> Result<(), YamlDbError> {
        let record = self
            .records
            .get_mut(id)
            .ok_or_else(|| YamlDbError::NotFound(id.to_string()))?;
        record.data = data;
        self.save()
    }

    pub fn delete(&mut self, id: &str) -> Result<(), YamlDbError> {
        self.records
            .remove(id)
            .ok_or_else(|| YamlDbError::NotFound(id.to_string()))?;
        self.save()
    }

    pub fn query<F>(&self, filter: F) -> Vec<&Record>
    where
        F: Fn(&Record) -> bool,
    {
        self.records.values().filter(|r| filter(r)).collect()
    }
}
