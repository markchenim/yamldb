#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use yamldb::{QueryOp, Record, YamlDb};

    #[test]
    fn test_create_and_read() {
        let mut db = YamlDb::memory();
        let mut record = Record::new("user1");
        record.set("name", "Alice").set("age", 30);
        db.create(record).unwrap();

        let record = db.read("user1").unwrap();
        assert_eq!(record.get_str("name"), Some("Alice"));
        assert_eq!(record.get_i64("age"), Some(30));
    }

    #[test]
    fn test_update_field() {
        let mut db = YamlDb::memory();
        let mut record = Record::new("user1");
        record.set("name", "Alice");
        db.create(record).unwrap();

        db.update_field("user1", "name", serde_yaml::Value::String("Bob".to_string())).unwrap();
        let record = db.read("user1").unwrap();
        assert_eq!(record.get_str("name"), Some("Bob"));
    }

    #[test]
    fn test_delete() {
        let mut db = YamlDb::memory();
        let record = Record::new("user1");
        db.create(record).unwrap();
        db.delete("user1").unwrap();
        assert!(!db.exists("user1"));
    }

    #[test]
    fn test_query_eq() {
        let mut db = YamlDb::memory();
        let mut record = Record::new("user1");
        record.set("city", "Beijing");
        db.create(record).unwrap();

        let result = db.query(&QueryOp::eq("city", "Beijing"));
        assert_eq!(result.count(), 1);
    }

    #[test]
    fn test_query_and() {
        let mut db = YamlDb::memory();
        let mut r1 = Record::new("user1");
        r1.set("city", "Beijing").set("age", 30);
        db.create(r1).unwrap();

        let mut r2 = Record::new("user2");
        r2.set("city", "Beijing").set("age", 25);
        db.create(r2).unwrap();

        let result = db.query(&QueryOp::and(vec![
            QueryOp::eq("city", "Beijing"),
            QueryOp::gte("age", serde_yaml::Value::Number(28.into())),
        ]));
        assert_eq!(result.count(), 1);
    }

    #[test]
    fn test_upsert() {
        let mut db = YamlDb::memory();
        let mut record = Record::new("user1");
        record.set("name", "Alice");
        db.upsert(record).unwrap();

        let mut record = Record::new("user1");
        record.set("name", "Bob");
        db.upsert(record).unwrap();

        let record = db.read("user1").unwrap();
        assert_eq!(record.get_str("name"), Some("Bob"));
    }

    #[test]
    fn test_count_and_exists() {
        let mut db = YamlDb::memory();
        let record = Record::new("user1");
        db.create(record).unwrap();

        assert_eq!(db.count(), 1);
        assert!(db.exists("user1"));
        assert!(!db.exists("user2"));
    }
}
