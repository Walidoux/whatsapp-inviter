# Installation Guide

Complete instructions for installing and setting up WhatsApp Group Manager.

## Prerequisites

You'll need:
- A computer running Linux, macOS, or Windows
- WhatsApp account with access to a smartphone
- Admin rights in your target WhatsApp group (recommended)

## Step 1: Install Rust

### Linux or macOS

Open a terminal and run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the on-screen instructions. After installation, restart your terminal or run:

```bash
source $HOME/.cargo/env
```

### Windows

Download and run [rustup-init.exe](https://rustup.rs/) from the Rust website, then follow the installer instructions.

## Step 2: Install Rust Nightly

This project requires Rust nightly features:

```bash
rustup install nightly
```

## Step 3: Clone or Download the Project

```bash
# If using git
git clone <your-repository-url>
cd whatsapp-invites

# Or download and extract the ZIP file, then navigate to the folder
cd /path/to/whatsapp-invites
```

## Step 4: Build the Project

Build the release version for optimal performance:

```bash
cargo +nightly build --release
```

This may take a few minutes on the first build.

## Step 5: Verify Installation

Test that everything works:

```bash
cargo +nightly run --example get_group_jid
```

You should see a prompt to scan a QR code with WhatsApp. If you see this, the installation is successful!

## Optional: Build All Examples

To compile all available examples:

```bash
cargo +nightly build --examples
```

## Troubleshooting Installation

### "rustup: command not found"

Make sure you've restarted your terminal after installing Rust, or run:

```bash
source $HOME/.cargo/env
```

### "cargo: command not found"

Same as above - restart your terminal or source the environment.

### Build Errors

Try cleaning and rebuilding:

```bash
cargo clean
cargo +nightly build --release
```

### Slow Build Times

The first build takes longer as it downloads and compiles dependencies. Subsequent builds will be much faster.

## Next Steps

Once installed, proceed to the [Usage Guide](Usage-Guide) to start using the tool!
