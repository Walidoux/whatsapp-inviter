# WhatsApp Group Member Management Tool

A Rust-based tool for managing WhatsApp group members, including finding group information, adding members in bulk, and querying group metadata.

## ✨ Features

- 🔍 **Find Group JID and Name**: Discover group identifiers and names by listening to messages
- 👥 **Bulk Member Addition**: Add multiple members to groups one-by-one
- 📊 **Group Metadata Query**: Fetch group name, participant count, and other details
- 🔄 **Smart Retry Logic**: Automatically retries rate-limited additions (429 errors)
- ⏱️ **Rate Limiting Protection**: 5-second delays between additions + 30s retry waits
- 📧 **Invite Message Fallback**: Sends personal messages for 403/404 errors
- 🗂️ **Invalid Phone Tracking**: Saves 400 errors to `invalid_phones.json`
- 📚 **Shared Library**: DRY code architecture with reusable utilities

## 📋 Requirements

- Rust nightly toolchain
- WhatsApp account
- Admin rights in the target group (for direct additions)

## 🚀 Installation

```bash
# Install Rust nightly if not already installed
rustup install nightly

# Clone the repository
cd /path/to/whatsapp-invites

# Build the project
cargo +nightly build --release
```

## 📖 Usage

### 1. Find Group JID and Name

Discover your group's JID and name by listening to messages:

```bash
cargo +nightly run --example get_group_jid
```

**Steps:**

1. Scan the QR code with WhatsApp
2. Send a message to any group (or wait for incoming messages)
3. The tool will display:
   - **Group Name** (e.g., "EPIK LEADER'S TEAM")
   - **Participant Count** (e.g., 24 members)
   - **Group JID** (e.g., `120363420434676715@g.us`)

**Example Output:**

```bash
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📱 GROUP MESSAGE RECEIVED!
   Group Name: EPIK LEADER'S TEAM
   Participants: 24
   Group JID: 120363420434676715@g.us
   Sender: 1234567890@s.whatsapp.net
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ To add members to this group, run:
   cargo +nightly run --example add_members 120363420434676715@g.us phones.json
```

---

### 2. Add Members from JSON File

Add multiple members using a JSON file containing phone numbers.

**Create `phones.json`:**

```json
["212696552892", "212906936704", "212651660005"]
```

**Run:**

```bash
cargo +nightly run --example add_members "120363420434676715@g.us" phones.json
```

**Features:**

- Reads phone numbers from JSON file
- Adds members one by one with 5-second delays
- Automatically retries rate-limited requests (429 errors)
- Sends invite messages for 403/404 errors
- Saves invalid numbers (400 errors) to `invalid_phones.json`

---

### 3. Add Members from CLI Arguments

Add members directly from command-line arguments (no JSON file needed).

```bash
cargo +nightly run --example add_members_args "120363420434676715@g.us" 212651660005 212696552892
```

**Features:**

- Quick one-off additions
- Same retry logic and error handling as JSON method
- Perfect for shell scripting

---

### 4. Use Main Binary

The main binary reads from `phones.json` in the current directory:

```bash
cargo +nightly run "120363420434676715@g.us"
```

## 🎯 How It Works

### Member Addition Flow

```json
For each phone number:
  │
  ├─> Try to add member
  │   │
  │   ├─> ✅ Success → Continue to next
  │   │
  │   ├─> ⚠️  429 (Rate Limit) → Wait 30s, retry (up to 2 times)
  │   │
  │   ├─> ❌ 400 (Bad Request) → Save to invalid_phones.json
  │   │
  │   ├─> ❌ 403/404 (Not Authorized/Not Found) → Send invite message
  │   │
  │   └─> ❌ 409 (Already in Group) → Skip
  │
  └─> Wait 5 seconds before next member
```

### Error Handling

| Error Code | Meaning          | Action Taken                          |
| ---------- | ---------------- | ------------------------------------- |
| **200**    | Success          | Member added ✅                       |
| **400**    | Bad request      | Saved to `invalid_phones.json`        |
| **403**    | Not authorized   | Sends personal invite message         |
| **404**    | User not found   | Sends personal invite message         |
| **409**    | Already in group | Skipped (expected)                    |
| **429**    | Rate limited     | Waits 30s and retries (up to 2 times) |

## 📁 Generated Files

### `invalid_phones.json`

Contains phone numbers that returned 400 errors (bad request):

```json
["212686884320", "212638861407", "212681077111"]
```

**Features:**

- Automatically created when 400 errors occur
- Appends to existing file without duplicates
- Helps clean your phone number list over time

## 🔒 WhatsApp Rate Limits

Based on testing and research:

- **Threshold**: ~8-10 member additions in a short time
- **Ban duration**: 30-60 minutes (soft ban)
- **Symptoms**: Immediate 429 errors on every attempt
- **Recovery**: Wait 30-60 minutes before trying again

### Recommendations

1. **Daily limit**: Add maximum 20-30 members per day
2. **Spacing**: Use 5-second delays (already implemented)
3. **Batches**: Process in sessions of 5-10 members
4. **Between sessions**: Wait 2-4 hours
5. **If rate limited**: Stop immediately, wait 30-60 minutes

