mod groups;

use groups::GroupManagement;
use lazy_static::lazy_static;
use qrcode::QrCode;
use qrcode::render::unicode;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use wacore::types::events::Event;
use wacore_binary::jid::Jid;
use waproto::whatsapp as wa;
use whatsapp_rust::bot::Bot;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

lazy_static! {
    static ref INVITE_LINK: String = {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 2 {
            eprintln!("Usage: {} <invite_link_or_group_jid>", args[0]);
            eprintln!("Example: {} https://chat.whatsapp.com/XXXXX", args[0]);
            eprintln!("Or:      {} 1234567890-1234567890@g.us", args[0]);
            eprintln!("\nNote: Members are added one by one with 5 second delays");
            eprintln!("      Rate limit errors (429) are automatically retried after 30 seconds");
            std::process::exit(1);
        }
        args[1].clone()
    };
}

/// Extract group JID from invite link or return the JID if already provided
fn extract_group_jid(input: &str) -> Option<String> {
    // If it's already a JID format (contains @g.us), return it
    if input.contains("@g.us") {
        return Some(input.to_string());
    }

    // Try to extract from invite link
    // Format: https://chat.whatsapp.com/INVITE_CODE
    if input.contains("chat.whatsapp.com/") {
        // We can't directly get the group JID from invite link without joining
        // User should provide the group JID directly
        None
    } else {
        None
    }
}

