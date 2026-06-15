# Changelog

All notable changes to this project will be documented in this file.

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
