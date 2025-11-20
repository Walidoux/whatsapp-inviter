# API Reference

Technical reference for the WhatsApp Group Manager library functions.

## Module: `member_utils`

Shared utilities for member management operations.

### Types

#### `AddMembersResult`

Result structure for batch member addition operations.

```rust
pub struct AddMembersResult {
    pub success_count: u32,
    pub skip_count: u32,
    pub fail_count: u32,
    pub invalid_phones: Vec<String>,
    pub failed_members: Vec<String>,
}
```

**Fields:**
- `success_count`: Number of successfully added members
- `skip_count`: Number of skipped members (already in group)
- `fail_count`: Number of failed additions
- `invalid_phones`: JIDs with 400 errors (invalid format)
- `failed_members`: JIDs with 403/404 errors (will receive invites)

#### `MemberAddResult`

Result for single member addition.

```rust
pub enum MemberAddResult {
    Success,
    Skipped,
    InvalidPhone,
    Failed,
    RateLimited,
}
```

**Variants:**
- `Success`: Member added (HTTP 200)
- `Skipped`: Already in group (HTTP 409)
- `InvalidPhone`: Bad phone format (HTTP 400)
- `Failed`: Not authorized/not found (HTTP 403/404)
- `RateLimited`: Hit rate limit (HTTP 429)

---

### Functions

#### `add_members_batch`

Process multiple members with automatic retry logic and delays.

```rust
pub async fn add_members_batch(
    client: &Client,
    group_jid: &str,
    phone_numbers: Vec<String>,
) -> AddMembersResult
```

**Parameters:**
- `client`: WhatsApp client instance
- `group_jid`: Group identifier (e.g., `"120363420434676715@g.us"`)
- `phone_numbers`: Vector of phone numbers in international format

**Returns:** `AddMembersResult` with counts and failed members

**Behavior:**
- Processes members sequentially
- 5-second delay between each member
- Automatic retry on 429 errors (up to 2 retries with 30s delay)
- Categorizes results by error code

**Example:**
```rust
let phones = vec![
    "212696552892".to_string(),
    "212906936704".to_string(),
];

let result = add_members_batch(&client, group_jid, phones).await;

println!("Added: {}", result.success_count);
println!("Failed: {}", result.fail_count);
```

---

#### `add_member_with_retry`

Add a single member with retry logic for rate limiting.

```rust
pub async fn add_member_with_retry(
    client: &Client,
    group_jid: &str,
    phone: &str,
    max_retries: u32,
) -> MemberAddResult
```

**Parameters:**
- `client`: WhatsApp client instance
- `group_jid`: Group identifier
- `phone`: Phone number in international format
- `max_retries`: Maximum retry attempts for 429 errors (typically 2)

**Returns:** `MemberAddResult` enum indicating the outcome

**Retry Logic:**
- Only retries on 429 (rate limit) errors
- 30-second delay between retries
- Other errors return immediately

**Example:**
```rust
let result = add_member_with_retry(
    &client,
    "120363420434676715@g.us",
    "212696552892",
    2,
).await;

match result {
    MemberAddResult::Success => println!("Added!"),
    MemberAddResult::RateLimited => println!("Rate limited"),
    _ => println!("Failed"),
}
```

---

#### `finalize_member_addition`

Post-processing after batch addition: print summary, send invites, save invalid phones.

```rust
pub async fn finalize_member_addition(
    client: &Client,
    group_jid: &str,
    result: AddMembersResult,
) -> Result<()>
```

**Parameters:**
- `client`: WhatsApp client instance
- `group_jid`: Group identifier
- `result`: Result from `add_members_batch()`

**Returns:** `Result<()>` - Ok if successful, Err on file I/O errors

**Actions:**
1. Prints summary of results
2. Sends invite messages to failed members
3. Saves invalid phones to `invalid_phones.json`

**Example:**
```rust
let result = add_members_batch(&client, group_jid, phones).await;
finalize_member_addition(&client, group_jid, result).await?;
```

---

#### `send_invite_messages`

Send personal WhatsApp messages with group invite links to members who couldn't be added.

```rust
pub async fn send_invite_messages(
    client: &Client,
    group_jid: &str,
    failed_jids: Vec<String>,
) -> Result<()>
```

**Parameters:**
- `client`: WhatsApp client instance
- `group_jid`: Group identifier
- `failed_jids`: Vector of JIDs that failed direct addition

**Returns:** `Result<()>` - Ok if successful, Err on errors

**Behavior:**
1. Reads `invites_sent.json` to check already-sent invites
2. Fetches group invite link
3. Reads custom message from `message.txt` (or uses default)
4. Sends message to each new failed member
5. 500ms delay between messages
6. Updates `invites_sent.json`

**Message Template:**
- Reads from `message.txt` if exists
- Replaces `{link}` placeholder with actual invite link
- Falls back to default English message if file missing

**Example:**
```rust
let failed = vec![
    "212696552892@s.whatsapp.net".to_string(),
];

send_invite_messages(&client, group_jid, failed).await?;
```

---

#### `save_invalid_phones`

Save invalid phone numbers to JSON file with deduplication.

```rust
pub fn save_invalid_phones(
    invalid_jids: Vec<String>,
    filename: &str,
) -> Result<()>
```