/// Send invite links to participants as fallback
async fn send_invite_links(
    client: &whatsapp_rust::Client,
    invite_link: &str,
    participant_jids: &[Jid],
) {
    for jid in participant_jids {
        let message = wa::Message {
            conversation: Some(format!("Join our group: {}", invite_link)),
            ..Default::default()
        };

        match client.send_message(jid.clone(), message).await {
            Ok(_) => println!("📧 Sent invite link to {}", jid),
            Err(e) => eprintln!("Failed to send invite to {}: {}", jid, e),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new("phones.json").exists() {
        eprintln!("phones.json not found. Please create a JSON array of phone numbers.");
        std::process::exit(1);
    }

    let backend = Arc::new(SqliteStore::new("whatsapp.db").await?);

    let transport_factory = TokioWebSocketTransportFactory::new();
    let http_client = UreqHttpClient::new();

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(transport_factory)
        .with_http_client(http_client)
        .on_event(|event, client| async move {
            let invite_link = &*INVITE_LINK;
            println!("{:?}", event);
            match event {
                Event::PairingQrCode { code, timeout } => {
                    let qr = QrCode::new(code.as_bytes()).unwrap();
                    let image = qr.render::<unicode::Dense1x2>()
                        .dark_color(unicode::Dense1x2::Dark)
                        .light_color(unicode::Dense1x2::Light)
                        .build();
                    println!("Scan this QR code to pair (valid for {}s):\n{}", timeout.as_secs(), image);
                }
                Event::Connected(_) => {
                    println!("Bot connected!");

                    // Read phone numbers from file
                    let phone_numbers: Vec<String> = match fs::read_to_string("phones.json") {
                        Ok(data) => match serde_json::from_str(&data) {
                            Ok(phones) => phones,
                            Err(e) => {
                                eprintln!("Failed to parse phones.json: {}", e);
                                return;
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to read phones.json: {}", e);
                            return;
                        }
                    };

                    // Convert phone numbers to JIDs
                    let mut participant_jids = Vec::new();
                    for phone_str in &phone_numbers {
                        let full_jid = format!("{}@s.whatsapp.net", phone_str);
                        match full_jid.parse::<Jid>() {
                            Ok(jid) => participant_jids.push(jid),
                            Err(_) => {
                                eprintln!("Invalid phone number: {}", phone_str);
                            }
                        }
                    }

                    if participant_jids.is_empty() {
                        eprintln!("No valid phone numbers to add!");
                        std::process::exit(1);
                    }

                    // Try to extract group JID from input
                    let group_jid_result = extract_group_jid(invite_link);

                    if let Some(group_jid_str) = group_jid_result {
                        // Direct addition method (preferred)
                        println!("\n=== Adding members directly to group ===");
                        match group_jid_str.parse::<Jid>() {
                            Ok(group_jid) => {
                                // Query group metadata to display group name
                                if let Ok(metadata) = client.query_group_metadata(&group_jid).await {
                                    println!("Group Name: {}", metadata.subject);
                                    println!("Current Participants: {}", metadata.participant_count);
                                }
                                println!("Group JID: {}", group_jid);
                                println!("Adding {} members one by one (5 second delay between each)...\n", participant_jids.len());

                                let mut success_count = 0;
                                let mut failed_jids = Vec::new();
                                let mut invalid_phones = Vec::new();

                                for (index, jid) in participant_jids.iter().enumerate() {
                                    println!("=== Adding member {}/{} ===", index + 1, participant_jids.len());

                                    let mut retry_count = 0;
                                    let max_retries = 2;
                                    let mut added = false;

                                    while retry_count <= max_retries && !added {
                                        if retry_count > 0 {
                                            println!("   Retry attempt {}/{}", retry_count, max_retries);
                                        }

                                        match client.add_group_participants(&group_jid, &[jid.clone()]).await {
                                            Ok(results) => {
                                                for (jid, success, error_code) in results {
                                                    if success {
                                                        println!("✓ Successfully added: {}", jid);
                                                        success_count += 1;
                                                        added = true;
                                                    } else {
                                                        // Check if it's a rate limit error (429)
                                                        if let Some(429) = error_code
                                                            && retry_count < max_retries {
                                                                println!("⚠️  Rate limited (429), waiting 30 seconds before retry...");
                                                                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                                                                retry_count += 1;
                                                                continue;
                                                            }

                                                        println!("✗ Failed to add: {} (error: {:?})", jid, error_code);

                                                        // Track invalid phones (400 errors)
                                                        if let Some(400) = error_code {
                                                            let phone = jid.to_string().replace("@s.whatsapp.net", "").replace("@lid", "");
                                                            invalid_phones.push(phone);
                                                        }

                                                        failed_jids.push(jid);
                                                        added = true;

                                                        // Explain common errors
                                                        if let Some(code) = error_code {
                                                            match code {
                                                                400 => println!("   → Bad request (invalid phone number - will be saved to invalid_phones.json)"),
                                                                403 => println!("   → Not authorized (you may not be an admin)"),
                                                                409 => println!("   → User is already in the group"),
                                                                404 => println!("   → User not found or doesn't have WhatsApp"),
                                                                429 => println!("   → Rate limit exceeded (max retries reached)"),
                                                                _ => {}
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let error_msg = e.to_string();
                                                // Check if error message contains rate limit
                                                if (error_msg.contains("429") || error_msg.contains("rate-overlimit"))
                                                    && retry_count < max_retries {
                                                        println!("⚠️  Rate limited, waiting 30 seconds before retry...");
                                                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                                                        retry_count += 1;
                                                        continue;
                                                    }

                                                // Track invalid phones (400 errors)
                                                if error_msg.contains("400") || error_msg.contains("bad-request") {
                                                    let phone = jid.to_string().replace("@s.whatsapp.net", "").replace("@lid", "");
                                                    invalid_phones.push(phone.clone());
                                                    eprintln!("✗ Failed to add {}: {} (saved to invalid_phones.json)", jid, e);
                                                } else {
                                                    eprintln!("✗ Failed to add {}: {}", jid, e);
                                                }

                                                failed_jids.push(jid.clone());
                                                added = true;
                                            }
                                        }

                                        if !added {
                                            retry_count += 1;
                                        }
                                    }

                                    // Wait 5 seconds before next member
                                    if index < participant_jids.len() - 1 {
                                        println!("Waiting 5 seconds before next member...\n");
                                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                    }
                                }

                                println!("\n=== Summary ===");
                                println!("✓ Successfully added: {}", success_count);
                                println!("✗ Failed: {}", failed_jids.len());

                                // Save invalid phones to JSON file
                                if !invalid_phones.is_empty() {
                                    use std::path::Path;

                                    let file_path = "invalid_phones.json";
                                    let mut all_invalid_phones: Vec<String> = Vec::new();

                                    // Load existing invalid phones if file exists
                                    if Path::new(file_path).exists()
                                        && let Ok(existing_data) = fs::read_to_string(file_path)
                                            && let Ok(existing_phones) = serde_json::from_str::<Vec<String>>(&existing_data) {
                                                all_invalid_phones = existing_phones;
                                            }

                                    // Add new invalid phones (avoid duplicates)
                                    for phone in invalid_phones {
                                        if !all_invalid_phones.contains(&phone) {
                                            all_invalid_phones.push(phone);
                                        }
                                    }

                                    // Save to file
                                    if let Ok(json_data) = serde_json::to_string_pretty(&all_invalid_phones) {
                                        if let Err(e) = fs::write(file_path, json_data) {
                                            eprintln!("⚠️  Failed to save invalid_phones.json: {}", e);
                                        } else {
                                            println!("\n📝 Saved {} invalid phone numbers to invalid_phones.json", all_invalid_phones.len());
                                        }
                                    }
                                }

                                // Fallback: send invite links to failed additions
                                if !failed_jids.is_empty() {
                                    println!("\n=== Sending invite links to failed additions ===");
                                    for jid in failed_jids {
                                        let message = wa::Message {
                                            conversation: Some(format!("Join our group: {}", invite_link)),
                                            ..Default::default()
                                        };

                                        match client.send_message(jid.clone(), message).await {
                                            Ok(_) => println!("📧 Sent invite link to {}", jid),
                                            Err(e) => eprintln!("Failed to send invite to {}: {}", jid, e),
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Invalid group JID: {}", e);
                                eprintln!("Falling back to sending invite links...\n");
                                send_invite_links(&client, invite_link, &participant_jids).await;
                            }
                        }
                    } else {
                        // Invite link method (fallback)
                        println!("\n=== Sending invite links ===");
                        println!("Note: Provide group JID (e.g., 1234567890-1234567890@g.us) to add members directly\n");
                        send_invite_links(&client, invite_link, &participant_jids).await;
                    }

                    std::process::exit(0);
                }
                _ => {}
            }
        })
        .build()
        .await?;

    let bot_handle = bot.run().await?;
    bot_handle.await?;
    Ok(())
}
