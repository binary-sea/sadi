# Contributing to SaDi

Thank you for your interest in contributing to SaDi! This document provides guidelines and information for contributors to help maintain code quality and ensure smooth collaboration.

## 🚀 Quick Start

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/your-username/sadi.git
   cd sadi
   ```
3. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```
4. **Make your changes** following the guidelines below
5. **Test your changes** thoroughly
6. **Submit a pull request**

## 📋 Development Setup

### Prerequisites

- **Rust** (latest stable version recommended)
- **Git** for version control
- A good **text editor** or **IDE** with Rust support (VS Code with rust-analyzer recommended)

### Local Development

```bash
# Clone the repository
git clone https://github.com/JoaoPedro61/sadi.git
cd sadi

# Build the project
cargo build

# Run tests
cargo test --all

# Run clippy for linting
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --check

# Build documentation
cargo doc --no-deps --open
```

### Running Examples

```bash
# Run the comprehensive basic example
cd examples/basic
cargo run

# Run with tracing enabled
RUST_LOG=debug cargo run --features tracing
```

## 🎯 Types of Contributions

We welcome various types of contributions:

### 🐛 Bug Fixes
- Fix existing bugs in the dependency injection logic
- Improve error handling and error messages
- Fix memory leaks or performance issues

### ✨ New Features
- Add new dependency injection patterns
- Implement async support (see roadmap)
- Add new service lifetime management options
- Enhance circular dependency detection

### 📚 Documentation
- Improve inline documentation (doc comments)
- Add more examples to doc comments
- Update README.md with new features
- Create tutorials and guides

### 🧪 Tests
- Add unit tests for edge cases
- Create integration tests for complex scenarios
- Add benchmarks for performance testing

### 🔧 Refactoring
- Improve code organization and structure
- Extract reusable components
- Optimize performance without changing APIs

## 📝 Code Guidelines

### Rust Style

- **Follow standard Rust conventions** (use `cargo fmt`)
- **Use meaningful variable and function names**
- **Write comprehensive doc comments** for public APIs
- **Include examples in doc comments** where helpful
- **Handle errors gracefully** - prefer `Result<T, E>` over panicking

### Code Quality

```bash
# Before submitting, ensure these pass:
cargo test --all                                          # All tests pass
cargo clippy --workspace --all-targets --all-features    # No clippy warnings
cargo fmt --check                                         # Code is formatted
cargo doc --no-deps                                       # Documentation builds
```

### API Design Principles

- **Type Safety**: Leverage Rust's type system for compile-time guarantees
- **Zero-Cost Abstractions**: Avoid runtime overhead where possible
- **Ergonomics**: APIs should be easy and intuitive to use
- **Composability**: Services should work well together
- **Error Clarity**: Error messages should be helpful and actionable

## 🧪 Testing Guidelines

### Test Structure

Tests are organized as follows:
- **Unit tests**: In `src/sadi.rs` using `#[cfg(test)]` modules
- **Doc tests**: Embedded in documentation comments
- **Integration tests**: In `examples/` directory
- **Example tests**: In `examples/basic/src/main.rs`

### Writing Tests

```rust
#[test]
fn test_descriptive_name() {
    // Arrange
    let container = SaDi::new()
        .factory(|_| MyService::new());

    // Act
    let result = container.get::<MyService>();

    // Assert
    assert_eq!(result.value, expected_value);
}
```

### Test Requirements

- **All new features must include tests**
- **Bug fixes must include regression tests**
- **Tests should cover both success and error cases**
- **Use descriptive test names** that explain what's being tested
- **Include edge cases and boundary conditions**

## 📚 Documentation Standards

### Doc Comments

