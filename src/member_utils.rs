use crate::groups::GroupManagement;
use std::fs;
use std::path::Path;
use wacore_binary::jid::Jid;
use waproto::whatsapp as wa;
use whatsapp_rust::Client;

#[derive(Debug)]
pub struct AddMemberResult {
    pub jid: Jid,
    pub success: bool,
    pub skipped: bool,
    pub should_send_invite: bool,
    pub should_track_invalid: bool,
}

#[derive(Debug, Default)]
pub struct AddMemberStats {
    pub total_success: usize,
    pub total_skipped: usize,
    pub total_failed: usize,
    pub invalid_phones: Vec<String>,
    pub failed_for_invite: Vec<Jid>,
}

/// Add a single member with retry logic for rate limits
pub async fn add_member_with_retry(
    client: &Client,
    group_jid: &Jid,
    member_jid: &Jid,
    max_retries: usize,
) -> AddMemberResult {
    let mut retry_count = 0;
    let mut result = AddMemberResult {
        jid: member_jid.clone(),
        success: false,
        skipped: false,
        should_send_invite: false,
        should_track_invalid: false,
    };

    while retry_count <= max_retries {
        if retry_count > 0 {
            println!("   Retry attempt {}/{}", retry_count, max_retries);
        }

        match client
            .add_group_participants(group_jid, &[member_jid.clone()])
            .await
        {
            Ok(results) => {
                for (jid, success, error_code) in results {
                    if success {
                        println!("✓ Successfully added: {}", jid);
                        result.success = true;
                        return result;
                    } else {
                        if let Some(429) = error_code
                            && retry_count < max_retries {
                                println!(
                                    "⚠️  Rate limited (429), waiting 30 seconds before retry..."
                                );
                                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                                retry_count += 1;
                                continue;
                            }

                        if let Some(code) = error_code {
                            result.should_track_invalid = code == 400;
                            result.should_send_invite = code == 403 || code == 404;
                            result.skipped = code == 409;

                            if code == 409 {
                                println!("⊘ Skipped: {} (already in group)", jid);
                            } else {
                                println!("✗ Failed to add: {} (error code: {:?})", jid, error_code);
                            }

                            match code {
                                400 => println!(
                                    "   → Bad request (invalid phone number - will be saved to invalid_phones.json)"
                                ),
                                403 => println!(
                                    "   → Not authorized (you may not be an admin - will send invite message)"
                                ),
                                409 => println!("   → User is already in the group"),
                                404 => println!(
                                    "   → User not found or doesn't have WhatsApp (will send invite message)"
                                ),
                                429 => println!("   → Rate limit exceeded (max retries reached)"),
                                _ => println!("   → Unknown error code"),
                            }
                        } else {
                            println!("✗ Failed to add: {} (error code: {:?})", jid, error_code);
                        }
                        return result;
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                if (error_msg.contains("429") || error_msg.contains("rate-overlimit"))
                    && retry_count < max_retries {
                        println!("⚠️  Rate limited, waiting 30 seconds before retry...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                        retry_count += 1;
                        continue;
                    }

                result.should_track_invalid =
                    error_msg.contains("400") || error_msg.contains("bad-request");
                result.should_send_invite = error_msg.contains("403") || error_msg.contains("404");

                if result.should_track_invalid {
                    eprintln!(
                        "✗ Failed to add {}: {} (saved to invalid_phones.json)",
                        member_jid, e
                    );
                } else {
                    eprintln!("✗ Failed to add {}: {}", member_jid, e);
                }

                return result;
            }
        }

        retry_count += 1;
    }

    result
}

/// Extract phone number from JID
pub fn jid_to_phone(jid: &Jid) -> String {
    jid.to_string()
        .replace("@s.whatsapp.net", "")
        .replace("@lid", "")
}

/// Save invalid phones to JSON file (appends without duplicates)
pub fn save_invalid_phones(invalid_phones: &[String]) -> Result<usize, String> {
    if invalid_phones.is_empty() {
        return Ok(0);
    }

    let file_path = "invalid_phones.json";
    let mut all_invalid_phones: Vec<String> = Vec::new();

    if Path::new(file_path).exists()
        && let Ok(existing_data) = fs::read_to_string(file_path)
            && let Ok(existing_phones) = serde_json::from_str::<Vec<String>>(&existing_data) {
                all_invalid_phones = existing_phones;
            }

    for phone in invalid_phones {
        if !all_invalid_phones.contains(phone) {
            all_invalid_phones.push(phone.clone());
        }
    }

    let json_data = serde_json::to_string_pretty(&all_invalid_phones)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    fs::write(file_path, json_data).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(all_invalid_phones.len())
}

/// Load list of phones that already received invite messages
fn load_invites_sent() -> Vec<String> {
    let file_path = "invites_sent.json";
    if Path::new(file_path).exists()
        && let Ok(data) = fs::read_to_string(file_path)
            && let Ok(phones) = serde_json::from_str::<Vec<String>>(&data) {
                return phones;
            }
    Vec::new()
}

/// Save list of phones that received invite messages
fn save_invites_sent(phones: &[String]) -> Result<(), String> {
    let file_path = "invites_sent.json";
    let json_data =
        serde_json::to_string_pretty(phones).map_err(|e| format!("Failed to serialize: {}", e))?;

    fs::write(file_path, json_data).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

/// Load invite message template from message.txt
/// Returns default template if file doesn't exist
fn load_invite_message_template() -> String {
    let file_path = "message.txt";

    if Path::new(file_path).exists()
        && let Ok(template) = fs::read_to_string(file_path) {
            return template.trim().to_string();
        }

    // Default template if file doesn't exist
    "Hi! You've been invited to join our WhatsApp group.\n\n\
     Join here: {link}\n\n\
     If the link doesn't work, please contact an admin."
        .to_string()
}

/// Send invite messages to members who couldn't be added
pub async fn send_invite_messages(client: &Client, group_jid: &Jid, failed_jids: &[Jid]) -> usize {
    if failed_jids.is_empty() {
        return 0;
    }

    // Load list of phones that already received invites
    let mut invites_sent = load_invites_sent();

    // Filter out JIDs that already received invite messages
    let mut pending_jids = Vec::new();
    let mut skipped_count = 0;

    for jid in failed_jids {
        let phone = jid_to_phone(jid);
        if invites_sent.contains(&phone) {
            println!("⊘ Skipped invite to {} (already sent)", jid);
            skipped_count += 1;
        } else {
            pending_jids.push(jid.clone());
        }
    }

    if pending_jids.is_empty() {
        if skipped_count > 0 {
            println!(
                "\n✓ All {} member(s) already received invite messages",
                skipped_count
            );
        }
        return 0;
    }

    println!("\n=== Sending Invite Messages ===");
    if skipped_count > 0 {
        println!(
            "Sending invite messages to {} new members ({} already sent)\n",
            pending_jids.len(),
            skipped_count
        );
    } else {
        println!(
            "Sending invite messages to {} members who couldn't be added directly\n",
            pending_jids.len()
        );
    }

    // Try to get the group invite link
    let invite_link = match client.get_group_invite_link(group_jid).await {
        Ok(link) => link,
        Err(e) => {
            eprintln!("⚠️  Failed to get group invite link: {}", e);
            "(ask admin for invite link)".to_string()
        }
    };

    // Load message template and interpolate the invite link
    let template = load_invite_message_template();
    let invite_message = template.replace("{link}", &invite_link);

    let mut sent_count = 0;

    for jid in &pending_jids {
        let message = wa::Message {
            conversation: Some(invite_message.clone()),
            ..Default::default()
        };

        match client.send_message(jid.clone(), message).await {
            Ok(_) => {
                println!("📧 Sent invite message to {}", jid);

                // Track that invite was sent
                let phone = jid_to_phone(jid);
                if !invites_sent.contains(&phone) {
                    invites_sent.push(phone);
                }

                sent_count += 1;
            }
            Err(e) => eprintln!("⚠️  Failed to send message to {}: {}", jid, e),
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Save updated list of invites sent
    if sent_count > 0
        && let Err(e) = save_invites_sent(&invites_sent) {
            eprintln!("⚠️  Failed to save invites_sent.json: {}", e);
        }

    sent_count
}

/// Process adding multiple members one by one with delays
pub async fn add_members_batch(
    client: &Client,
    group_jid: &Jid,
    member_jids: &[Jid],
    delay_seconds: u64,
) -> AddMemberStats {
    let mut stats = AddMemberStats::default();

    println!(
        "Adding {} members one by one ({}s delay between each)...\n",
        member_jids.len(),
        delay_seconds
    );

    for (index, jid) in member_jids.iter().enumerate() {
        println!("=== Adding member {}/{} ===", index + 1, member_jids.len());

        let result = add_member_with_retry(client, group_jid, jid, 2).await;

        if result.success {
            stats.total_success += 1;
        } else if result.skipped {
            stats.total_skipped += 1;
        } else {
            stats.total_failed += 1;

            if result.should_track_invalid {
                let phone = jid_to_phone(jid);
                stats.invalid_phones.push(phone);
            }

            if result.should_send_invite {
                stats.failed_for_invite.push(jid.clone());
            }
        }

        if index < member_jids.len() - 1 {
            println!("Waiting {} seconds before next member...\n", delay_seconds);
            tokio::time::sleep(tokio::time::Duration::from_secs(delay_seconds)).await;
        }
    }

    stats
}

pub async fn finalize_member_addition(client: &Client, group_jid: &Jid, stats: AddMemberStats) {
    println!("\n=== Final Summary ===");
    println!("✓ Successfully added: {}", stats.total_success);
    println!("⊘ Skipped: {}", stats.total_skipped);
    println!("✗ Failed: {}", stats.total_failed);
    println!(
        "Total processed: {}",
        stats.total_success + stats.total_skipped + stats.total_failed
    );

    if !stats.failed_for_invite.is_empty() {
        send_invite_messages(client, group_jid, &stats.failed_for_invite).await;
    }

    if !stats.invalid_phones.is_empty() {
        match save_invalid_phones(&stats.invalid_phones) {
            Ok(total) => println!(
                "\n📝 Saved {} invalid phone numbers to invalid_phones.json",
                total
            ),
            Err(e) => eprintln!("⚠️  Failed to save invalid_phones.json: {}", e),
        }
    }
}
