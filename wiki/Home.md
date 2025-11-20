# WhatsApp Group Manager - Wiki

Welcome to the WhatsApp Group Manager documentation! This wiki provides comprehensive information for both users and developers.

## Quick Navigation

### For Users
- **[Installation](Installation)** - Step-by-step setup instructions
- **[Usage Guide](Usage-Guide)** - How to use the tool with examples
- **[Troubleshooting](Troubleshooting)** - Common issues and solutions

### For Developers
- **[Developer Guide](Developer-Guide)** - Architecture, code structure, and development workflow
- **[API Reference](API-Reference)** - Library functions and interfaces

## What is WhatsApp Group Manager?

WhatsApp Group Manager is a Rust-based tool that helps you efficiently add multiple members to WhatsApp groups. It includes smart features like:

- Bulk member addition with safe rate limiting
- Automatic retry logic for temporary failures
- Personal invite message fallback when direct adding fails
- Invalid phone number tracking
- Customizable invite messages

## Key Features

✅ **Smart & Safe** - Automatic delays and retry logic to avoid WhatsApp rate limits  
✅ **User-Friendly** - Simple CLI commands with clear output  
✅ **Flexible** - Multiple ways to add members (JSON files, CLI args, or main binary)  
✅ **Reliable** - Comprehensive error handling with automatic fallback to invites  
✅ **Customizable** - Personalize invite messages in any language  

## Getting Started

1. **[Install Rust and build the project](Installation)**
2. **[Find your group's ID](Usage-Guide#step-1-find-your-group-id)**
3. **[Add members to your group](Usage-Guide#step-2-add-members)**

## Requirements

- Rust nightly toolchain
- WhatsApp account
- Group admin rights (recommended but not required)

## License

This is a personal tool. Feel free to fork and modify for your needs.

## Contributing

Found a bug or have a suggestion? This project welcomes contributions! See the [Developer Guide](Developer-Guide) for information on the codebase architecture.
