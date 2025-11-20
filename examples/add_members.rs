use qrcode::QrCode;
use qrcode::render::unicode;
use std::fs;
use std::sync::Arc;
use wacore::types::events::Event;
use wacore_binary::jid::Jid;
/// Add members to a WhatsApp group from a JSON file
/// See README.md for usage instructions
use whatsapp_rust::bot::Bot;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

use whatsapp_invites::groups::GroupManagement;
use whatsapp_invites::member_utils::{add_members_batch, finalize_member_addition};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <group_jid> <phones_json_file>", args[0]);
        eprintln!(
            "Example: {} \"1234567890-1234567890@g.us\" phones.json",
            args[0]
        );
        eprintln!("\nThe JSON file should contain an array of phone numbers:");
        eprintln!(r#"  ["1234567890", "0987654321"]"#);
        eprintln!("\nNote: Members are added one by one with 5 second delays");
        eprintln!("      Rate limit errors (429) are automatically retried after 30 seconds");
        std::process::exit(1);
    }

    let group_jid_str = &args[1];
    let phones_file = &args[2];

    let group_jid: Jid = group_jid_str.parse()?;

    if !group_jid_str.ends_with("@g.us") {
        eprintln!("Error: Group JID must end with '@g.us'");
        std::process::exit(1);
    }

    let phone_numbers: Vec<String> = match fs::read_to_string(phones_file) {
        Ok(data) => match serde_json::from_str(&data) {
            Ok(phones) => phones,
            Err(e) => {
                eprintln!("Error: Failed to parse {}: {}", phones_file, e);
                eprintln!("Expected JSON format: [\"1234567890\", \"0987654321\"]");
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Error: Failed to read {}: {}", phones_file, e);
            std::process::exit(1);
        }
    };

    if phone_numbers.is_empty() {
        eprintln!("Error: No phone numbers found in {}", phones_file);
        std::process::exit(1);
    }

    let participant_jids: Vec<Jid> = phone_numbers
        .iter()
        .map(|phone| format!("{}@s.whatsapp.net", phone).parse())
        .collect::<Result<Vec<_>, _>>()?;

    println!(
        "Will add {} participants to group {}",
        participant_jids.len(),
        group_jid
    );
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
                            let image = qr
                                .render::<unicode::Dense1x2>()
                                .dark_color(unicode::Dense1x2::Dark)
                                .light_color(unicode::Dense1x2::Light)
                                .build();
                            println!(
                                "Scan this QR code to pair (valid for {}s):\n{}",
                                timeout.as_secs(),
                                image
                            );
                        }
                        Event::Connected(_) => {
                            println!("Bot connected! Fetching group info...\n");

                            if let Ok(metadata) = client.query_group_metadata(&group_jid).await {
                                println!("=== Group Information ===");
                                println!("Group Name: {}", metadata.subject);
                                println!("Current Participants: {}", metadata.participant_count);
                                println!("Group JID: {}", group_jid);
                                println!();
                            }

                            let stats =
                                add_members_batch(&client, &group_jid, &participant_jids, 5).await;

                            finalize_member_addition(&client, &group_jid, stats).await;
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