### Safe Usage Pattern

```bash
# Session 1 (Morning)
cargo +nightly run --example add_members "GROUP_JID" batch1.json  # 5 members

# Wait 4 hours...

# Session 2 (Afternoon)
cargo +nightly run --example add_members "GROUP_JID" batch2.json  # 5 members

# Wait 4 hours...

# Session 3 (Evening)
cargo +nightly run --example add_members "GROUP_JID" batch3.json  # 5 members
```

## 🏗️ Architecture

### Project Structure

```bash
whatsapp-invites/
├── src/
│   ├── lib.rs              # Library exports
│   ├── main.rs             # Main binary (uses phones.json)
│   ├── groups.rs           # Group management trait
│   └── member_utils.rs     # Shared utilities (NEW!)
├── examples/
│   ├── get_group_jid.rs    # Find group JIDs
│   ├── add_members.rs      # Add from JSON file
│   └── add_members_args.rs # Add from CLI args
├── phones.json             # Phone numbers to add
├── invalid_phones.json     # Auto-generated invalid numbers
└── whatsapp.db            # WhatsApp session data
```

### Shared Library (`member_utils.rs`)

All examples use shared utilities for consistency:

- `add_members_batch()` - Process multiple members with retry logic
- `add_member_with_retry()` - Add single member with 429 retry handling
- `finalize_member_addition()` - Print summary, send invites, save invalid phones
- `send_invite_messages()` - Send personal WhatsApp messages
- `save_invalid_phones()` - Save to JSON with deduplication
- `jid_to_phone()` - Extract phone from JID

**Benefits:**

- Fix bugs once, apply everywhere
- Consistent behavior across all entry points
- Easier to test and maintain
- 44% code reduction!

## 🛠️ Development

### Build

```bash
# Debug build
cargo +nightly build

# Release build (faster)
cargo +nightly build --release

# Build all examples
cargo +nightly build --examples
```

### Run Tests

```bash
# Run tests (if any)
cargo +nightly test

# Run with logging
RUST_LOG=debug cargo +nightly run --example get_group_jid
```

### Clean Build

```bash
cargo clean
cargo +nightly build
```

## ⚠️ Important Notes

### Session Persistence

- Session data is stored in `whatsapp.db`
- QR code authentication required on first run
- Subsequent runs reuse the saved session
- Delete `whatsapp.db*` files to start fresh

### Phone Number Format

Use international format **without** the `+` sign:

- ✅ Correct: `212696552892`
- ❌ Wrong: `+212696552892`
- ❌ Wrong: `0696552892`

### Group JID Format

Group JIDs always end with `@g.us`:

- ✅ Example: `120363420434676715@g.us`

### Admin Rights

You must be an **admin** of the group to add members directly. If you're not an admin:

- You'll get 403 errors
- Tool will automatically send invite messages instead

## 🐛 Troubleshooting

### Error: "429 Too Many Requests"

**Solution**: Wait 30-60 minutes. You've hit WhatsApp's rate limit.

### Error: "403 Not Authorized"

**Causes:**

- You're not an admin
- User has privacy settings restricting group additions

**Solution**: Tool automatically sends invite messages.

### Error: "400 Bad Request"

**Cause**: Invalid phone number

**Solution**: Number is saved to `invalid_phones.json`. Remove it from your list.

### Error: "404 User Not Found"

**Causes:**

- Phone number doesn't have WhatsApp
- User has strict privacy settings

**Solution**: Tool automatically sends invite messages.

### No Messages Appearing in `get_group_jid`

**Cause**: Old offline messages aren't displayed

**Solution**: Send a new message to the group while the tool is running

## 📄 License

See LICENSE file for details.

## 🤝 Contributing

This is a personal tool. Feel free to fork and modify for your needs.

## ⚡ Quick Reference

```bash
# Find group JID
cargo +nightly run --example get_group_jid

# Add members from JSON
cargo +nightly run --example add_members "GROUP_JID" phones.json

# Add members from CLI
cargo +nightly run --example add_members_args "GROUP_JID" 212123456 212789012

# Use main binary
cargo +nightly run "GROUP_JID"
```

## 📊 Example Output

```bash
=== Group Information ===
Group Name: EPIK LEADER'S TEAM
Current Participants: 24
Group JID: 120363420434676715@g.us

Adding 7 members one by one (5s delay between each)...

=== Adding member 1/7 ===
✓ Successfully added: 111183835193538@lid
Waiting 5 seconds before next member...

=== Adding member 2/7 ===
✗ Failed to add: 181982478766289@lid (error code: Some(403))
   → Not authorized (you may not be an admin - will send invite message)
Waiting 5 seconds before next member...

=== Final Summary ===
✓ Successfully added: 5
✗ Failed: 2
Total processed: 7

=== Sending Invite Messages ===
Sending invite messages to 2 members who couldn't be added directly

📧 Sent invite message to 181982478766289@lid
📧 Sent invite message to 194729706844198@lid

📝 Saved 0 invalid phone numbers to invalid_phones.json
```

---
