# Usage Guide

Complete guide on how to use WhatsApp Group Manager.

## Overview

There are three main ways to use this tool:

1. **Get Group JID** - Find your group's identifier
2. **Add Members from JSON** - Bulk add from a file
3. **Add Members from CLI** - Quick one-off additions

## Step 1: Find Your Group ID

Every WhatsApp group has a unique identifier (JID). You need this to add members.

### Command

```bash
cargo +nightly run --example get_group_jid
```

### Steps

1. Run the command above
2. Scan the QR code with WhatsApp (WhatsApp > Settings > Linked Devices)
3. Send a message to your target group (or wait for someone else to send a message)
4. The tool will display the group information

### Example Output

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

**Note:** Copy the Group JID (e.g., `120363420434676715@g.us`) - you'll need it for the next step.

## Step 2: Add Members

### Method 1: Add from JSON File (Recommended)

Best for adding many members at once.

#### 1. Create a JSON file

Create a file named `phones.json` (or any name you prefer) with phone numbers:

```json
["212696552892", "212906936704", "212651660005"]
```

**Important:** Use international format **without** the `+` sign:
- ✅ Correct: `212696552892` (Morocco)
- ✅ Correct: `33612345678` (France)
- ✅ Correct: `447911123456` (UK)
- ❌ Wrong: `+212696552892`
- ❌ Wrong: `0696552892` (local format)

#### 2. Run the command

```bash
cargo +nightly run --example add_members "120363420434676715@g.us" phones.json
```

Replace `120363420434676715@g.us` with your group's JID.

### Method 2: Add from Command Line

Quick method for adding a few members without creating a file.

```bash
cargo +nightly run --example add_members_args "120363420434676715@g.us" 212651660005 212696552892
```

Just list phone numbers as arguments (space-separated).

### Method 3: Use Main Binary

Uses `phones.json` from the current directory by default:

```bash
cargo +nightly run "120363420434676715@g.us"
```

## Understanding the Process

### What Happens When You Add Members

```
For each phone number:
  │
  ├─> Try to add member directly
  │   │
  │   ├─> ✅ Success → Continue to next
  │   │
  │   ├─> ⚠️  429 (Rate Limit) → Wait 30s, retry (up to 2 times)
  │   │
  │   ├─> ❌ 400 (Bad Request) → Save to invalid_phones.json
  │   │
  │   ├─> ❌ 403/404 (Not Authorized) → Send invite message
  │   │
  │   └─> ⊘ 409 (Already in Group) → Skip (counted separately)
  │
  └─> Wait 5 seconds before next member
```

### Error Handling

| Error Code | Meaning | What Happens |
|------------|---------|--------------|
| **200** | Success | Member added directly ✅ |
| **400** | Invalid phone | Saved to `invalid_phones.json` |
| **403** | Not authorized | Sends personal invite message |
| **404** | User not found | Sends personal invite message |
| **409** | Already in group | Skipped (not an error) |
| **429** | Rate limited | Waits 30s and retries up to 2 times |

## Example Output

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
⊘ Skipped: 1
✗ Failed: 1
Total processed: 7

=== Sending Invite Messages ===
Sending invite messages to 1 new members (1 already sent)

📧 Sent invite message to 181982478766289@lid
⊘ Skipped invite to 194729706844198@lid (already sent)

📝 Saved 0 invalid phone numbers to invalid_phones.json
```

## Customizing Invite Messages

When members can't be added directly (403/404 errors), the tool sends them a personal WhatsApp message with a group invite link.

### Default Message

If you don't customize it, members receive:

```
Hi! You've been invited to join our WhatsApp group.

Join here: {link}

If the link doesn't work, please contact an admin.
```

### Custom Message

Create a `message.txt` file in the project root:

```txt
Bonsoir 👋 ! Je suis Walid Korchi, membre de l'équipe du club d'Epik Leaders. 

Voici votre invitation à rejoindre le groupe : {link}

À bientôt !
```

**Placeholder:**
- `{link}` - Automatically replaced with the group's invite link

**Supports:**
- Any language (English, French, Arabic, emoji, etc.)
- Multiple lines
- Custom formatting

## Generated Files

The tool automatically creates and manages these files:

### `invalid_phones.json`

Contains phone numbers that returned 400 errors (invalid format):

```json
["212686884320", "212638861407"]
```

- Automatically created when errors occur
- No duplicates
- Clean these from your main list

### `invites_sent.json`

Tracks who already received invite messages:

```json
["212696552892", "212906936704"]
```

- Prevents duplicate invites
- Persists across runs
- Delete entries to resend invites

### `whatsapp.db`

WhatsApp session data:
- Stores your login session
- No need to scan QR code on every run
- Delete `whatsapp.db*` files to start fresh

## Best Practices

### Rate Limiting Safety

WhatsApp has rate limits. Follow these guidelines:

1. **Daily limit**: Max 20-30 members per day
2. **Spacing**: Tool automatically waits 5 seconds between members
3. **Batches**: Process 5-10 members at a time
4. **Between sessions**: Wait 2-4 hours
5. **If rate limited**: Stop immediately, wait 30-60 minutes

### Safe Usage Pattern

```bash
# Morning - Add 5 members
cargo +nightly run --example add_members "GROUP_JID" batch1.json

# Wait 4 hours...

# Afternoon - Add 5 more
cargo +nightly run --example add_members "GROUP_JID" batch2.json

# Wait 4 hours...

# Evening - Add 5 more
cargo +nightly run --example add_members "GROUP_JID" batch3.json
```

### Tips

- **Be a group admin** for best results (direct adding works better)
- **Clean your phone list** using `invalid_phones.json` feedback
- **Don't rush** - respect rate limits to avoid temporary bans
- **Monitor output** - check what's succeeding vs. failing

## Advanced Usage

### With Logging

Enable detailed logging:

```bash
RUST_LOG=debug cargo +nightly run --example add_members "GROUP_JID" phones.json
```

### Using Release Build

For faster execution (especially with large lists):

```bash
cargo +nightly build --release
./target/release/examples/add_members "GROUP_JID" phones.json
```

## Next Steps

- Having issues? See [Troubleshooting](Troubleshooting)
- Want to understand the code? See [Developer Guide](Developer-Guide)
