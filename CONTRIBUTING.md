# Contributing to Skillshare App

Thank you for your interest in contributing to Skillshare App! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Reporting Bugs](#reporting-bugs)
- [Requesting Features](#requesting-features)

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment. Please be kind and constructive in all interactions.

## Getting Started

1. Fork the repository
2. Clone your fork locally
3. Set up the development environment
4. Create a branch for your changes
5. Make your changes and test them
6. Submit a pull request

## Development Setup

### Prerequisites

- Node.js 18+
- Rust 1.70+
- pnpm

### Installation

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/skillshare-app.git
cd skillshare-app

# Install dependencies
pnpm install

# Start Vite (web UI)
pnpm dev

# Start the desktop app (Tauri)
pnpm dev:tauri
```

### Project Structure

```
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── hooks/              # Custom React hooks
│   ├── lib/                # Utilities & Tauri API
│   └── types/              # TypeScript types
├── src-tauri/              # Rust backend
│   └── src/
│       ├── commands/       # Tauri IPC handlers
│       └── models/         # Data structures
```

## How to Contribute

### Types of Contributions

- **Bug fixes**: Fix issues reported in the issue tracker
- **Features**: Implement new features (please discuss first)
- **Documentation**: Improve or add documentation
- **Tests**: Add or improve test coverage
- **Refactoring**: Improve code quality without changing functionality

### Before You Start

1. Check [existing issues](https://github.com/runkids/skillshare-app/issues) to avoid duplicates
2. For new features, open an issue first to discuss the proposal
3. For bug fixes, check if there's an existing issue or create one

## Pull Request Process

1. **Create a branch**
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/issue-description
   ```

2. **Make your changes**
   - Write clear, concise commit messages
   - Follow the coding standards below
   - Test your changes thoroughly

3. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add new feature description"
   ```

4. **Push to your fork**
   ```bash
   git push origin feature/your-feature-name
   ```

5. **Create a Pull Request**
   - Use a clear, descriptive title
   - Reference any related issues
   - Describe what changes you made and why

### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting, etc.)
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks

Examples:
```
feat: add workflow export functionality
fix: resolve terminal output encoding issue
docs: update installation instructions
```

## Coding Standards

### TypeScript/React

- Use functional components with hooks
- Use TypeScript strict mode
- Follow existing code patterns in the codebase
- Use meaningful variable and function names

### Rust

- Follow standard Rust conventions
- Use `serde` for serialization
- Handle errors appropriately with Result types
- Document public functions

### General

- Keep functions small and focused
- Write self-documenting code
- Add comments only when necessary to explain "why"
- Avoid premature optimization

## Reporting Bugs

When reporting a bug, please include:

1. **Title**: Clear, concise description of the issue
2. **Environment**:
   - OS and version (e.g., macOS 14.0)
   - Skillshare App version
   - Node.js version
3. **Steps to Reproduce**: Detailed steps to reproduce the issue
4. **Expected Behavior**: What you expected to happen
5. **Actual Behavior**: What actually happened
6. **Screenshots**: If applicable
7. **Additional Context**: Any other relevant information

### Bug Report Template

```markdown
## Description
[Clear description of the bug]

## Environment
- OS: [e.g., macOS 14.0]
- Skillshare App Version: [e.g., 1.0.0]
- Node.js Version: [e.g., 20.10.0]

## Steps to Reproduce
1. Go to '...'
2. Click on '...'
3. See error

## Expected Behavior
[What you expected to happen]

## Actual Behavior
[What actually happened]

## Screenshots
[If applicable]

## Additional Context
[Any other relevant information]
```

## Requesting Features

When requesting a feature, please include:

1. **Title**: Clear, concise description of the feature
2. **Problem**: What problem does this solve?
3. **Solution**: Your proposed solution
4. **Alternatives**: Any alternative solutions you considered
5. **Additional Context**: Mockups, examples, or references

### Feature Request Template

```markdown
## Feature Description
[Clear description of the feature]

## Problem
[What problem does this solve?]

## Proposed Solution
[Your proposed solution]

## Alternatives Considered
[Any alternative solutions you considered]

## Additional Context
[Mockups, examples, or references]
```

## Questions?

If you have questions about contributing, feel free to:

1. Open a [Discussion](https://github.com/runkids/skillshare-app/discussions)
2. Check existing issues and discussions

Thank you for contributing to Skillshare App!