```rust
/// Brief one-line description.
///
/// Longer description providing context and usage information.
/// Explain the purpose, behavior, and any important considerations.
///
/// # Arguments
///
/// * `param` - Description of the parameter
///
/// # Returns
///
/// Description of what is returned
///
/// # Errors
///
/// When this function returns an error and why
///
/// # Examples
///
/// ```rust
/// use sadi::SaDi;
///
/// let container = SaDi::new()
///     .factory(|_| MyService::new());
/// let service = container.get::<MyService>();
/// ```
///
/// # Panics
///
/// When this function might panic (if applicable)
pub fn example_function(&self) -> Result<T, Error> {
    // Implementation
}
```

### Documentation Requirements

- **All public APIs must have comprehensive documentation**
- **Include at least one example** for non-trivial functions
- **Document error conditions** and when they occur
- **Explain the purpose and use cases** for new features
- **Keep examples simple but realistic**

## 🔄 Pull Request Process

### Before Opening a PR

1. **Ensure all tests pass**:
   ```bash
   cargo test --all
   ```

2. **Fix any clippy warnings**:
   ```bash
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```

3. **Format your code**:
   ```bash
   cargo fmt
   ```

4. **Update documentation** if needed

5. **Add entries to CHANGELOG.md** for notable changes

### PR Requirements

- **Fill out the PR template** completely
- **Include tests** for new functionality
- **Update documentation** as needed
- **Keep PRs focused** on a single feature or fix
- **Write clear commit messages**

### Commit Message Format

```
type: brief description (50 chars max)

Longer description if needed, explaining the what and why,
not the how. Wrap lines at 72 characters.

- List specific changes
- Reference issues: Fixes #123
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

### Review Process

1. **Automated checks** will run on your PR
2. **Maintainer review** will focus on:
   - Code quality and style
   - Test coverage
   - Documentation completeness
   - API design consistency
3. **Address feedback** promptly and thoroughly
4. **Squash commits** before merging if requested

## 🚦 Git Workflow

### Branching Strategy

- `main` - Stable production branch
- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates

### Working with Branches

```bash
# Stay up to date with main
git checkout main
git pull origin main

# Create and switch to your feature branch
git checkout -b feature/awesome-feature

# Make your changes, commit frequently
git add .
git commit -m "feat: add awesome feature"

# Push your branch
git push origin feature/awesome-feature

# Open PR on GitHub
```

## 📊 Performance Considerations

### Benchmarking

When making performance-related changes:

1. **Measure before and after** your changes
2. **Use consistent test environments**
3. **Include benchmark results** in your PR description
4. **Focus on real-world scenarios**

```bash
# Example benchmark command (if benchmarks exist)
cargo bench

# Profile with specific features
cargo build --release --features tracing
```

### Memory Usage

- **Minimize heap allocations** in hot paths
- **Use `Rc<T>` for shared singleton instances**
- **Consider `Box<T>` only when necessary** for recursive types
- **Avoid memory leaks** in circular dependencies

## 🗺️ Roadmap & Priorities

See the README.md for our current roadmap. High-priority items include:

1. **Async Support** - Async factory functions and service resolution
2. **Thread Safety** - Arc-based container for multi-threaded use
3. **Service Scoping** - Request-scoped and custom lifetime management
4. **Performance** - Compile-time DI and optimization

## 💬 Communication

### Getting Help

- **GitHub Issues** - For bug reports and feature requests
- **GitHub Discussions** - For questions and general discussion
- **Pull Requests** - For code review and collaboration

### Reporting Issues

When reporting bugs:

1. **Use the issue template** provided
2. **Include minimal reproduction** case
3. **Specify Rust version** and platform
4. **Include relevant error messages** and stack traces
5. **Check existing issues** to avoid duplicates

## 📜 Code of Conduct

- **Be respectful** and inclusive
- **Focus on technical merit** in discussions
- **Help others learn** and grow
- **Assume good intentions**
- **Keep discussions professional**

## ⚖️ Legal

By contributing to SaDi, you agree that:

- Your contributions will be licensed under the same license as the project (MIT)
- You have the right to submit your contributions
- Your contributions are your original work or properly attributed

## 🙏 Recognition

Contributors are recognized in:
- **README.md** acknowledgments section
- **CHANGELOG.md** for notable contributions
- **GitHub contributors** page

Thank you for helping make SaDi better! 🦀✨

---

**Questions?** Feel free to open an issue or start a discussion. We're here to help!