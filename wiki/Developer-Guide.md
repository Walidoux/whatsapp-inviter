# Developer Guide

Technical documentation for developers working on WhatsApp Group Manager.

## Architecture Overview

WhatsApp Group Manager is built with a modular architecture that separates concerns and promotes code reuse.

### Project Structure

```
whatsapp-invites/
├── src/
│   ├── lib.rs              # Library exports & public API
│   ├── main.rs             # Main binary (uses phones.json)
│   ├── groups.rs           # Group management trait
│   └── member_utils.rs     # Shared utilities
├── examples/
│   ├── get_group_jid.rs    # Find group JIDs
│   ├── add_members.rs      # Add from JSON file
│   └── add_members_args.rs # Add from CLI args
├── phones.json             # Phone numbers to add
├── message.txt             # Custom invite message (optional)
├── invalid_phones.json     # Auto-generated invalid numbers
├── invites_sent.json       # Auto-generated invite tracking
└── whatsapp.db             # WhatsApp session data
```

## Core Components

### 1. `groups.rs` - Group Management Trait

Defines the `GroupManagement` trait for interacting with WhatsApp groups.

**Key Methods:**
- `get_group_metadata()` - Fetch group name, participants, etc.
- `add_member()` - Add a single member to a group
- `get_group_invite_link()` - Get shareable invite link

### 2. `member_utils.rs` - Shared Utilities

Centralized utilities used across all entry points. This promotes DRY principles and consistency.

**Key Functions:**

#### `add_members_batch()`
Process multiple members with retry logic
```rust
pub async fn add_members_batch(
    client: &Client,
    group_jid: &str,
    phone_numbers: Vec<String>,
) -> AddMembersResult
```

#### `add_member_with_retry()`
Add single member with 429 retry handling
```rust
pub async fn add_member_with_retry(
    client: &Client,
    group_jid: &str,
    phone: &str,
    max_retries: u32,
) -> MemberAddResult
```

#### `finalize_member_addition()`
Print summary, send invites, save invalid phones
```rust
pub async fn finalize_member_addition(
    client: &Client,
    group_jid: &str,
    result: AddMembersResult,
) -> Result<()>
```

#### `send_invite_messages()`
Send personal WhatsApp messages with invite links
```rust
pub async fn send_invite_messages(
    client: &Client,
    group_jid: &str,
    failed_jids: Vec<String>,
) -> Result<()>
```

#### `save_invalid_phones()`
Save to JSON with deduplication
```rust
pub fn save_invalid_phones(
    invalid_jids: Vec<String>,
    filename: &str,
) -> Result<()>
```

#### `jid_to_phone()`
Extract phone number from JID
```rust
pub fn jid_to_phone(jid: &str) -> String
```

**Benefits:**
- Fix bugs once, apply everywhere
- Consistent behavior across all entry points
- Easier to test and maintain
- 44% code reduction!

### 3. Entry Points

#### Main Binary (`src/main.rs`)
Reads from `phones.json` in current directory:
```bash
cargo +nightly run "GROUP_JID"
```

#### Example: Get Group JID (`examples/get_group_jid.rs`)
Listens for group messages and displays JID:
```bash
cargo +nightly run --example get_group_jid
```

#### Example: Add Members (`examples/add_members.rs`)
Adds members from a JSON file:
```bash
cargo +nightly run --example add_members "GROUP_JID" phones.json
```

#### Example: Add Members Args (`examples/add_members_args.rs`)
Adds members from CLI arguments:
```bash
cargo +nightly run --example add_members_args "GROUP_JID" 212123456 212789012
```

## Error Handling Flow

```
Member Addition Attempt
    │
    ├─> Success (200)
    │   └─> Return Success
    │
    ├─> Rate Limited (429)
    │   ├─> Wait 30 seconds
    │   ├─> Retry (up to 2 times)
    │   └─> Still failing? Return RateLimited
    │
    ├─> Invalid Phone (400)
    │   └─> Return InvalidPhone
    │
    ├─> Not Authorized / Not Found (403/404)
    │   └─> Return Failed (will send invite)
    │
    └─> Already Member (409)
        └─> Return Skipped
```

## Rate Limiting Strategy

### Built-in Delays

1. **Between members**: 5 seconds (hardcoded)
2. **After rate limit**: 30 seconds before retry
3. **Between invites**: 500ms

### Retry Logic

```rust
max_retries: 2
delay_on_429: 30 seconds

for attempt in 1..=max_retries {
    match add_member(client, group_jid, phone).await {
        429 => {
            if attempt < max_retries {
                sleep(30s).await;
                continue;
            }
            return RateLimited;
        }
        _ => return result
    }
}
```

## WhatsApp Integration

Uses `whatsmeow` library for WhatsApp Web API.

### Key Concepts

**JID (Jabber ID)**: WhatsApp's internal identifier format
- Groups: `120363420434676715@g.us`
- Users: `212696552892@s.whatsapp.net`
- LID: `111183835193538@lid`

