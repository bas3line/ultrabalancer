# Contributing to UltraBalancer

Thanks for your interest in contributing! This document outlines the process for contributing to UltraBalancer.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting Started

### Prerequisites

- Rust 1.75 or higher
- Git
- A code editor (VS Code, Neovim, etc.)

### Development Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/bas3line/ultrabalancer.git
   cd ultrabalancer
   ```

3. Install Rust if you haven't already:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

4. Build the project:
   ```bash
   cargo build
   ```

5. Run tests:
   ```bash
   cargo test
   ```

## Contribution Guidelines

### Reporting Bugs

1. Check existing issues to avoid duplicates
2. Use the bug report template
3. Include:
   - Clear title and description
   - Steps to reproduce
   - Expected vs actual behavior
   - Rust version and OS
   - Any relevant logs

### Suggesting Features

1. Check existing issues and discussions
2. Open a discussion first for major features
3. Provide clear use cases and rationale

### Pull Requests

#### Before You Submit

- [ ] Tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated if needed
- [ ] Commits are clean and descriptive

#### PR Process

1. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/bug-description
   ```

2. Make your changes

3. Commit with clear messages:
   ```bash
   git commit -m "feat: add power of two choice algorithm"
   ```

4. Push and create PR:
   ```bash
   git push origin feature/your-feature-name
   ```

#### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code refactoring
- `test`: Tests
- `chore`: Maintenance

### Coding Standards

- Use meaningful variable names
- Add comments for complex logic
- Keep functions small and focused
- Write tests for new features
- Handle errors gracefully

### Documentation

- Update README.md for user-facing changes
- Add doc comments to public APIs
- Include examples where helpful

## Getting Help

- Open a [Discussion](https://github.com/bas3line/ultrabalancer/discussions)
- Join our community
- Email: hi@ultrabalancer.com

## Recognition

Contributors will be listed in the README.md and on our website.

Thank you for contributing!