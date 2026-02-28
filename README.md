# epub-to-pdf-html-rust 🦀📖

A high-performance Rust tool designed to transform large EPUB files (500+ pages) into lightweight, searchable PDFs and single-file HTML documents.

## The Problem
Many virtual printers (like PDF24 or standard Print-to-PDF drivers) generate massive temporary spool files—sometimes exceeding 30GB for a single book—due to inefficient rasterization. This tool bypasses the Windows Print Spooler entirely.

## Key Features
- **Dual Output:** Generates both a professional PDF and a single, portable HTML file.
- **High Efficiency:** Uses a headless Microsoft Edge/Chromium engine to render layout without drive-bloat.
- **Image Preservation:** Automatically extracts and embeds images from the EPUB archive.
- **Smart Formatting:** Includes an automatic Table of Contents, page numbering, and serif-font optimization.
- **GUI Interface:** Built with Slint for a native Windows experience with real-time progress tracking.

## Installation & Requirements
- **Rust Toolchain:** Installed via [rustup.rs](https://rustup.rs/).
- **Browser:** Microsoft Edge (standard path) or Google Chrome.
- **OS:** Optimized for Windows.

## Usage
1. Run the application.
2. Click **Choose EPUB** to select your book.
3. (Optional) Change the destination path.
4. Click **Start Transformation**.
5. Find your optimized `.pdf` and `.html` files in the destination folder.

## License
Distributed under the **MIT License**. See `LICENSE` for more information.