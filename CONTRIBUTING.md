# Contributing to Gmail MCP Server

Thank you for your interest in contributing! This project follows clean code principles and welcomes contributions that maintain code quality and simplicity.

## 🎯 Code Philosophy

This project prioritizes:
- **Simplicity** over complexity
- **Readability** over performance (unless performance is critical)
- **Clean architecture** with single responsibility
- **Comprehensive error handling**
- **Minimal dependencies**

## 🚀 Getting Started

1. **Fork** the repository
2. **Clone** your fork:
   ```bash
   git clone https://github.com/your-username/gmail-mcp-server
   cd gmail-mcp-server
   ```
3. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
4. **Build and test**:
   ```bash
   cargo build
   cargo test
   ```

## 📝 Development Guidelines

### Code Quality Standards

- **Use `cargo fmt`** before committing
- **Run `cargo clippy`** and fix all warnings
- **Add tests** for new functionality
- **Keep functions small** (ideally <20 lines)
- **Add comprehensive comments** explaining the "why", not just the "what"

### Commit Standards

- Use clear, descriptive commit messages
- Reference issues when applicable: `Fix authentication issue (#42)`
- Keep commits atomic (one logical change per commit)

### Testing

- Add unit tests for new functions
- Test error conditions and edge cases
- Ensure all tests pass: `cargo test`
- Manual testing with real Gmail API when needed

## 🔧 Pull Request Process

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**:
   - Follow the coding standards above
   - Add tests for new functionality
   - Update documentation if needed

3. **Test thoroughly**:
   ```bash
   cargo test
   cargo clippy
   cargo fmt
   ```

4. **Commit your changes**:
   ```bash
   git add .
   git commit -m "Add: descriptive commit message"
   ```

5. **Push and create PR**:
   ```bash
   git push origin feature/your-feature-name
   ```
   Then create a Pull Request on GitHub.

## 🎯 Areas for Contribution

### High Priority
- **Documentation improvements**
- **Additional email filters**
- **Error handling enhancements**
- **Performance optimizations**

### Medium Priority
- **Additional MCP tools**
- **Configuration options**
- **Logging improvements**

### Low Priority
- **Code refactoring**
- **Dependency updates**

## 🐛 Bug Reports

When reporting bugs, please include:
- **Clear description** of the issue
- **Steps to reproduce**
- **Expected vs actual behavior**
- **Error messages** (if any)
- **Environment details** (OS, Rust version)

## 💡 Feature Requests

For new features:
- **Describe the use case** clearly
- **Explain why** it's needed
- **Consider the impact** on simplicity
- **Provide implementation ideas** (optional)

## 🔒 Security

For security-related issues:
- **DO NOT** open public issues
- **Email maintainers** directly
- **Provide details** about the vulnerability
- **Allow time** for patch development

## 📚 Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [MCP Specification](https://docs.anthropic.com/en/docs/build-with-claude/mcp)
- [Gmail API Documentation](https://developers.google.com/gmail/api)
- [Clean Code Principles](https://blog.cleancoder.com/)

## 🙏 Recognition

Contributors will be:
- **Listed** in project acknowledgments
- **Tagged** in release notes
- **Given attribution** for significant contributions

---

**Happy coding! 🦀** 