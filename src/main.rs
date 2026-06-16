use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

fn parse_value(value: &str) -> serde_yaml::Value {
    if let Ok(n) = value.parse::<i64>() {
        serde_yaml::Value::Number(n.into())
    } else if let Ok(f) = value.parse::<f64>() {
        serde_yaml::Value::Number(serde_yaml::Number::from(f))
    } else if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        serde_yaml::Value::Bool(value.eq_ignore_ascii_case("true"))
    } else {
        serde_yaml::Value::String(value.to_string())
    }
}

fn parse_fields(
    fields: &[String],
) -> Result<HashMap<String, serde_yaml::Value>, Box<dyn std::error::Error>> {
    let mut map = HashMap::new();
    for field in fields {
        let Some((key, value)) = field.split_once('=') else {
            return Err(format!("Invalid field '{}', expected key=value", field).into());
        };
        if key.trim().is_empty() {
            return Err(format!("Invalid field '{}', key cannot be empty", field).into());
        }
        map.insert(key.trim().to_string(), parse_value(value.trim()));
    }
    Ok(map)
}

fn format_record(record: &Record, format: Option<&str>) -> String {
    match format {
        Some("json") => record.to_json().unwrap_or_default(),
        _ => format!(
            "{}: {}",
            record.id,
            serde_yaml::to_string(&record.data).unwrap_or_default()
        ),
    }
}

fn import_records(db: &mut YamlDb, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "json" => {
            db.import_json(path)?;
        }
        "yaml" | "yml" => {
            db.import_yaml(path)?;
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
            let data = parse_fields(&fields)?;
            db.create(Record {
                id: id.clone(),
                data,
            })?;
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
            let data = parse_fields(&fields)?;
            db.update(&id, data)?;
            println!("Updated record: {}", id);
        }
        Commands::Delete { id } => {
            db.delete(&id)?;
            println!("Deleted record: {}", id);
        }
        Commands::Query { key, value, op } => {
            let query_op = match op.as_deref() {
                Some("ne") => QueryOp::ne(key, parse_value(&value)),
                Some("gt") => QueryOp::gt(key, parse_value(&value)),
                Some("lt") => QueryOp::lt(key, parse_value(&value)),
                Some("gte") => QueryOp::gte(key, parse_value(&value)),
                Some("lte") => QueryOp::lte(key, parse_value(&value)),
                Some("contains") => QueryOp::contains(key, value),
                Some(other) => return Err(format!("Unsupported query operator: {}", other).into()),
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
