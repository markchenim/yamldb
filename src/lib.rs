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
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Record not found: {0}")]
    NotFound(String),
    #[error("Duplicate key: {0}")]
    DuplicateKey(String),
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    #[serde(flatten)]
    pub data: HashMap<String, serde_yaml::Value>,
}

impl Record {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            data: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<serde_yaml::Value>) -> &mut Self {
        self.data.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&serde_yaml::Value> {
        self.data.get(key)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.data.get(key).and_then(|v| v.as_str())
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.data.get(key).and_then(|v| v.as_i64())
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.data.get(key).and_then(|v| v.as_f64())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.data.get(key).and_then(|v| v.as_bool())
    }
}

#[derive(Debug, Clone)]
pub enum QueryOp {
    Eq(String, serde_yaml::Value),
    Ne(String, serde_yaml::Value),
    Gt(String, serde_yaml::Value),
    Lt(String, serde_yaml::Value),
    Gte(String, serde_yaml::Value),
    Lte(String, serde_yaml::Value),
    Contains(String, String),
    And(Vec<QueryOp>),
    Or(Vec<QueryOp>),
}

impl QueryOp {
    pub fn eq(key: impl Into<String>, value: impl Into<serde_yaml::Value>) -> Self {
        Self::Eq(key.into(), value.into())
    }

    pub fn ne(key: impl Into<String>, value: impl Into<serde_yaml::Value>) -> Self {
        Self::Ne(key.into(), value.into())
    }

    pub fn gt(key: impl Into<String>, value: impl Into<serde_yaml::Value>) -> Self {
        Self::Gt(key.into(), value.into())
    }

    pub fn lt(key: impl Into<String>, value: impl Into<serde_yaml::Value>) -> Self {
        Self::Lt(key.into(), value.into())
    }

    pub fn gte(key: impl Into<String>, value: impl Into<serde_yaml::Value>) -> Self {
        Self::Gte(key.into(), value.into())
    }

    pub fn lte(key: impl Into<String>, value: impl Into<serde_yaml::Value>) -> Self {
        Self::Lte(key.into(), value.into())
    }

    pub fn contains(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Contains(key.into(), value.into())
    }

    pub fn and(ops: Vec<QueryOp>) -> Self {
        Self::And(ops)
    }

    pub fn or(ops: Vec<QueryOp>) -> Self {
        Self::Or(ops)
    }

    fn matches(&self, record: &Record) -> bool {
        match self {
            QueryOp::Eq(key, value) => record.data.get(key).map(|v| v == value).unwrap_or(false),
            QueryOp::Ne(key, value) => record.data.get(key).map(|v| v != value).unwrap_or(true),
            QueryOp::Gt(key, value) => compare_values(record.data.get(key), value, std::cmp::Ordering::Greater),
            QueryOp::Lt(key, value) => compare_values(record.data.get(key), value, std::cmp::Ordering::Less),
            QueryOp::Gte(key, value) => {
                compare_values(record.data.get(key), value, std::cmp::Ordering::Greater)
                    || record.data.get(key).map(|v| v == value).unwrap_or(false)
            }
            QueryOp::Lte(key, value) => {
                compare_values(record.data.get(key), value, std::cmp::Ordering::Less)
                    || record.data.get(key).map(|v| v == value).unwrap_or(false)
            }
            QueryOp::Contains(key, substr) => record
                .data
                .get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.contains(substr.as_str()))
                .unwrap_or(false),
            QueryOp::And(ops) => ops.iter().all(|op| op.matches(record)),
            QueryOp::Or(ops) => ops.iter().any(|op| op.matches(record)),
        }
    }
}

fn compare_values(
    record_val: Option<&serde_yaml::Value>,
    query_val: &serde_yaml::Value,
    ordering: std::cmp::Ordering,
) -> bool {
    match (record_val, query_val) {
        (Some(serde_yaml::Value::Number(n1)), serde_yaml::Value::Number(n2)) => {
            if let (Some(a), Some(b)) = (n1.as_i64(), n2.as_i64()) {
                a.cmp(&b) == ordering
            } else if let (Some(a), Some(b)) = (n1.as_f64(), n2.as_f64()) {
                a.partial_cmp(&b).map(|o| o == ordering).unwrap_or(false)
            } else {
                false
            }
        }
        (Some(serde_yaml::Value::String(s1)), serde_yaml::Value::String(s2)) => {
            s1.cmp(s2) == ordering
        }
        _ => false,
    }
}

