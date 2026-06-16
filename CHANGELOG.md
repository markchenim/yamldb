# Changelog

All notable changes to this project will be documented in this file.

## [0.9.0] - 2026-06-16

### Added
- Dependency-free JDBC driver with `jdbc:yamldb:` URLs and read-only `SELECT * FROM data` support.
- JDBC build scripts for Windows and Unix-like environments.
- JDBC smoke test and GitHub Actions CI job.
- Chinese documentation (`README.zh-CN.md`).
- Release artifacts for ODBC shared libraries and JDBC jar.

### Changed
- Release workflow now publishes CLI binaries, ODBC drivers, and `yamldb-jdbc.jar`.
- Package metadata is synchronized for Cargo publishing.
- YAML/JSON export and query output are ordered by record id for stable results.

### Fixed
- `QueryResult::page` now handles zero page or page size safely.
- YAML save, backup, and export now use a safer write path.
- `YamlDb::memory().export_yaml(...)` now writes records correctly.
- CLI field parsing now reports invalid `key=value` input instead of silently ignoring it.
- CLI query operators no longer coerce invalid comparison values to zero.
- Clippy and Rust 2024 warnings in tests and examples.

## [0.7.0] - 2026-06-15

### Added
- ODBC driver integration tests (6 test cases)
- Comprehensive README documentation

### Fixed
- Handle ID offset to avoid null pointer from slot index 0
- Case-insensitive SQL parsing (preserve value case)
- Null handle checks for all ODBC functions
- Test isolation for global state

## [0.6.0] - 2026-06-15

### Added
- ODBC driver module (`src/odbc.rs`)
- ODBC exported functions: `SQLAllocHandle`, `SQLFreeHandle`, `SQLConnect`, `SQLDisconnect`, `SQLExecDirect`, `SQLFetch`, `SQLGetData`, `SQLNumResultCols`, `SQLDescribeCol`, `SQLRowCount`, `SQLColAttribute`
- Shared library build support (`cdylib`)

### Fixed
- Rust 2024 edition compatibility (`#[unsafe(no_mangle)]`, `unsafe {}` blocks)
- Lifetime issues in ODBC driver (copy data before use)
- Clippy warnings (`missing_safety_doc`, `manual_dangling_ptr`)

## [0.5.0] - 2026-06-15

### Added
- `search` command for fuzzy search by keyword
- `export` command for exporting to JSON/YAML
- `backup` command for database backup
- `stats` command for database statistics
- `count` command for record count
- `exists` command for checking record existence
- `clear` command for clearing database
- `--format` option for JSON output
- `--limit` option for list command
- `--op` option for query command (ne, gt, lt, gte, lte, contains)
- `QueryOp::starts_with` and `QueryOp::ends_with` operators
- `QueryOp::negate` operator (renamed from `not` to avoid trait conflict)
- `Record::merge` method
- `Record::to_json` method
- `Record::has_key` and `Record::keys` methods
- `YamlDb::search` and `YamlDb::search_all` methods
- `YamlDb::stats` method
- `YamlDb::backup` method
- `YamlDb::read_many` method
- `YamlDb::update_many` method
- `YamlDb::export_yaml` method
- `QueryResult::last` method
- `QueryResult::skip` method
- `QueryResult::page` method for pagination
- `QueryResult::ids` method
- `DbStats` struct for database statistics

### Changed
- Improved error handling with `Validation` error type

## [0.4.0] - 2026-06-15

### Added
- `search` command for fuzzy search by keyword
- `export` command for exporting to JSON/YAML
- `backup` command for database backup
- `stats` command for database statistics
- `count` command for record count
- `exists` command for checking record existence
- `clear` command for clearing database
- `--format` option for JSON output
- `--limit` option for list command
- `--op` option for query command (ne, gt, lt, gte, lte, contains)
- `QueryOp::starts_with` and `QueryOp::ends_with` operators
- `QueryOp::not` operator
- `Record::merge` method
- `Record::to_json` method
- `Record::has_key` and `Record::keys` methods
- `YamlDb::search` and `YamlDb::search_all` methods
- `YamlDb::stats` method
- `YamlDb::backup` method
- `YamlDb::read_many` method
- `YamlDb::update_many` method
- `YamlDb::export_yaml` method
- `QueryResult::last` method
- `QueryResult::skip` method
- `QueryResult::page` method for pagination
- `QueryResult::ids` method
- `DbStats` struct for database statistics

### Changed
- Improved error handling with `Validation` error type

## [0.3.2] - 2026-06-15

### Fixed
- Explicit lifetime annotations for `QueryResult`

## [0.3.1] - 2026-06-15

### Fixed
- Remove ODBC module causing compilation errors

## [0.3.0] - 2026-06-15

### Added
- Query builder with `QueryOp` enum
- `QueryResult` with sorting, pagination, and iteration
- `upsert` method for insert or update
- `update_field` method for single field update
- `delete_many` method for batch delete
- `count` and `exists` methods
- `import_json`, `import_yaml`, `export_json` methods

## [0.2.0] - 2026-06-15

### Added
- CLI `import` command for JSON/YAML import
- `serde_json` dependency

## [0.1.0] - 2026-06-15

### Added
- Initial release
- Basic CRUD operations (create, read, update, delete)
- CLI tool
- YAML file storage
- Memory database support