**Parameters:**
- `invalid_jids`: Vector of JIDs with invalid phone format
- `filename`: Output filename (typically `"invalid_phones.json"`)

**Returns:** `Result<()>` - Ok if successful, Err on file I/O errors

**Behavior:**
1. Extracts phone numbers from JIDs
2. Reads existing file (if exists)
3. Merges and deduplicates
4. Writes sorted array to file

**File Format:**
```json
["212686884320", "212638861407", "212681077111"]
```

**Example:**
```rust
let invalid = vec![
    "212686884320@s.whatsapp.net".to_string(),
];

save_invalid_phones(invalid, "invalid_phones.json")?;
```

---

#### `jid_to_phone`

Extract phone number from WhatsApp JID.

```rust
pub fn jid_to_phone(jid: &str) -> String
```

**Parameters:**
- `jid`: WhatsApp JID (e.g., `"212696552892@s.whatsapp.net"` or `"111183835193538@lid"`)

**Returns:** Phone number as string

**Behavior:**
- Splits on `@` and returns first part
- Works with both `@s.whatsapp.net` and `@lid` formats

**Example:**
```rust
let phone = jid_to_phone("212696552892@s.whatsapp.net");
// phone = "212696552892"

let phone = jid_to_phone("111183835193538@lid");
// phone = "111183835193538"
```

---

## Module: `groups`

Group management trait and implementations.

### Trait: `GroupManagement`

Defines operations for WhatsApp group management.

```rust
pub trait GroupManagement {
    async fn get_group_metadata(&self, jid: &str) -> Result<GroupInfo>;
    async fn add_member(&self, group_jid: &str, member_jid: &str) -> Result<()>;
    async fn get_group_invite_link(&self, jid: &str) -> Result<String>;
}
```

#### Methods

##### `get_group_metadata`

Fetch group information.

```rust
async fn get_group_metadata(&self, jid: &str) -> Result<GroupInfo>
```

**Parameters:**
- `jid`: Group JID

**Returns:** `GroupInfo` struct with name, participant count, etc.

---

##### `add_member`

Add a single member to a group (low-level).

```rust
async fn add_member(&self, group_jid: &str, member_jid: &str) -> Result<()>
```

**Parameters:**
- `group_jid`: Group identifier
- `member_jid`: Member JID to add

**Returns:** `Result<()>` - Ok on success, Err with status code on failure

**Note:** Use `add_member_with_retry()` from `member_utils` for higher-level usage with retry logic.

---

##### `get_group_invite_link`

Get shareable invite link for a group.

```rust
async fn get_group_invite_link(&self, jid: &str) -> Result<String>
```

**Parameters:**
- `jid`: Group JID

**Returns:** Invite link URL as string

**Requires:** Admin permissions in the group

---

## Constants

Defined in `member_utils.rs`:

```rust
const MAX_RETRIES: u32 = 2;              // Retry attempts for 429 errors
const RETRY_DELAY_SECS: u64 = 30;        // Delay between retries
const MEMBER_DELAY_SECS: u64 = 5;        // Delay between member additions
const INVITE_DELAY_MS: u64 = 500;        // Delay between invite messages
```

## Error Handling

All functions use `anyhow::Result` for error handling.

### Common Errors

- **Network errors**: Connection issues, timeouts
- **File I/O errors**: Permission denied, file not found
- **WhatsApp errors**: Invalid JID, not authorized, rate limited
- **JSON errors**: Invalid format, parsing failures

### Error Codes

HTTP status codes from WhatsApp API:

| Code | Constant | Meaning |
|------|----------|---------|
| 200 | OK | Success |
| 400 | Bad Request | Invalid phone number |
| 403 | Forbidden | Not authorized (not admin) |
| 404 | Not Found | User not found |
| 409 | Conflict | Already in group |
| 429 | Too Many Requests | Rate limited |

## Usage Examples

### Complete Flow

```rust
use whatsapp_invites::member_utils::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize WhatsApp client
    let client = initialize_client().await?;
    
    // Load phone numbers
    let phones = vec![
        "212696552892".to_string(),
        "212906936704".to_string(),
    ];
    
    let group_jid = "120363420434676715@g.us";
    
    // Add members
    let result = add_members_batch(&client, group_jid, phones).await;
    
    // Handle results
    finalize_member_addition(&client, group_jid, result).await?;
    
    Ok(())
}
```

### Custom Retry Logic

```rust
let result = add_member_with_retry(
    &client,
    group_jid,
    "212696552892",
    3,  // Custom: 3 retries instead of 2
).await;
```

### Manual Invite Sending

```rust
let failed_members = vec![
    "212696552892@s.whatsapp.net".to_string(),
    "212906936704@s.whatsapp.net".to_string(),
];

send_invite_messages(&client, group_jid, failed_members).await?;
```

## Thread Safety

- All async functions are thread-safe
- Client can be shared across tasks with `Arc<Client>`
- File operations use synchronous I/O (blocking)

## Performance Notes

- Sequential processing (not parallel) to respect rate limits
- File I/O is minimal (only at start/end of batch)
- Network calls are the main bottleneck
- Memory usage is O(n) where n is number of phone numbers

## See Also

- [Usage Guide](Usage-Guide) - User-facing documentation
- [Developer Guide](Developer-Guide) - Architecture overview
- [Troubleshooting](Troubleshooting) - Common issues
