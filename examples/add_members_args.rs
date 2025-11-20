use qrcode::QrCode;
use qrcode::render::unicode;
use std::sync::Arc;
use wacore::types::events::Event;
use wacore_binary::jid::Jid;
/// Add members to a WhatsApp group using CLI arguments
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
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <group_jid> <phone1> [phone2] [phone3] ...",
            args[0]
        );
        eprintln!(
            "Example: {} \"1234567890-1234567890@g.us\" 212651660005 212696552892",
            args[0]
        );
        eprintln!("\nPhone numbers should be in international format without + sign");
        eprintln!("Note: Members are added one by one with 5 second delays");
        eprintln!("      Rate limit errors (429) are automatically retried after 30 seconds");
        std::process::exit(1);
    }

    let group_jid_str = &args[1];
    let phone_numbers: Vec<String> = args[2..].to_vec();

    let group_jid: Jid = group_jid_str.parse()?;

    if !group_jid_str.ends_with("@g.us") {
        eprintln!("Error: Group JID must end with '@g.us'");
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
