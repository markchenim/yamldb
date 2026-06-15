# Contributing to YamlDB

Thank you for your interest in contributing to YamlDB!

## Getting Started

1. Fork the repository
2. Clone your fork
3. Create a feature branch
4. Make your changes
5. Run tests
6. Submit a pull request

## Development Setup

```bash
# Clone the repo
git clone https://github.com/your-username/yamldb.git
cd yamldb

# Build
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy -- -D warnings
```

## Code Style

- Follow Rust standard conventions
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for warnings
- Add tests for new features
- Update documentation for API changes

## Commit Messages

Use conventional commits:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation
- `test:` - Tests
- `refactor:` - Code refactoring
- `chore:` - Maintenance

## Pull Requests

1. Keep PRs focused on a single change
2. Include tests for new functionality
3. Update documentation as needed
4. Ensure all tests pass
5. Ensure clippy has no warnings

## Reporting Issues

- Use GitHub Issues
- Include reproduction steps
- Include Rust version and OS
- Include error messages

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
