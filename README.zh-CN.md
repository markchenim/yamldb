# YamlDB 中文文档

[English](README.md)

[![CI](https://github.com/markchenim/yamldb/actions/workflows/ci.yml/badge.svg)](https://github.com/markchenim/yamldb/actions/workflows/ci.yml)
[![Release](https://github.com/markchenim/yamldb/actions/workflows/release.yml/badge.svg)](https://github.com/markchenim/yamldb/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

YamlDB 是一个轻量级 YAML 文件数据库，提供命令行工具、Rust API、实验性的 ODBC 驱动和 JDBC 驱动支持。它适合保存少量结构化数据、测试数据、配置型数据和需要人工可读/可编辑的数据文件。

## 功能特性

- **YAML 存储**：数据文件可读、可编辑，适合版本管理。
- **CRUD 操作**：支持创建、读取、更新、删除记录。
- **查询构造器**：支持等于、不等于、大小比较、字符串匹配、AND/OR/NOT。
- **模糊搜索**：支持大小写不敏感的关键字搜索。
- **导入导出**：支持 JSON 和 YAML。
- **备份**：可生成数据库快照。
- **CLI 工具**：可直接在命令行管理 YAML 数据。
- **ODBC 驱动**：支持通过简单 SQL 查询 YAML 数据。
- **JDBC 驱动**：支持 Java 通过 JDBC 读取 YAML 数据。

## 安装

### 通过 Cargo 安装

```bash
cargo install yamldb
```

### 从 GitHub Releases 下载

可以从 [GitHub Releases](https://github.com/markchenim/yamldb/releases) 下载预构建文件：

| 平台 | CLI | ODBC 驱动 | JDBC 驱动 |
| ---- | --- | --------- | --------- |
| Linux | `yamldb-linux` | `libyamldb-linux-odbc.so` | `yamldb-jdbc.jar` |
| Windows | `yamldb-windows.exe` | `yamldb-windows-odbc.dll` | `yamldb-jdbc.jar` |
| macOS | `yamldb-macos` | `libyamldb-macos-odbc.dylib` | `yamldb-jdbc.jar` |

## CLI 用法

### 全局参数

```text
-f, --file <FILE>  指定 YAML 数据文件路径，默认 data.yaml
```

### 记录操作

```bash
# 创建记录
yamldb create user1 --fields name=Alice,age=30,city=Beijing

# 读取记录
yamldb get user1
yamldb get user1 --format json

# 列出记录
yamldb list
yamldb list --limit 10
yamldb list --format json

# 更新记录
yamldb update user1 --fields age=31,city=Guangzhou

# 删除记录
yamldb delete user1
```

### 查询与搜索

```bash
# 等值查询
yamldb query --key city --value Beijing

# 比较操作
yamldb query --key age --value 25 --op gt
yamldb query --key age --value 30 --op gte
yamldb query --key age --value 50 --op lt
yamldb query --key age --value 25 --op lte
yamldb query --key city --value Beijing --op ne

# 字符串包含
yamldb query --key name --value Ali --op contains

# 模糊搜索
yamldb search --keyword alice
yamldb search --keyword alice --key name
```

### 导入、导出与备份

```bash
# 导入
yamldb import -i users.json
yamldb import -i users.yaml

# 导出
yamldb export -o backup.json
yamldb export -o backup.yaml --format yaml

# 备份
yamldb backup -o backup.yaml
```

### 工具命令

```bash
# 统计信息
yamldb stats

# 记录数量
yamldb count

# 检查记录是否存在
yamldb exists user1

# 清空数据库
yamldb clear --force
```

## Rust API

### 基本用法

```rust
use yamldb::{Record, YamlDb};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = YamlDb::new("data.yaml");
    db.load()?;

    let mut record = Record::new("user1");
    record
        .set("name", "Alice")
        .set("age", 30)
        .set("city", "Beijing");
    db.create(record)?;

    let record = db.read("user1")?;
    println!("Name: {:?}", record.get_str("name"));
    println!("Age: {:?}", record.get_i64("age"));

    db.update_field("user1", "age", serde_yaml::Value::Number(31.into()))?;
    db.delete("user1")?;

    Ok(())
}
```

### 内存数据库

```rust
let mut db = YamlDb::memory();
let mut record = Record::new("test");
record.set("key", "value");
db.create(record)?;
```

### Record API

```rust
let mut record = Record::new("user1");

record
    .set("name", "Alice")
    .set("age", 30)
    .set("active", true);

record.get_str("name");
record.get_i64("age");
record.get_f64("score");
record.get_bool("active");

record.has_key("name");
record.keys();

let mut other = Record::new("user1");
other.set("email", "alice@example.com");
record.merge(&other);

let json = record.to_json()?;
```

### 查询构造器

```rust
use yamldb::{QueryOp, YamlDb};

let mut db = YamlDb::new("data.yaml");
db.load()?;

let result = db.query(&QueryOp::eq("city", "Beijing"));
let result = db.query(&QueryOp::ne("city", "Shanghai"));
let result = db.query(&QueryOp::gt("age", serde_yaml::Value::Number(25.into())));
let result = db.query(&QueryOp::gte("age", serde_yaml::Value::Number(30.into())));
let result = db.query(&QueryOp::lt("age", serde_yaml::Value::Number(50.into())));
let result = db.query(&QueryOp::lte("age", serde_yaml::Value::Number(25.into())));

let result = db.query(&QueryOp::contains("name", "Ali"));
let result = db.query(&QueryOp::starts_with("name", "Ali"));
let result = db.query(&QueryOp::ends_with("name", "Smith"));

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

### 搜索

```rust
let result = db.search("name", "alice");
let result = db.search_all("alice");
```

### QueryResult

```rust
let result = db.query(&QueryOp::eq("city", "Beijing"));

println!("Found: {}", result.count());
println!("Is empty: {}", result.is_empty());

if let Some(first) = result.first() {
    println!("First: {}", first.id);
}

let page1 = result.page(1, 10);
let page2 = result.page(2, 10);

let skipped = result.skip(5);
let limited = result.limit(10);
let ids = result.ids();

for record in result.iter() {
    println!("{}: {:?}", record.id, record.data);
}

let all = result.to_vec();
```

### 批量操作

```rust
use std::collections::HashMap;

let records = db.read_many(&["user1", "user2", "user3"]);

let updates = vec![
    (
        "user1".to_string(),
        HashMap::from([("age".to_string(), serde_yaml::Value::Number(31.into()))]),
    ),
    (
        "user2".to_string(),
        HashMap::from([("age".to_string(), serde_yaml::Value::Number(26.into()))]),
    ),
];
let updated = db.update_many(updates)?;

let deleted = db.delete_many(&["user1", "user2"])?;

let mut record = Record::new("user1");
record.set("name", "Alice");
db.upsert(record)?;
```

### 统计与备份

```rust
use std::path::Path;

let stats = db.stats();
println!("Total records: {}", stats.total_records);
println!("Unique keys: {:?}", stats.unique_keys);
println!("File size: {:?} bytes", stats.file_size);

db.backup(Path::new("backup.yaml"))?;
db.clear()?;
```

### 导入与导出

```rust
use std::path::Path;

let count = db.import_json(Path::new("users.json"))?;
let count = db.import_yaml(Path::new("users.yaml"))?;

db.export_json(Path::new("backup.json"))?;
db.export_yaml(Path::new("backup.yaml"))?;
```

## ODBC 驱动

YamlDB 提供 ODBC 驱动，可用简单 SQL 读取 YAML 数据。

### 连接字符串

```text
DRIVER={YamlDB};DBQ=data.yaml;
DRIVER={YamlDB};FILE=data.yaml;
```

### 支持的 SQL

```sql
SELECT * FROM data
SELECT * FROM data WHERE city = 'Beijing'
SELECT * FROM data WHERE age > 25
SELECT * FROM data WHERE age >= 28
SELECT * FROM data WHERE age < 30
SELECT * FROM data WHERE age <= 25
SELECT * FROM data WHERE city != 'Shanghai'
SELECT * FROM data WHERE city = 'Beijing' AND age >= 28
SELECT * FROM data WHERE city = 'Beijing' OR city = 'Shanghai'
```

### 构建共享库

```bash
cargo build --release
```

输出文件：

- Windows：`target/release/yamldb.dll`
- Linux：`target/release/libyamldb.so`
- macOS：`target/release/libyamldb.dylib`

### 注册驱动

Windows：

1. 打开 ODBC 数据源管理器。
2. 进入 Drivers 选项卡。
3. 点击 Add。
4. 选择 `yamldb.dll`。

Linux：

在 `/etc/odbcinst.ini` 中添加：

```ini
[YamlDB]
Description=YamlDB ODBC Driver
Driver=/path/to/libyamldb.so
```

### Python 示例

```python
import pyodbc

conn = pyodbc.connect('DRIVER={YamlDB};DBQ=data.yaml;')
cursor = conn.cursor()

cursor.execute("SELECT * FROM data WHERE city = 'Beijing'")
for row in cursor:
    print(row)

conn.close()
```

### C# 示例

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

## JDBC 驱动

YamlDB 提供一个无外部依赖的 JDBC 驱动，可供 Java 通过 SQL 读取 YAML 数据。

### 连接 URL

```text
jdbc:yamldb:data.yaml
jdbc:yamldb:file:data.yaml
```

### 支持的 SQL

JDBC 驱动支持和 ODBC 类似的只读 SQL 子集：

```sql
SELECT * FROM data
SELECT * FROM data WHERE city = 'Beijing'
SELECT * FROM data WHERE age >= 28
SELECT * FROM data WHERE city = 'Beijing' AND age >= 28
SELECT * FROM data WHERE city = 'Beijing' OR city = 'Shanghai'
```

### 构建 JDBC Jar

Windows：

```powershell
powershell -ExecutionPolicy Bypass -File jdbc\build.ps1
```

Linux/macOS：

```bash
bash jdbc/build.sh
```

输出文件：

```text
jdbc/target/yamldb-jdbc.jar
```

### Java 示例

```java
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.Statement;

Class.forName("io.github.markchenim.yamldb.jdbc.YamlDbJdbcDriver");

try (Connection conn = DriverManager.getConnection("jdbc:yamldb:data.yaml");
     Statement stmt = conn.createStatement();
     ResultSet rs = stmt.executeQuery("SELECT * FROM data WHERE city = 'Beijing'")) {
    while (rs.next()) {
        System.out.println(rs.getString("id") + ": " + rs.getString("name"));
    }
}
```

## 数据格式

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

## 错误处理

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

## 项目结构

```text
yamldb/
├── src/
│   ├── lib.rs      # 核心库
│   ├── main.rs     # CLI 工具
│   └── odbc.rs     # ODBC 驱动
├── tests/
│   ├── integration.rs
│   └── odbc.rs
├── examples/
│   └── odbc_usage.rs
├── jdbc/
│   ├── src/main/java   # JDBC 驱动
│   └── src/test/java   # JDBC 冒烟测试
├── Cargo.toml
├── README.md
├── README.zh-CN.md
├── CHANGELOG.md
└── LICENSE
```

## 注意事项

- YamlDB 适合轻量数据、配置型数据和测试数据，不适合作为高并发或大规模数据库。
- YAML 文件便于人工编辑，但并发写入需要由调用方控制。
- ODBC SQL 支持的是项目内实现的简单子集，不等同于完整 SQL 数据库。
- JDBC SQL 支持的是项目内实现的简单只读子集，不等同于完整 SQL 数据库。

## 许可证

MIT
