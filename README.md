# File Icon Provider

**File Icon Provider** is a cross-platform Rust library designed to simplify the retrieval of file icons on Windows and macOS. Whether you need to fetch a specific file's icon or manage icon caching for improved performance, this library has you covered.

## Features

- **Simple Functionality**: Use the `get_file_icon` function to retrieve the icon for a specific file path.
- **Efficient Caching**: Leverage the `FileIconProvider` struct to cache icons based on file extensions, reducing repetitive lookups and improving performance.
- **Cross-Platform Support**: Works seamlessly on both Windows and macOS.