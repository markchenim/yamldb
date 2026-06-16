fn main() {
    println!("YamlDB ODBC Driver Example");
    println!("=========================");
    println!();
    println!("This driver allows accessing YAML files via ODBC interface.");
    println!();
    println!("Usage in ODBC connection string:");
    println!("  DRIVER={{YamlDB}};DBQ=data.yaml;");
    println!("  or");
    println!("  DRIVER={{YamlDB}};FILE=data.yaml;");
    println!();
    println!("SQL Examples:");
    println!("  SELECT * FROM data");
    println!("  SELECT * FROM data WHERE city = 'Beijing'");
    println!("  SELECT * FROM data WHERE age > 25");
    println!("  SELECT * FROM data WHERE city = 'Beijing' AND age >= 28");
    println!();
    println!("To use this driver:");
    println!("1. Build as shared library: cargo build --release");
    println!("2. Register the driver in ODBC Data Source Administrator");
    println!("3. Use connection string to connect");
}
