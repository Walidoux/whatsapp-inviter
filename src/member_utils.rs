use wacore_binary::jid::Jid;
use whatsapp_rust::Client;
use std::fs;
use std::path::Path;
use waproto::whatsapp as wa;
use crate::groups::GroupManagement;

#[derive(Debug)]
pub struct AddMemberResult {
    pub jid: Jid,
    pub success: bool,
    pub should_send_invite: bool,
    pub should_track_invalid: bool,
}

#[derive(Debug, Default)]
pub struct AddMemberStats {
    pub total_success: usize,
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
        should_send_invite: false,
        should_track_invalid: false,
    };

    while retry_count <= max_retries {
        if retry_count > 0 {
            println!("   Retry attempt {}/{}", retry_count, max_retries);
        }

        match client.add_group_participants(group_jid, &[member_jid.clone()]).await {
            Ok(results) => {
                for (jid, success, error_code) in results {
                    if success {
                        println!("✓ Successfully added: {}", jid);
                        result.success = true;
                        return result;
                    } else {
                        if let Some(429) = error_code {
                            if retry_count < max_retries {
                                println!("⚠️  Rate limited (429), waiting 30 seconds before retry...");
                                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                                retry_count += 1;
                                continue;
                            }
                        }

                        println!("✗ Failed to add: {} (error code: {:?})", jid, error_code);

                        if let Some(code) = error_code {
                            result.should_track_invalid = code == 400;
                            result.should_send_invite = code == 403 || code == 404;

                            match code {
                                400 => println!("   → Bad request (invalid phone number - will be saved to invalid_phones.json)"),
                                403 => println!("   → Not authorized (you may not be an admin - will send invite message)"),
                                409 => println!("   → User is already in the group"),
                                404 => println!("   → User not found or doesn't have WhatsApp (will send invite message)"),
                                429 => println!("   → Rate limit exceeded (max retries reached)"),
                                _ => println!("   → Unknown error code"),
                            }
                        }
                        return result;
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                if error_msg.contains("429") || error_msg.contains("rate-overlimit") {
                    if retry_count < max_retries {
                        println!("⚠️  Rate limited, waiting 30 seconds before retry...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                        retry_count += 1;
                        continue;
                    }
                }

                result.should_track_invalid = error_msg.contains("400") || error_msg.contains("bad-request");
                result.should_send_invite = error_msg.contains("403") || error_msg.contains("404");

                if result.should_track_invalid {
                    eprintln!("✗ Failed to add {}: {} (saved to invalid_phones.json)", member_jid, e);
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

    if Path::new(file_path).exists() {
        if let Ok(existing_data) = fs::read_to_string(file_path) {
            if let Ok(existing_phones) = serde_json::from_str::<Vec<String>>(&existing_data) {
                all_invalid_phones = existing_phones;
            }
        }
    }

    for phone in invalid_phones {
        if !all_invalid_phones.contains(phone) {
            all_invalid_phones.push(phone.clone());
        }
    }

    let json_data = serde_json::to_string_pretty(&all_invalid_phones)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    fs::write(file_path, json_data)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(all_invalid_phones.len())
}

/// Send invite messages to members who couldn't be added
pub async fn send_invite_messages(
    client: &Client,
    group_jid: &Jid,
    failed_jids: &[Jid],
) -> usize {
    if failed_jids.is_empty() {
        return 0;
    }

    println!("\n=== Sending Invite Messages ===");
    println!("Sending invite messages to {} members who couldn't be added directly\n", failed_jids.len());

    let invite_message = format!(
        "Hi! You've been invited to join our WhatsApp group.\n\
         Group: {}\n\
         Please ask an admin for the invite link or let them know you'd like to join.",
        group_jid
    );

    let mut sent_count = 0;

    for jid in failed_jids {
        let message = wa::Message {
            conversation: Some(invite_message.clone()),
            ..Default::default()
        };

        match client.send_message(jid.clone(), message).await {
            Ok(_) => {
                println!("📧 Sent invite message to {}", jid);
                sent_count += 1;
            }
            Err(e) => eprintln!("⚠️  Failed to send message to {}: {}", jid, e),
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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

    println!("Adding {} members one by one ({}s delay between each)...\n", member_jids.len(), delay_seconds);

    for (index, jid) in member_jids.iter().enumerate() {
        println!("=== Adding member {}/{} ===", index + 1, member_jids.len());

        let result = add_member_with_retry(client, group_jid, jid, 2).await;

        if result.success {
            stats.total_success += 1;
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

pub async fn finalize_member_addition(
    client: &Client,
    group_jid: &Jid,
    stats: AddMemberStats,
) {
    println!("\n=== Final Summary ===");
    println!("✓ Successfully added: {}", stats.total_success);
    println!("✗ Failed: {}", stats.total_failed);
    println!("Total processed: {}", stats.total_success + stats.total_failed);

    if !stats.failed_for_invite.is_empty() {
        send_invite_messages(client, group_jid, &stats.failed_for_invite).await;
    }

    if !stats.invalid_phones.is_empty() {
        match save_invalid_phones(&stats.invalid_phones) {
            Ok(total) => println!("\n📝 Saved {} invalid phone numbers to invalid_phones.json", total),
            Err(e) => eprintln!("⚠️  Failed to save invalid_phones.json: {}", e),
        }
    }
}
