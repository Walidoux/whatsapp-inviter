# WhatsApp Group Manager

Easily add multiple members to your WhatsApp groups with automatic invite fallback and smart error handling.

## What does it do?

This tool helps you:
- Add multiple people to a WhatsApp group at once
- Automatically send invite links when direct adding fails
- Handle WhatsApp's rate limits gracefully
- Track invalid phone numbers

## Quick Start

### Step 1: Install Rust

```bash
# Install Rust (if you don't have it)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install nightly
```

### Step 2: Get Your Group ID

```bash
cargo +nightly run --example get_group_jid
```

Scan the QR code, then send a message in your group. The tool will show your Group ID.

### Step 3: Add Members

Create a file called `phones.json` with phone numbers (international format, no `+`):

```json
["212696552892", "212906936704", "212651660005"]
```

Run:

```bash
cargo +nightly run --example add_members "YOUR_GROUP_ID" phones.json
```

That's it! The tool will add members with safe delays and send invites when needed.

## Features

✅ **Bulk member addition** - Add many people at once  
✅ **Smart retry logic** - Automatically handles rate limits  
✅ **Invite fallback** - Sends personal invites when direct add fails  
✅ **Customizable messages** - Create `message.txt` to personalize invites  
✅ **Error tracking** - Saves invalid numbers automatically  

## Important Notes

- You need to be a **group admin** to add members directly
- Use phone numbers **without** the `+` sign (e.g., `212696552892`)
- Be careful with rate limits: add max 20-30 members per day
- The tool waits 5 seconds between each member to stay safe

## Need Help?

See our [Wiki](../../wiki) for:
- [Installation Guide](../../wiki/Installation)
- [Usage Guide](../../wiki/Usage-Guide)
- [Troubleshooting](../../wiki/Troubleshooting)
- [Developer Guide](../../wiki/Developer-Guide)

## License

Personal tool - feel free to fork and modify!
