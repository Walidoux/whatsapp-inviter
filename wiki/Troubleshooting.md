# Troubleshooting

Solutions to common issues when using WhatsApp Group Manager.

## Installation Issues

### "rustup: command not found"

**Problem:** Rust is not installed or not in PATH.

**Solution:**
1. Make sure Rust is installed: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Restart your terminal
3. Or run: `source $HOME/.cargo/env`

### "cargo: command not found"

**Problem:** Same as above - Rust toolchain not in PATH.

**Solution:**
- Restart terminal after installation
- Or: `source $HOME/.cargo/env`
- Verify: `cargo --version`

### Build Fails with Errors

**Problem:** Compilation errors or dependency issues.

**Solution:**
```bash
# Clean everything and rebuild
cargo clean
cargo +nightly build --release

# If still failing, update Rust
rustup update nightly
cargo +nightly build --release
```

## Connection & Authentication Issues

### QR Code Won't Scan

**Problem:** QR code doesn't appear or won't scan.

**Solution:**
1. Make sure you're connected to the internet
2. Check that WhatsApp is up to date on your phone
3. Try deleting session files and rescanning:
   ```bash
   rm whatsapp.db*
   cargo +nightly run --example get_group_jid
   ```

### "Connection Lost" / "Disconnected"

**Problem:** Tool loses connection to WhatsApp.

**Solution:**
1. Check your internet connection
2. Make sure WhatsApp Web is not logged in elsewhere
3. Try restarting the tool
4. If persistent, delete session and re-login:
   ```bash
   rm whatsapp.db*
   cargo +nightly run --example get_group_jid
   ```

### Session Expired

**Problem:** "Session expired" or "Invalid session" error.

**Solution:**
```bash
# Delete session files and scan QR code again
rm whatsapp.db*
cargo +nightly run --example get_group_jid
```

## Group Discovery Issues

### No Messages Appearing in `get_group_jid`

**Problem:** Running `get_group_jid` but no group info shows up.

**Solution:**
- Old/offline messages aren't displayed
- **Send a NEW message** to the group while the tool is running
- Or wait for someone else to send a message

### Wrong Group JID

**Problem:** Got the JID for the wrong group.

**Solution:**
- Send a message in the CORRECT group while `get_group_jid` is running
- The tool shows JID for every group message it receives
- Make sure you're in the right group

## Member Addition Errors

### Error 400: Bad Request

**Error Message:** `Failed to add: PHONE_NUMBER (error code: 400)`

**Causes:**
- Invalid phone number format
- Non-existent phone number

**Solution:**
- Check phone number format (international without `+`)
- Examples:
  - ✅ `212696552892` (Morocco)
  - ✅ `33612345678` (France)
  - ❌ `+212696552892` (has +)
  - ❌ `0696552892` (local format)
- Number is saved to `invalid_phones.json` - remove it from your list

### Error 403: Not Authorized

**Error Message:** `Failed to add: PHONE_NUMBER (error code: 403)`

**Causes:**
- You're not a group admin
- User has privacy settings that prevent being added to groups

**Solution:**
- **Check admin status:** Make sure you're an admin of the group
- **Automatic fallback:** Tool will send personal invite message
- User can join via invite link

### Error 404: User Not Found

**Error Message:** `Failed to add: PHONE_NUMBER (error code: 404)`

**Causes:**
- Phone number doesn't have WhatsApp
- User has strict privacy settings
- User blocked you

**Solution:**
- **Automatic fallback:** Tool sends personal invite message
- Verify the phone number is correct
- User may need to accept your chat first

### Error 409: Already in Group

**Error Message:** `Skipped: PHONE_NUMBER (already in group)`

**Not an error!** User is already a member. Tool continues with next number.

### Error 429: Too Many Requests (Rate Limited)

**Error Message:** `Rate limited: PHONE_NUMBER (error code: 429)`

**Cause:** You've hit WhatsApp's rate limit.

**Solution:**
1. **Stop immediately** - don't continue adding
2. **Wait 30-60 minutes** before trying again
3. **Follow safe limits:**
   - Max 20-30 members per day
   - Process in batches of 5-10
   - Wait 2-4 hours between batches

