# YamlDB

A lightweight YAML file-based database with CLI support.

## Features

- YAML file as data storage
- CRUD operations
- Query builder with conditions
- Fuzzy search
- Import/Export (JSON, YAML)
- Backup/Restore
- Database statistics
- CLI tool

## Installation

```bash
cargo install yamldb
```

Or download from [Releases](https://github.com/markchenim/yamldb/releases).

## CLI Usage

### Basic Commands

```bash
# Create record
yamldb create user1 --fields name=Alice,age=30,city=Beijing

# Get record
yamldb get user1
yamldb get user1 --format json

# List all records
yamldb list
yamldb list --limit 10
yamldb list --format json

# Update record
yamldb update user1 --fields age=31,city=Guangzhou

# Delete record
yamldb delete user1
```

### Query Commands

```bash
# Query with conditions
yamldb query --key city --value Beijing
yamldb query --key age --value 25 --op gt
yamldb query --key name --value Ali --op contains

# Fuzzy search
yamldb search --keyword alice
yamldb search --keyword alice --key name
```

### Import/Export

```bash
# Import from file
yamldb import -i users.json
yamldb import -i users.yaml

# Export to file
yamldb export -o backup.json
yamldb export -o backup.yaml --format yaml

# Backup database
yamldb backup -o backup.yaml
```

### Utility Commands

```bash
# Show statistics
yamldb stats

# Count records
yamldb count

# Check if record exists
yamldb exists user1

# Clear database (requires --force)
yamldb clear --force
```

### Global Options

```
-f, --file <FILE>  Specify YAML file path [default: data.yaml]
```

## Rust API

### Basic Usage

```rust
use yamldb::{Record, YamlDb};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = YamlDb::new("data.yaml");
    db.load()?;

    // Create record
    let mut record = Record::new("user1");
    record.set("name", "Alice").set("age", 30);
    db.create(record)?;

    // Read record
    let record = db.read("user1")?;
    println!("Name: {:?}", record.get_str("name"));

    // Update record
    db.update_field("user1", "age", serde_yaml::Value::Number(31.into()))?;

    // Delete record
    db.delete("user1")?;

    Ok(())
}
```

### Query Builder

```rust
use yamldb::{QueryOp, YamlDb};

let db = YamlDb::new("data.yaml");
db.load()?;

// Equal
let result = db.query(&QueryOp::eq("city", "Beijing"));

// Comparison
let result = db.query(&QueryOp::gt("age", serde_yaml::Value::Number(25.into())));

// String operations
let result = db.query(&QueryOp::contains("name", "Ali"));
let result = db.query(&QueryOp::starts_with("name", "Ali"));
let result = db.query(&QueryOp::ends_with("name", "Smith"));

// AND condition
let result = db.query(&QueryOp::and(vec![
    QueryOp::eq("city", "Beijing"),
    QueryOp::gte("age", serde_yaml::Value::Number(28.into())),
]));

// OR condition
let result = db.query(&QueryOp::or(vec![
    QueryOp::eq("city", "Beijing"),
    QueryOp::eq("city", "Shanghai"),
]));

// NOT condition
let result = db.query(&QueryOp::not(QueryOp::eq("city", "Beijing")));
```

### Search

```rust
// Search in specific field
let result = db.search("name", "alice");

// Search in all fields
let result = db.search_all("alice");
```

### QueryResult

```rust
let result = db.query(&QueryOp::eq("city", "Beijing"));

// Get count
println!("Count: {}", result.count());

// Get first/last record
if let Some(record) = result.first() {
    println!("First: {}", record.id);
}

// Pagination
let page = result.page(1, 10); // page 1, 10 items per page

// Iterate
for record in result.iter() {
    println!("{}: {:?}", record.id, record.data);
}
```

### Statistics

```rust
let stats = db.stats();
println!("Total records: {}", stats.total_records);
println!("Unique keys: {:?}", stats.unique_keys);
```

### Backup

```rust
db.backup(Path::new("backup.yaml"))?;
```

## Import Format

**JSON:**
```json
[
  {"id": "user1", "name": "Alice", "age": 30},
  {"id": "user2", "name": "Bob", "age": 25}
]
```

**YAML:**
```yaml
- id: user1
  name: Alice
  age: 30
- id: user2
  name: Bob
  age: 25
```

## Error Handling

```rust
use yamldb::{YamlDb, YamlDbError};

match db.create(record) {
    Ok(_) => println!("Success"),
    Err(YamlDbError::DuplicateKey(id)) => println!("Duplicate: {}", id),
    Err(YamlDbError::NotFound(id)) => println!("Not found: {}", id),
    Err(e) => println!("Error: {}", e),
}
```

## License

MIT
