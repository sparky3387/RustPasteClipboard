# RustPasteClipboard
A simple rust application for Linux that types text into the currently focused window after a configurable delay. This is useful for pasting text into   applications that don't support standard copy-paste, or for automating simple data entry tasks.

## Features

*   Set custom text to be typed.
*   Configure a delay in seconds before typing starts.
*   Automatically saves your settings (`~/.config/PasteClipboard/config.ini`).
*   Simple and intuitive interface.

## Requirements

Before building, you need to install Rust, GTK4, and the `libxdo` development libraries.

**1. Rust:**
If you don't have Rust installed, get it from [rustup.rs](https://rustup.rs/).

**2. GTK4 & libxdo:**

*   **Debian / Ubuntu:**
    ```bash
    sudo apt-get update
    sudo apt-get install libgtk-4-dev libxdo-dev
    ```

*   **Fedora / CentOS / RHEL:**
    ```bash
    sudo dnf install gtk4-devel libxdo-devel
    ```

*   **Arch Linux:**
    ```bash
    sudo pacman -Syu gtk4 libxdo
    ```

## Building and Running

1.  Clone the repository:
    ```bash
    git clone <repository-url>
    cd pasteclipboard
    ```

2.  Build the application in release mode:
    ```bash
    cargo build --release
    ```

3.  Run the executable:
    ```bash
    ./target/release/paste_clipboard
    ```

## Usage

1.  Launch the application.
2.  Enter the text you want to be typed into the main text area.
3.  Set the delay in seconds.
4.  Click the "Type After Delay" button.
5.  Quickly switch to and focus the window where you want the text to be typed.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
=======
