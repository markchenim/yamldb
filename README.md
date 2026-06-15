# YamlDB

A lightweight YAML file-based database with CLI and ODBC driver support.

## Features

- YAML file as data storage
- CRUD operations
- Query builder with conditions
- Import/Export (JSON, YAML)
- CLI tool
- ODBC driver interface

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

# List all records
yamldb list

# Update record
yamldb update user1 --fields age=31,city=Guangzhou

# Delete record
yamldb delete user1

# Query records
yamldb query --key city --value Beijing

# Import from file
yamldb import -i users.json
yamldb import -i users.yaml
```

### Options

```
-f, --file <FILE>  Specify YAML file path [default: data.yaml]
```

### Import Format

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
```

### QueryResult

```rust
let result = db.query(&QueryOp::eq("city", "Beijing"));

// Get count
println!("Count: {}", result.count());

// Get first record
if let Some(record) = result.first() {
    println!("First: {}", record.id);
}

// Iterate
for record in result.iter() {
    println!("{}: {:?}", record.id, record.data);
}
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
```

## ODBC Driver

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
