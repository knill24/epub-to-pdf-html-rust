# Contributing to epub-to-pdf-html-rust 🦀

First off, thank you for considering contributing! It’s people like you who make the Rust community such a great place to build tools.

## How Can I Contribute?

### Reporting Bugs
- Check the **Issues** tab to see if the bug has already been reported.
- If not, open a new issue. Include your OS version, the version of Microsoft Edge/Chrome you are using, and (if possible) the EPUB file that caused the crash.

### Suggesting Enhancements
I am always looking to make this tool faster and more compatible! Some ideas I'm currently exploring:
- Support for Firefox/Gecko engines.
- Batch processing (multiple EPUBs at once).
- Custom CSS injection for the final PDF.

### Pull Requests
1. **Fork** the repository.
2. **Clone** your fork to your local machine.
3. Create a **Branch** for your feature (`git checkout -b feature/AmazingFeature`).
4. **Commit** your changes (`git commit -m 'Add some AmazingFeature'`).
5. **Push** to the branch (`git push origin feature/AmazingFeature`).
6. Open a **Pull Request**.



## Technical Standards
- **Rustfmt:** Please run `cargo fmt` before committing to keep the code clean.
- **Safety:** Avoid `unsafe` blocks unless absolutely necessary for performance.
- **UI:** If you are modifying the Slint files, ensure the layout remains responsive for different window sizes.

## License
By contributing, you agree that your contributions will be licensed under the project's **MIT License**.