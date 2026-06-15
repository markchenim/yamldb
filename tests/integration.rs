#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use yamldb::{Record, YamlDb};

    #[test]
    fn test_create_and_read() {
        let mut db = YamlDb::memory();
        let mut data = HashMap::new();
        data.insert("name".to_string(), serde_yaml::Value::String("Alice".to_string()));
        db.create(Record { id: "1".to_string(), data }).unwrap();
        let record = db.read("1").unwrap();
        assert_eq!(record.data.get("name").unwrap().as_str().unwrap(), "Alice");
    }

    #[test]
    fn test_update() {
        let mut db = YamlDb::memory();
        let mut data = HashMap::new();
        data.insert("name".to_string(), serde_yaml::Value::String("Alice".to_string()));
        db.create(Record { id: "1".to_string(), data }).unwrap();

        let mut new_data = HashMap::new();
        new_data.insert("name".to_string(), serde_yaml::Value::String("Bob".to_string()));
        db.update("1", new_data).unwrap();

        let record = db.read("1").unwrap();
        assert_eq!(record.data.get("name").unwrap().as_str().unwrap(), "Bob");
    }

    #[test]
    fn test_delete() {
        let mut db = YamlDb::memory();
        let data = HashMap::new();
        db.create(Record { id: "1".to_string(), data }).unwrap();
        db.delete("1").unwrap();
        assert!(db.read("1").is_err());
    }

    #[test]
    fn test_query() {
        let mut db = YamlDb::memory();
        let mut data = HashMap::new();
        data.insert("city".to_string(), serde_yaml::Value::String("Beijing".to_string()));
        db.create(Record { id: "1".to_string(), data }).unwrap();

        let results = db.query(|r| {
            r.data.get("city").and_then(|v| v.as_str()).map(|c| c == "Beijing").unwrap_or(false)
        });
        assert_eq!(results.len(), 1);
    }
}