#[derive(Debug)]
pub struct QueryResult<'a> {
    records: Vec<&'a Record>,
}

impl<'a> QueryResult<'a> {
    pub fn first(&self) -> Option<&'a Record> {
        self.records.first().copied()
    }

    pub fn count(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn sort_by_key(&mut self, key: &str, ascending: bool) {
        self.records.sort_by(|a, b| {
            let cmp = a.data.get(key).partial_cmp(&b.data.get(key));
            if ascending {
                cmp.unwrap_or(std::cmp::Ordering::Equal)
            } else {
                cmp.unwrap_or(std::cmp::Ordering::Equal).reverse()
            }
        });
    }

    pub fn limit(&self, n: usize) -> Vec<&'a Record> {
        self.records.iter().take(n).copied().collect()
    }

    pub fn to_vec(&self) -> Vec<&'a Record> {
        self.records.clone()
    }

    pub fn iter(&self) -> impl Iterator<Item = &&'a Record> {
        self.records.iter()
    }
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
            return Err(YamlDbError::DuplicateKey(record.id));
        }
        self.records.insert(record.id.clone(), record);
        self.save()
    }

    pub fn insert(&mut self, record: Record) -> Result<(), YamlDbError> {
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

    pub fn update_field(&mut self, id: &str, key: &str, value: serde_yaml::Value) -> Result<(), YamlDbError> {
        let record = self
            .records
            .get_mut(id)
            .ok_or_else(|| YamlDbError::NotFound(id.to_string()))?;
        record.data.insert(key.to_string(), value);
        self.save()
    }

    pub fn delete(&mut self, id: &str) -> Result<(), YamlDbError> {
        self.records
            .remove(id)
            .ok_or_else(|| YamlDbError::NotFound(id.to_string()))?;
        self.save()
    }

    pub fn delete_many(&mut self, ids: &[&str]) -> Result<usize, YamlDbError> {
        let mut count = 0;
        for id in ids {
            if self.records.remove(*id).is_some() {
                count += 1;
            }
        }
        if count > 0 {
            self.save()?;
        }
        Ok(count)
    }

    pub fn query(&self, op: &QueryOp) -> QueryResult<'_> {
        let records: Vec<&Record> = self.records.values().filter(|r| op.matches(r)).collect();
        QueryResult { records }
    }

    pub fn find_where<F>(&self, filter: F) -> QueryResult<'_>
    where
        F: Fn(&Record) -> bool,
    {
        let records: Vec<&Record> = self.records.values().filter(|r| filter(r)).collect();
        QueryResult { records }
    }

    pub fn count(&self) -> usize {
        self.records.len()
    }

    pub fn exists(&self, id: &str) -> bool {
        self.records.contains_key(id)
    }

    pub fn clear(&mut self) -> Result<(), YamlDbError> {
        self.records.clear();
        self.save()
    }

    pub fn upsert(&mut self, record: Record) -> Result<(), YamlDbError> {
        self.records.insert(record.id.clone(), record);
        self.save()
    }

    pub fn import_json(&mut self, path: &Path) -> Result<usize, YamlDbError> {
        let content = fs::read_to_string(path)?;
        let items: Vec<serde_json::Value> = serde_json::from_str(&content)?;
        let mut count = 0;
        for item in items {
            if let Some(obj) = item.as_object() {
                let id = obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or(YamlDbError::InvalidQuery("Missing 'id' field".to_string()))?
                    .to_string();
                let data: HashMap<String, serde_yaml::Value> = obj
                    .iter()
                    .filter(|(k, _)| *k != "id")
                    .map(|(k, v)| {
                        let yaml_val: serde_yaml::Value =
                            serde_yaml::to_value(v).unwrap_or(serde_yaml::Value::Null);
                        (k.clone(), yaml_val)
                    })
                    .collect();
                self.records.insert(id.clone(), Record { id, data });
                count += 1;
            }
        }
        self.save()?;
        Ok(count)
    }

    pub fn import_yaml(&mut self, path: &Path) -> Result<usize, YamlDbError> {
        let content = fs::read_to_string(path)?;
        let records: Vec<Record> = serde_yaml::from_str(&content)?;
        let count = records.len();
        for record in records {
            self.records.insert(record.id.clone(), record);
        }
        self.save()?;
        Ok(count)
    }

    pub fn export_json(&self, path: &Path) -> Result<(), YamlDbError> {
        let records: Vec<&Record> = self.records.values().collect();
        let content = serde_json::to_string_pretty(&records)?;
        fs::write(path, content)?;
        Ok(())
    }
}