**Session Persistence**: 
- Stored in `whatsapp.db` using SQLite
- QR code scan required on first run only
- Subsequent runs reuse session

## Data Persistence

### Session Data (`whatsapp.db`)
- Binary SQLite database
- Contains encryption keys and session info
- Managed automatically by `whatsmeow`

### Invalid Phones (`invalid_phones.json`)
```json
["212686884320", "212638861407"]
```
- JSON array of phone numbers
- Deduplicates on write
- Append-only (never deletes)

### Invites Sent (`invites_sent.json`)
```json
["212696552892", "212906936704"]
```
- JSON array of phone numbers
- Prevents duplicate invites
- Append-only (never deletes)

### Custom Message (`message.txt`)
```txt
Bonsoir 👋 ! Voici le lien : {link}
```
- Plain text file
- `{link}` placeholder replaced at runtime
- Falls back to default if missing

## Building

### Debug Build
```bash
cargo +nightly build
```
- Fast compilation
- Includes debug symbols
- Slower execution

### Release Build
```bash
cargo +nightly build --release
```
- Slower compilation
- Optimized binary
- Much faster execution

### Build Examples
```bash
cargo +nightly build --examples
```

### Clean Build
```bash
cargo clean
cargo +nightly build
```

## Testing

### Run Tests
```bash
cargo +nightly test
```

### With Logging
```bash
RUST_LOG=debug cargo +nightly test
```

## Common Development Tasks

### Adding a New Feature

1. **Update `member_utils.rs`** if it's shared functionality
2. **Update relevant entry points** (main.rs, examples)
3. **Test with small datasets** first
4. **Update documentation** (this wiki!)

### Debugging Issues

Enable debug logging:
```bash
RUST_LOG=debug cargo +nightly run --example add_members "GROUP_JID" phones.json
```

### Modifying Retry Logic

Edit `member_utils.rs`:
```rust
// Current: 2 retries with 30s delay
const MAX_RETRIES: u32 = 2;
const RETRY_DELAY_SECS: u64 = 30;
```

### Changing Delays

Edit delays in `member_utils.rs`:
```rust
// Between member additions
const MEMBER_DELAY_SECS: u64 = 5;

// Between invite messages
const INVITE_DELAY_MS: u64 = 500;
```

## Dependencies

Key dependencies in `Cargo.toml`:

```toml
[dependencies]
whatsmeow = "..."        # WhatsApp Web API
tokio = { version = "...", features = ["full"] }  # Async runtime
serde = "..."            # JSON serialization
serde_json = "..."       # JSON parsing
anyhow = "..."           # Error handling
```

## Code Style

- Use `async/await` for IO operations
- Prefer `Result<T>` over panics
- Use `anyhow::Result` for errors
- Add comments for complex logic only
- Follow Rust naming conventions

## Performance Considerations

### Memory
- Small phone lists: <1 MB memory
- Large lists (1000+): ~10-50 MB

### Network
- Each member add: 1 API call
- Each invite: 2 API calls (get link + send message)
- Session setup: Multiple API calls

### Bottlenecks
- Rate limiting (intentional delays)
- Network latency
- WhatsApp API response time

## Security Considerations

### Session Data
- `whatsapp.db` contains encryption keys
- **Never commit to git**
- Add to `.gitignore`

### Phone Numbers
- Considered PII (personally identifiable information)
- **Never commit real phone numbers to git**
- Use dummy data in examples

### API Keys
- None required (uses WhatsApp Web protocol)
- Session keys stored locally only

## Troubleshooting Development Issues

### Compilation Errors

```bash
# Clean and rebuild
cargo clean
cargo +nightly build
```

### Runtime Panics

```bash
# Run with backtrace
RUST_BACKTRACE=1 cargo +nightly run --example add_members "GROUP_JID" phones.json
```

### Connection Issues

- Check internet connection
- Verify WhatsApp account is active
- Delete `whatsapp.db*` and re-scan QR code

## Contributing

### Before Submitting PR

1. Run tests: `cargo +nightly test`
2. Check formatting: `cargo fmt --check`
3. Run clippy: `cargo clippy`
4. Test with real WhatsApp account
5. Update documentation

### Code Review Checklist

- [ ] Follows existing code style
- [ ] Includes error handling
- [ ] Uses shared utilities when possible
- [ ] Respects rate limits
- [ ] No hardcoded credentials
- [ ] Documentation updated

## Future Enhancements

Potential improvements:

- [ ] Add unit tests for utilities
- [ ] Support for removing members
- [ ] Export group member lists
- [ ] GUI interface
- [ ] Configuration file (delays, retries, etc.)
- [ ] Progress bar for large operations
- [ ] Parallel processing (with careful rate limiting)

## Resources

- [whatsmeow GitHub](https://github.com/tulir/whatsmeow)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [Tokio Documentation](https://tokio.rs/)
