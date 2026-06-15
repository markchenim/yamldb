# YamlDB

[![CI](https://github.com/markchenim/yamldb/actions/workflows/ci.yml/badge.svg)](https://github.com/markchenim/yamldb/actions/workflows/ci.yml)
[![Release](https://github.com/markchenim/yamldb/actions/workflows/release.yml/badge.svg)](https://github.com/markchenim/yamldb/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A lightweight YAML file-based database with CLI tool, Rust API, and ODBC driver support.

## Features

- **YAML Storage** - Human-readable data format
- **CRUD Operations** - Create, Read, Update, Delete
- **Query Builder** - Flexible conditions with AND/OR/NOT
- **Fuzzy Search** - Case-insensitive keyword search
- **Import/Export** - JSON and YAML formats
- **Backup/Restore** - Database snapshots
- **CLI Tool** - Command-line interface
- **ODBC Driver** - SQL query support

## Installation

### From Cargo

```bash
cargo install yamldb
```

### From Releases

Download pre-built binaries from [GitHub Releases](https://github.com/markchenim/yamldb/releases):

| Platform | File |
|----------|------|
| Linux | `yamldb-linux` |
| Windows | `yamldb-windows.exe` |
| macOS | `yamldb-macos` |

## CLI Usage

### Global Options

```
-f, --file <FILE>  Specify YAML file path [default: data.yaml]
```

### Record Operations

```bash
# Create
yamldb create user1 --fields name=Alice,age=30,city=Beijing

# Read
yamldb get user1
yamldb get user1 --format json

# List
yamldb list
yamldb list --limit 10
yamldb list --format json

# Update
yamldb update user1 --fields age=31,city=Guangzhou

# Delete
yamldb delete user1
```

### Query & Search

```bash
# Equality
yamldb query --key city --value Beijing

# Comparison operators
yamldb query --key age --value 25 --op gt      # > 25
yamldb query --key age --value 30 --op gte     # >= 30
yamldb query --key age --value 50 --op lt      # < 50
yamldb query --key age --value 25 --op lte     # <= 25
yamldb query --key city --value Beijing --op ne  # != Beijing

# String operations
yamldb query --key name --value Ali --op contains

# Fuzzy search
yamldb search --keyword alice
yamldb search --keyword alice --key name
```

### Import & Export

```bash
# Import
yamldb import -i users.json
yamldb import -i users.yaml

# Export
yamldb export -o backup.json
yamldb export -o backup.yaml --format yaml

# Backup
yamldb backup -o backup.yaml
```

### Utility Commands

```bash
# Statistics
yamldb stats

# Count records
yamldb count

# Check existence
yamldb exists user1

# Clear database
yamldb clear --force
```

## Rust API

### Basic Usage

```rust
use yamldb::{Record, YamlDb};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open database
    let mut db = YamlDb::new("data.yaml");
    db.load()?;

    // Create record
    let mut record = Record::new("user1");
    record.set("name", "Alice")
          .set("age", 30)
          .set("city", "Beijing");
    db.create(record)?;

    // Read record
    let record = db.read("user1")?;
    println!("Name: {:?}", record.get_str("name"));
    println!("Age: {:?}", record.get_i64("age"));

    // Update single field
    db.update_field("user1", "age", serde_yaml::Value::Number(31.into()))?;

    // Delete record
    db.delete("user1")?;

    Ok(())
}
```

### Memory Database

```rust
// In-memory database (no file I/O)
let mut db = YamlDb::memory();
let mut record = Record::new("test");
record.set("key", "value");
db.create(record)?;
```

### Record API

```rust
let mut record = Record::new("user1");

// Set values (chainable)
record.set("name", "Alice")
      .set("age", 30)
      .set("active", true);

// Get typed values
record.get_str("name");    // Some("Alice")
record.get_i64("age");     // Some(30)
record.get_f64("score");   // None
record.get_bool("active"); // Some(true)

// Check keys
record.has_key("name");  // true
record.keys();           // ["name", "age", "active"]

// Merge records
let mut other = Record::new("user1");
other.set("email", "alice@example.com");
record.merge(&other);

// Convert to JSON
let json = record.to_json()?;
```

### Query Builder

```rust
use yamldb::{QueryOp, YamlDb};

let db = YamlDb::new("data.yaml");
db.load()?;

// Comparison operators
let result = db.query(&QueryOp::eq("city", "Beijing"));
let result = db.query(&QueryOp::ne("city", "Shanghai"));
let result = db.query(&QueryOp::gt("age", serde_yaml::Value::Number(25.into())));
let result = db.query(&QueryOp::gte("age", serde_yaml::Value::Number(30.into())));
let result = db.query(&QueryOp::lt("age", serde_yaml::Value::Number(50.into())));
let result = db.query(&QueryOp::lte("age", serde_yaml::Value::Number(25.into())));

// String operations
let result = db.query(&QueryOp::contains("name", "Ali"));
let result = db.query(&QueryOp::starts_with("name", "Ali"));
let result = db.query(&QueryOp::ends_with("name", "Smith"));

// Logical operators
let result = db.query(&QueryOp::and(vec![
    QueryOp::eq("city", "Beijing"),
    QueryOp::gte("age", serde_yaml::Value::Number(28.into())),
]));

let result = db.query(&QueryOp::or(vec![
    QueryOp::eq("city", "Beijing"),
    QueryOp::eq("city", "Shanghai"),
]));

let result = db.query(&QueryOp::negate(QueryOp::eq("city", "Beijing")));
```

### Search

```rust
// Search in specific field (case-insensitive)
let result = db.search("name", "alice");

// Search in all fields
let result = db.search_all("alice");
```

### QueryResult

```rust
let result = db.query(&QueryOp::eq("city", "Beijing"));

// Count
println!("Found: {}", result.count());
println!("Is empty: {}", result.is_empty());

// Access records
if let Some(first) = result.first() {
    println!("First: {}", first.id);
}
if let Some(last) = result.last() {
    println!("Last: {}", last.id);
}

// Pagination
let page1 = result.page(1, 10);  // Page 1, 10 items per page
let page2 = result.page(2, 10);  // Page 2

// Skip and limit
let skipped = result.skip(5);
let limited = result.limit(10);

// Get all IDs
let ids = result.ids();

// Iterate
for record in result.iter() {
    println!("{}: {:?}", record.id, record.data);
}

// Convert to Vec
let all = result.to_vec();
```

### Batch Operations

```rust
// Read multiple records
let records = db.read_many(&["user1", "user2", "user3"]);

// Update multiple records
let updates = vec![
    ("user1".to_string(), HashMap::from([("age".to_string(), serde_yaml::Value::Number(31.into()))])),
    ("user2".to_string(), HashMap::from([("age".to_string(), serde_yaml::Value::Number(26.into()))])),
];
let updated = db.update_many(updates)?;

// Delete multiple records
let deleted = db.delete_many(&["user1", "user2"])?;

// Upsert (insert or update)
let mut record = Record::new("user1");
record.set("name", "Alice");
db.upsert(record)?;
```

### Statistics & Backup

```rust
use std::path::Path;

// Get statistics
let stats = db.stats();
println!("Total records: {}", stats.total_records);
println!("Unique keys: {:?}", stats.unique_keys);
println!("File size: {:?} bytes", stats.file_size);

// Backup
db.backup(Path::new("backup.yaml"))?;

// Clear all records
db.clear()?;
```

### Import/Export

```rust
use std::path::Path;

// Import from JSON
let count = db.import_json(Path::new("users.json"))?;
println!("Imported {} records", count);

// Import from YAML
let count = db.import_yaml(Path::new("users.yaml"))?;

// Export to JSON
db.export_json(Path::new("backup.json"))?;

// Export to YAML
db.export_yaml(Path::new("backup.yaml"))?;
```

## ODBC Driver

YamlDB includes an ODBC driver for SQL-based access.

### Connection String

```
DRIVER={YamlDB};DBQ=data.yaml;
DRIVER={YamlDB};FILE=data.yaml;
```

### Supported SQL

```sql
-- Select all
SELECT * FROM data

-- Where clause
SELECT * FROM data WHERE city = 'Beijing'

-- Comparison operators
SELECT * FROM data WHERE age > 25
SELECT * FROM data WHERE age >= 28
SELECT * FROM data WHERE age < 30
SELECT * FROM data WHERE age <= 25
SELECT * FROM data WHERE city != 'Shanghai'

-- AND/OR conditions
SELECT * FROM data WHERE city = 'Beijing' AND age >= 28
SELECT * FROM data WHERE city = 'Beijing' OR city = 'Shanghai'
```

### Build Shared Library

```bash
cargo build --release
```

Output:
- Windows: `target/release/yamldb.dll`
- Linux: `target/release/libyamldb.so`
- macOS: `target/release/libyamldb.dylib`

### Register Driver

**Windows:**
1. Open ODBC Data Source Administrator
2. Go to "Drivers" tab
3. Click "Add"
4. Browse to `yamldb.dll`

**Linux:**
Add to `/etc/odbcinst.ini`:
```ini
[YamlDB]
Description=YamlDB ODBC Driver
Driver=/path/to/libyamldb.so
```

### Usage Example (Python)

```python
import pyodbc

conn = pyodbc.connect('DRIVER={YamlDB};DBQ=data.yaml;')
cursor = conn.cursor()

cursor.execute("SELECT * FROM data WHERE city = 'Beijing'")
for row in cursor:
    print(row)

conn.close()
```

### Usage Example (C#)

```csharp
using System.Data.Odbc;

var conn = new OdbcConnection("DRIVER={YamlDB};DBQ=data.yaml;");
conn.Open();

var cmd = new OdbcCommand("SELECT * FROM data WHERE age > 25", conn);
var reader = cmd.ExecuteReader();

while (reader.Read())
{
    Console.WriteLine($"{reader["id"]}: {reader["name"]}");
}

conn.Close();
```

## Data Format

### YAML

```yaml
- id: user1
  name: Alice
  age: 30
  city: Beijing
- id: user2
  name: Bob
  age: 25
  city: Shanghai
```

### JSON

```json
[
  {"id": "user1", "name": "Alice", "age": 30, "city": "Beijing"},
  {"id": "user2", "name": "Bob", "age": 25, "city": "Shanghai"}
]
```

## Error Handling

```rust
use yamldb::{YamlDb, YamlDbError};

match db.create(record) {
    Ok(_) => println!("Success"),
    Err(YamlDbError::DuplicateKey(id)) => eprintln!("Duplicate: {}", id),
    Err(YamlDbError::NotFound(id)) => eprintln!("Not found: {}", id),
    Err(YamlDbError::Io(e)) => eprintln!("IO error: {}", e),
    Err(YamlDbError::Yaml(e)) => eprintln!("YAML error: {}", e),
    Err(YamlDbError::Json(e)) => eprintln!("JSON error: {}", e),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Project Structure

```
yamldb/
├── src/
│   ├── lib.rs      # Core library
│   ├── main.rs     # CLI tool
│   └── odbc.rs     # ODBC driver
├── tests/
│   ├── integration.rs  # Unit tests
│   └── odbc.rs         # ODBC tests
├── Cargo.toml
├── README.md
├── CHANGELOG.md
└── LICENSE
```

## License

MIT
