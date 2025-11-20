/// Get Group JID by querying a group you're already in
/// 
/// Usage: 
///   First, send a message to the group you want to add members to
///   Then run: cargo +nightly run --example get_group_jid
///   The bot will listen for messages and display group JIDs

use whatsapp_rust::bot::Bot;
use whatsapp_rust::store::SqliteStore;
use wacore::types::events::Event;
use std::sync::Arc;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;
use qrcode::QrCode;
use qrcode::render::unicode;

// Import our group management trait
use whatsapp_invites::groups::GroupManagement;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let backend = Arc::new(SqliteStore::new("whatsapp.db").await?);
    let transport_factory = TokioWebSocketTransportFactory::new();
    let http_client = UreqHttpClient::new();

    println!("\n=== WhatsApp Group JID Finder ===\n");
    println!("This tool will help you find the JID of your groups.\n");
    println!("📝 Instructions:");
    println!("  1. Scan the QR code below");
    println!("  2. Send a message to the group you want (or wait for someone else to send)");
    println!("  3. The group JID will be displayed here");
    println!("  4. Copy the JID and use it with: cargo +nightly run <GROUP_JID>\n");

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(transport_factory)
        .with_http_client(http_client)
        .on_event(move |event, _client| async move {
            match event {
                Event::PairingQrCode { code, timeout } => {
                    let qr = QrCode::new(code.as_bytes()).unwrap();
                    let image = qr.render::<unicode::Dense1x2>()
                        .dark_color(unicode::Dense1x2::Dark)
                        .light_color(unicode::Dense1x2::Light)
                        .build();
                    println!("Scan this QR code (valid for {}s):\n{}\n", timeout.as_secs(), image);
                }
                Event::Connected(_) => {
                    println!("✅ Connected! Listening for group messages...\n");
                    println!("💡 Send a message to any group now, or wait for incoming messages.\n");
                }
                Event::Message(_msg, info) => {
                    let from = &info.source.chat;
                    
                    // Check if it's a group message (groups end with @g.us)
                    if from.to_string().ends_with("@g.us") {
                        let sender = info.source.sender.to_string();
                        
                        // Query group metadata to get the name
                        let group_info = match _client.query_group_metadata(from).await {
                            Ok(info) => Some(info),
                            Err(e) => {
                                eprintln!("⚠️  Could not fetch group name: {}", e);
                                None
                            }
                        };
                        
                        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        println!("📱 GROUP MESSAGE RECEIVED!");
                        if let Some(info) = group_info {
                            println!("   Group Name: {}", info.subject);
                            println!("   Participants: {}", info.participant_count);
                        }
                        println!("   Group JID: {}", from);
                        println!("   Sender: {}", sender);
                        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
                        println!("✅ To add members to this group, run:");
                        println!("   cargo +nightly run {}\n", from);
                    } else {
                        // It's a direct message, not a group
                        println!("💬 Direct message from: {} (not a group)", from);
                    }
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
