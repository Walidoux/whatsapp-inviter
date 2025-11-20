/// Example: Add members directly to a WhatsApp group using CLI arguments
/// 
/// Usage: cargo run --example add_members_args <group_jid> <phone1> [phone2] [phone3] ...
/// Example: cargo run --example add_members_args "1234567890-1234567890@g.us" 212651660005 212696552892

use whatsapp_rust::bot::Bot;
use whatsapp_rust::store::SqliteStore;
use wacore::types::events::Event;
use wacore_binary::jid::Jid;
use std::sync::Arc;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;
use qrcode::QrCode;
use qrcode::render::unicode;

mod groups {
    include!("../src/groups.rs");
}
use groups::GroupManagement;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <group_jid> <phone1> [phone2] [phone3] ...", args[0]);
        eprintln!("Example: {} \"1234567890-1234567890@g.us\" 212651660005 212696552892", args[0]);
        eprintln!("\nPhone numbers should be in international format without + sign");
        eprintln!("Note: Members are added one by one with 5 second delays");
        eprintln!("      Rate limit errors (429) are automatically retried after 30 seconds");
        std::process::exit(1);
    }

    let group_jid_str = &args[1];
    let phone_numbers: Vec<String> = args[2..].to_vec();

    let group_jid: Jid = group_jid_str.parse()?;
    
    // Validate group JID format
    if !group_jid_str.ends_with("@g.us") {
        eprintln!("Error: Group JID must end with '@g.us'");
        std::process::exit(1);
    }

    // Convert phone numbers to JIDs
    let participant_jids: Vec<Jid> = phone_numbers
        .iter()
        .map(|phone| format!("{}@s.whatsapp.net", phone).parse())
        .collect::<Result<Vec<_>, _>>()?;

    println!("Will add {} participants to group {}", participant_jids.len(), group_jid);
    for jid in &participant_jids {
        println!("  - {}", jid);
    }

    let backend = Arc::new(SqliteStore::new("whatsapp.db").await?);
    let transport_factory = TokioWebSocketTransportFactory::new();
    let http_client = UreqHttpClient::new();

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(transport_factory)
        .with_http_client(http_client)
        .on_event({
            let group_jid = group_jid.clone();
            let participant_jids = participant_jids.clone();
            move |event, client| {
                let group_jid = group_jid.clone();
                let participant_jids = participant_jids.clone();
                async move {
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
                            println!("Bot connected! Fetching group info...\n");

                            // Query group metadata to display group name
                            if let Ok(metadata) = client.query_group_metadata(&group_jid).await {
                                println!("=== Group Information ===");
                                println!("Group Name: {}", metadata.subject);
                                println!("Current Participants: {}", metadata.participant_count);
                                println!("Group JID: {}", group_jid);
                                println!();
                            }

                            println!("Adding {} members one by one (5 second delay between each)...\n", participant_jids.len());

                            let mut total_success = 0;
                            let mut total_failed = 0;

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
                                                    total_success += 1;
                                                    added = true;
                                                } else {
                                                    // Check if it's a rate limit error (429)
                                                    if let Some(429) = error_code {
                                                        if retry_count < max_retries {
                                                            println!("⚠️  Rate limited (429), waiting 30 seconds before retry...");
                                                            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                                                            retry_count += 1;
                                                            continue;
                                                        }
                                                    }
                                                    
                                                    println!("✗ Failed to add: {} (error code: {:?})", jid, error_code);
                                                    total_failed += 1;
                                                    added = true;
                                                    
                                                    // Explain common error codes
                                                    if let Some(code) = error_code {
                                                        match code {
                                                            403 => println!("   → Not authorized (you may not be an admin)"),
                                                            409 => println!("   → User is already in the group"),
                                                            404 => println!("   → User not found or doesn't have WhatsApp"),
                                                            429 => println!("   → Rate limit exceeded (max retries reached)"),
                                                            _ => println!("   → Unknown error code"),
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let error_msg = e.to_string();
                                            // Check if error message contains rate limit
                                            if error_msg.contains("429") || error_msg.contains("rate-overlimit") {
                                                if retry_count < max_retries {
                                                    println!("⚠️  Rate limited, waiting 30 seconds before retry...");
                                                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                                                    retry_count += 1;
                                                    continue;
                                                }
                                            }
                                            
                                            eprintln!("✗ Failed to add {}: {}", jid, e);
                                            total_failed += 1;
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

                            println!("\n=== Final Summary ===");
                            println!("✓ Successfully added: {}", total_success);
                            println!("✗ Failed: {}", total_failed);
                            println!("Total processed: {}", total_success + total_failed);

                            std::process::exit(0);
                        }
                        _ => {}
                    }
                }
            }
        })
        .build()
        .await?;

    let bot_handle = bot.run().await?;
    bot_handle.await?;
    Ok(())
}
