use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use yamldb::{Record, YamlDb};

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
    },
    List,
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
    },
    Import {
        #[arg(short, long)]
        input: PathBuf,
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
        Commands::Get { id } => {
            let record = db.read(&id)?;
            println!("{}: {}", record.id, serde_yaml::to_string(&record.data)?);
        }
        Commands::List => {
            let records = db.read_all();
            if records.is_empty() {
                println!("No records found");
            } else {
                for record in records {
                    println!("{}: {}", record.id, serde_yaml::to_string(&record.data)?);
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
        Commands::Query { key, value } => {
            let results = db.query(|r| {
                r.data
                    .get(&key)
                    .and_then(|v| {
                        if let Some(s) = v.as_str() {
                            Some(s == value)
                        } else if let Some(n) = v.as_i64() {
                            Some(n.to_string() == value)
                        } else if let Some(b) = v.as_bool() {
                            Some(b.to_string() == value)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(false)
            });
            if results.is_empty() {
                println!("No matching records");
            } else {
                for record in results {
                    println!("{}: {}", record.id, serde_yaml::to_string(&record.data)?);
                }
            }
        }
        Commands::Import { input } => {
            import_records(&mut db, &input)?;
            println!("Imported records from: {}", input.display());
        }
    }

    Ok(())
}
