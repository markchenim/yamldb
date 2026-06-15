use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use yamldb::{QueryOp, Record, YamlDb};

#[derive(Parser)]
#[command(name = "yamldb", about = "YAML file based database CLI")]
struct Cli {
    #[arg(short, long, default_value = "data.yaml")]
    file: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create {
        id: String,
        #[arg(short, long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    Get {
        id: String,
        #[arg(long)]
        format: Option<String>,
    },
    List {
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    Update {
        id: String,
        #[arg(short, long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    Delete {
        id: String,
    },
    Query {
        #[arg(short, long)]
        key: String,
        #[arg(short, long)]
        value: String,
        #[arg(long)]
        op: Option<String>,
    },
    Search {
        #[arg(short, long)]
        keyword: String,
        #[arg(short, long)]
        key: Option<String>,
    },
    Import {
        #[arg(short, long)]
        input: PathBuf,
    },
    Export {
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value = "json")]
        format: String,
    },
    Backup {
        #[arg(short, long)]
        output: PathBuf,
    },
    Stats,
    Count,
    Exists {
        id: String,
    },
    Clear {
        #[arg(long)]
        force: bool,
    },
}

fn parse_fields(fields: &[String]) -> HashMap<String, serde_yaml::Value> {
    let mut map = HashMap::new();
    for field in fields {
        if let Some((key, value)) = field.split_once('=') {
            let val = if let Ok(n) = value.parse::<i64>() {
                serde_yaml::Value::Number(n.into())
            } else if value == "true" || value == "false" {
                serde_yaml::Value::Bool(value == "true")
            } else {
                serde_yaml::Value::String(value.to_string())
            };
            map.insert(key.to_string(), val);
        }
    }
    map
}

fn format_record(record: &Record, format: Option<&str>) -> String {
    match format {
        Some("json") => record.to_json().unwrap_or_default(),
        _ => format!("{}: {}", record.id, serde_yaml::to_string(&record.data).unwrap_or_default()),
    }
}

fn import_records(db: &mut YamlDb, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "json" => {
            let records: Vec<serde_json::Value> = serde_json::from_str(&content)?;
            for item in records {
                if let Some(obj) = item.as_object() {
                    let id = obj
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing 'id' field")?
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
                    db.create(Record { id, data })?;
                }
            }
        }
        "yaml" | "yml" => {
            let records: Vec<Record> = serde_yaml::from_str(&content)?;
            for record in records {
                db.create(record)?;
            }
        }
        _ => return Err(format!("Unsupported format: {}", ext).into()),
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut db = YamlDb::new(&cli.file);
    db.load()?;

    match cli.command {
        Commands::Create { id, fields } => {
            let data = parse_fields(&fields);
            db.create(Record { id: id.clone(), data })?;
            println!("Created record: {}", id);
        }
        Commands::Get { id, format } => {
            let record = db.read(&id)?;
            println!("{}", format_record(record, format.as_deref()));
        }
        Commands::List { format, limit } => {
            let records = db.read_all();
            if records.is_empty() {
                println!("No records found");
            } else {
                let iter: Vec<_> = if let Some(n) = limit {
                    records.into_iter().take(n).collect()
                } else {
                    records
                };
                for record in iter {
                    println!("{}", format_record(record, format.as_deref()));
                }
            }
        }
        Commands::Update { id, fields } => {
            let data = parse_fields(&fields);
            db.update(&id, data)?;
            println!("Updated record: {}", id);
        }
        Commands::Delete { id } => {
            db.delete(&id)?;
            println!("Deleted record: {}", id);
        }
        Commands::Query { key, value, op } => {
            let query_op = match op.as_deref() {
                Some("ne") => QueryOp::ne(key, value),
                Some("gt") => QueryOp::gt(key, serde_yaml::Value::Number(value.parse::<i64>().unwrap_or(0).into())),
                Some("lt") => QueryOp::lt(key, serde_yaml::Value::Number(value.parse::<i64>().unwrap_or(0).into())),
                Some("gte") => QueryOp::gte(key, serde_yaml::Value::Number(value.parse::<i64>().unwrap_or(0).into())),
                Some("lte") => QueryOp::lte(key, serde_yaml::Value::Number(value.parse::<i64>().unwrap_or(0).into())),
                Some("contains") => QueryOp::contains(key, value),
                _ => QueryOp::eq(key, value),
            };
            let results = db.query(&query_op);
            if results.is_empty() {
                println!("No matching records");
            } else {
                for record in results.to_vec() {
                    println!("{}", format_record(record, None));
                }
            }
        }
        Commands::Search { keyword, key } => {
            let results = if let Some(k) = key {
                db.search(&k, &keyword)
            } else {
                db.search_all(&keyword)
            };
            if results.is_empty() {
                println!("No matching records");
            } else {
                for record in results.to_vec() {
                    println!("{}", format_record(record, None));
                }
            }
        }
        Commands::Import { input } => {
            import_records(&mut db, &input)?;
            println!("Imported records from: {}", input.display());
        }
        Commands::Export { output, format } => {
            match format.as_str() {
                "json" => db.export_json(&output)?,
                "yaml" | "yml" => db.export_yaml(&output)?,
                _ => return Err(format!("Unsupported format: {}", format).into()),
            }
            println!("Exported to: {}", output.display());
        }
        Commands::Backup { output } => {
            db.backup(&output)?;
            println!("Backup created: {}", output.display());
        }
        Commands::Stats => {
            let stats = db.stats();
            println!("Total records: {}", stats.total_records);
            println!("Total unique keys: {}", stats.total_keys);
            println!("Keys: {:?}", stats.unique_keys);
            if let Some(size) = stats.file_size {
                println!("File size: {} bytes", size);
            }
        }
        Commands::Count => {
            println!("{}", db.count());
        }
        Commands::Exists { id } => {
            if db.exists(&id) {
                println!("true");
            } else {
                println!("false");
            }
        }
        Commands::Clear { force } => {
            if force {
                db.clear()?;
                println!("Database cleared");
            } else {
                println!("Use --force to confirm clearing the database");
            }
        }
    }

    Ok(())
}
