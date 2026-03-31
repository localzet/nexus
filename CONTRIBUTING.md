# Contributing to NexusDB

First off, thank you for considering contributing to NexusDB! It's people like you that make NexusDB such a great tool.

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the issue list as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible:

* **Use a clear and descriptive title**
* **Describe the exact steps which reproduce the problem**
* **Provide specific examples to demonstrate the steps**
* **Describe the behavior you observed after following the steps**
* **Explain which behavior you expected to see instead and why**
* **Include screenshots and animated GIFs if possible**
* **Include your environment details** (OS, Rust version, NexusDB version)

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please include:

* **Use a clear and descriptive title**
* **Provide a step-by-step description of the suggested enhancement**
* **Provide specific examples to demonstrate the steps**
* **Describe the current behavior and expected behavior**
* **Explain why this enhancement would be useful**
* **List some other databases where this enhancement exists, if applicable**

### Pull Requests

* Fill in the required template
* Follow the Rust styleguides
* Include appropriate test cases
* Update documentation as needed
* End all files with a newline

## Development Setup

### Prerequisites

* Rust 1.70 or later
* Cargo package manager
* Git

### Building from Source

```bash
git clone https://github.com/yourusername/nexus-db.git
cd nexus-db
cargo build --lib
cargo test --lib
```

### Running Tests

```bash
# Run all tests
cargo test --lib

# Run specific module tests
cargo test --lib mvcc
cargo test --lib sharding
cargo test --lib window_functions

# Run tests with output
cargo test --lib -- --nocapture

# Run benchmarks
cargo bench
```

### Code Style

We follow standard Rust conventions:

```bash
# Format code
cargo fmt

# Check code with Clippy
cargo clippy -- -D warnings
```

### Commit Messages

* Use the present tense ("Add feature" not "Added feature")
* Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
* Limit the first line to 72 characters or less
* Reference issues and pull requests liberally after the first line
* Consider starting the commit message with an applicable emoji:
  * 🎨 when improving the format/structure of the code
  * 🐎 when improving performance
  * 🔒 when dealing with security
  * 🐛 when fixing a bug
  * ✅ when adding tests
  * 📝 when writing docs
  * 🚀 when adding a feature

### Pull Request Process

1. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** and write tests:
   - Add unit tests for new functionality
   - Ensure all existing tests pass
   - Maintain or improve test coverage

3. **Update documentation**:
   - Update IMPLEMENTATION.md if architecture changed
   - Update README.md if user-facing features changed
   - Add docstring comments to public APIs

4. **Format and lint**:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   ```

5. **Commit with clear messages**:
   ```bash
   git commit -m "✨ Add window function support"
   ```

6. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

7. **Create a Pull Request** on GitHub:
   - Write a clear PR description
   - Reference any related issues
   - Include test results
   - Request review from maintainers

### Areas for Contribution

#### Easy (Good for first-time contributors)
- [ ] Improve documentation and examples
- [ ] Add more test cases for edge cases
- [ ] Fix typos and improve code comments
- [ ] Optimize existing implementations

#### Medium
- [ ] Implement planned window functions (NTILE, PERCENT_RANK)
- [ ] Add extended string functions (SUBSTRING, CONCAT, TRIM, etc.)
- [ ] Implement date/time functions
- [ ] Improve query optimization

#### Advanced
- [ ] Parallel query execution
- [ ] Columnar storage implementation
- [ ] Query plan caching
- [ ] Adaptive partitioning
- [ ] Geo-distributed replication

### Testing Requirements

Every contribution must include:

1. **Unit tests** for new functionality:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_feature_basic() {
           // Happy path
       }

       #[test]
       fn test_feature_edge_cases() {
           // Edge cases
       }

       #[test]
       fn test_feature_performance() {
           // Performance validation
       }
   }
   ```

2. **All existing tests must pass**:
   ```bash
   cargo test --lib
   ```

3. **Integration tests** if applicable

### Documentation

Every public API must have documentation:

```rust
/// Brief description of what this does
///
/// More detailed explanation with examples
///
/// # Examples
///
/// ```
/// // code example
/// ```
///
/// # Errors
///
/// Returns `Err` if...
pub fn my_function() -> Result<()> {
    // implementation
}
```

### Benchmarking

If performance is affected by your changes:

```rust
#[bench]
fn bench_my_feature(b: &mut Bencher) {
    let data = setup_test_data();
    b.iter(|| my_feature(&data))
}
```

## Style Guide

### Naming Conventions

- **Modules**: `snake_case` (e.g., `window_functions`)
- **Types**: `PascalCase` (e.g., `WindowFrame`)
- **Functions**: `snake_case` (e.g., `execute_query`)
- **Constants**: `UPPER_SNAKE_CASE` (e.g., `DEFAULT_CAPACITY`)

### Code Organization

```rust
// 1. Module documentation
/// Module description

// 2. Imports
use crate::types::{Row, Value};

// 3. Re-exports for public API
pub use self::submodule::PublicType;

// 4. Type definitions
pub struct MyStruct { }

// 5. Implementation
impl MyStruct { }

// 6. Tests
#[cfg(test)]
mod tests { }
```

### Error Handling

Use `anyhow::Result<T>` for public APIs:

```rust
pub fn my_function() -> anyhow::Result<String> {
    // implementation
}
```

## Questions?

Feel free to open a GitHub Discussion or contact the maintainers directly.

## Licensing

By contributing to NexusDB, you agree that your contributions will be licensed under its AGPL 3.0 License.

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [IMPLEMENTATION.md](IMPLEMENTATION.md) - Technical architecture
- [DEVELOPMENT_ROADMAP.md](DEVELOPMENT_ROADMAP.md) - Planned features
- [NexusDB Discussions](https://github.com/yourusername/nexus-db/discussions)

Thank you for contributing! 🚀