**Prevention:**
- Don't add too many members at once
- The tool already waits 5 seconds between each member
- Use the safe usage pattern from the [Usage Guide](Usage-Guide#safe-usage-pattern)

### Rate Limit Symptoms

Signs you've been rate limited:
- Every member returns 429 immediately
- No members being added successfully
- Even retries fail with 429

**Recovery:**
- **Stop the tool immediately**
- Wait 30-60 minutes
- Try adding 1-2 members as a test
- If successful, continue slowly

## File Issues

### "File not found: phones.json"

**Problem:** Tool can't find the phone numbers file.

**Solution:**
1. Make sure `phones.json` exists in the current directory
2. Check filename spelling (case-sensitive on Linux/Mac)
3. Or specify full path:
   ```bash
   cargo +nightly run --example add_members "GROUP_JID" /full/path/to/phones.json
   ```

### "Invalid JSON format"

**Problem:** JSON file is malformed.

**Solution:**
Check your JSON syntax:
```json
["212696552892", "212906936704"]
```

Common mistakes:
- ❌ Missing quotes: `[212696552892]`
- ❌ Missing commas: `["212696552892" "212906936704"]`
- ❌ Extra comma: `["212696552892", "212906936704",]`

Use a JSON validator online to check your file.

### Can't Write to Generated Files

**Problem:** Permission denied writing to `invalid_phones.json` or `invites_sent.json`

**Solution:**
- Check file permissions
- Make sure you have write access to the directory
- Try running from a different directory

## Invite Message Issues

### Invite Messages Not Sending

**Problem:** Failed members don't receive invite messages.

**Causes:**
- No message.txt file (uses default message)
- Can't get group invite link
- Rate limiting

**Solution:**
1. Check output for "Sending Invite Messages" section
2. Create custom `message.txt` if needed
3. Verify you're a group admin (needed to get invite link)

### {link} Not Replaced in Message

**Problem:** Recipients see literal `{link}` in message.

**Cause:** Bug in message processing.

**Solution:**
- Report the issue with logs
- Temporarily: Manually share invite link

### Duplicate Invites Being Sent

**Problem:** Same person receiving multiple invite messages.

**Solution:**
- Check `invites_sent.json` - it should prevent duplicates
- Delete the file if you want to resend:
  ```bash
  rm invites_sent.json
  ```

## Performance Issues

### Tool Running Slowly

**Problem:** Takes very long to process members.

**Expected:** This is intentional! The tool waits 5 seconds between each member for safety.

**For 100 members:** ~8-9 minutes (100 × 5 seconds + API calls)

**Not a problem unless:**
- Taking significantly longer than expected
- Hanging/freezing on one member

**Solutions for legitimate slowness:**
- Check network connection
- Use release build: `cargo +nightly build --release`
- Run in smaller batches

### High Memory Usage

**Problem:** Tool using too much RAM.

**Typical usage:** <50 MB even with large lists

**If excessive:**
- Restart the tool
- Check for other processes
- Report if consistently >500 MB

## Output Issues

### No Output / Hanging

**Problem:** Tool starts but shows nothing.

**Solution:**
1. Make sure you scanned the QR code
2. For `get_group_jid`: Send a message in the group
3. Check network connection
4. Try with debug logging:
   ```bash
   RUST_LOG=debug cargo +nightly run --example get_group_jid
   ```

### Truncated/Garbled Output

**Problem:** Output looks wrong or cut off.

**Solution:**
- Ensure terminal supports UTF-8
- Try redirecting to file:
  ```bash
  cargo +nightly run --example add_members "GROUP_JID" phones.json > output.txt 2>&1
  ```

## Edge Cases

### Adding Myself to Group

**Problem:** Trying to add your own number.

**Result:** Will likely return 409 (already member) since you must be a member to add others.

### Adding Group Creator/Admins

**Problem:** Trying to add people who are already admins.

**Result:** Returns 409 (already member) - not an error.

### International Phone Numbers

**Problem:** Unsure about format for different countries.

**Solution:**
- Always use international format WITHOUT `+`
- Country code + number
- Examples:
  - 🇲🇦 Morocco: `212696552892`
  - 🇫🇷 France: `33612345678`
  - 🇬🇧 UK: `447911123456`
  - 🇺🇸 USA: `15551234567`
  - 🇩🇪 Germany: `491234567890`

## Getting More Help

### Enable Debug Logging

Get detailed information about what's happening:

```bash
RUST_LOG=debug cargo +nightly run --example add_members "GROUP_JID" phones.json
```

### Check System Logs

On Linux/Mac:
```bash
# Check system logs
dmesg | tail

# Check Rust panic logs
RUST_BACKTRACE=1 cargo +nightly run --example add_members "GROUP_JID" phones.json
```

### Still Stuck?

If you've tried everything:

1. **Check the logs** with `RUST_LOG=debug`
2. **Document the issue:**
   - What command you ran
   - What error message you got
   - What you expected to happen
3. **Report the bug** with all details

## Common Error Messages Reference

| Error | Meaning | Action |
|-------|---------|--------|
| 400 | Bad phone format | Fix number format, check `invalid_phones.json` |
| 403 | Not authorized | Verify admin status, invite will be sent |
| 404 | User not found | Check number, invite will be sent |
| 409 | Already member | Not an error, skip |
| 429 | Rate limited | Stop, wait 30-60 mins |
| Connection error | Network issue | Check internet, restart tool |
| Session expired | Auth expired | Delete `whatsapp.db*`, rescan QR |

## Prevention Tips

To avoid issues:

✅ **Use correct phone format** (international, no `+`)  
✅ **Be a group admin** for best results  
✅ **Follow rate limits** (20-30/day max)  
✅ **Test with small batches** first  
✅ **Monitor output** for errors  
✅ **Keep session active** (don't delete `whatsapp.db` unnecessarily)  

## Still Having Issues?

Check the [Usage Guide](Usage-Guide) for correct usage patterns, or see the [Developer Guide](Developer-Guide) if you're working on the code.
