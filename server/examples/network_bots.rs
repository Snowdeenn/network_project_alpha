//! JSON-lines adapter: Python chooses actions, Renet handles real UDP clients.
use renet::RenetClient;
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Write},
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use utils::{net::*, protocol::*};

struct Bot {
    id: u64,
    client: RenetClient,
    transport: NetcodeClientTransport,
}
#[derive(Deserialize)]
struct Action {
    bot: usize,
    channel: String,
    data: Value,
}
#[derive(Deserialize)]
struct Request {
    #[serde(default)]
    actions: Vec<Action>,
    #[serde(default)]
    wait_ms: u64,
}

fn send<T: bincode::Encode>(
    bot: &mut Bot,
    channel: u8,
    data: T,
) -> Result<(), Box<dyn std::error::Error>> {
    bot.client.send_message(
        channel,
        bincode::encode_to_vec(data, bincode::config::standard())?,
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let count: usize = args.get(1).ok_or("player count required")?.parse()?;
    if !(1..=4).contains(&count) {
        return Err("player count must be 1..4".into());
    }
    let address: SocketAddr = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("127.0.0.1:7777")
        .parse()?;
    let base = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    let mut bots = Vec::new();
    for index in 0..count {
        let id = base + index as u64;
        let transport = NetcodeClientTransport::new(
            SystemTime::now().duration_since(UNIX_EPOCH)?,
            ClientAuthentication::Unsecure {
                client_id: id,
                protocol_id: 1337,
                server_addr: address,
                user_data: None,
            },
            UdpSocket::bind("127.0.0.1:0")?,
        )?;
        bots.push(Bot {
            id,
            client: RenetClient::new(connection_config()),
            transport,
        });
    }
    let mut last = Instant::now();
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        if request.wait_ms > 1000 {
            return Err("wait_ms exceeds 1000".into());
        }
        for action in request.actions {
            let bot = bots.get_mut(action.bot).ok_or("invalid bot index")?;
            match action.channel.as_str() {
                "input" => send(
                    bot,
                    CHANNEL_INPUT,
                    serde_json::from_value::<InputPacket>(action.data)?,
                )?,
                "lobby" => send(
                    bot,
                    CHANNEL_LOBBY,
                    serde_json::from_value::<LobbyMessage>(action.data)?,
                )?,
                "shop" => send(
                    bot,
                    CHANNEL_SHOP,
                    serde_json::from_value::<ShopAction>(action.data)?,
                )?,
                "event" => send(
                    bot,
                    CHANNEL_EVENT,
                    serde_json::from_value::<GameEvent>(action.data)?,
                )?,
                _ => return Err("invalid channel".into()),
            }
        }
        let deadline = Instant::now() + Duration::from_millis(request.wait_ms);
        let mut messages = Vec::new();
        let mut payload_bytes = 0usize;
        loop {
            let now = Instant::now();
            let delta = now.duration_since(last);
            last = now;
            for (index, bot) in bots.iter_mut().enumerate() {
                bot.client.update(delta);
                bot.transport.update(delta, &mut bot.client)?;
                for (channel, name) in [
                    (CHANNEL_STATE, "snapshot"),
                    (CHANNEL_EVENT, "event"),
                    (CHANNEL_LOBBY, "lobby"),
                ] {
                    while let Some(bytes) = bot.client.receive_message(channel) {
                        payload_bytes += bytes.len();
                        let data = match channel {
                            CHANNEL_STATE => json!(
                                bincode::decode_from_slice::<StateSnapshot, _>(
                                    &bytes,
                                    bincode::config::standard()
                                )?
                                .0
                            ),
                            CHANNEL_EVENT => json!(
                                bincode::decode_from_slice::<GameEvent, _>(
                                    &bytes,
                                    bincode::config::standard()
                                )?
                                .0
                            ),
                            _ => json!(
                                bincode::decode_from_slice::<LobbyMessage, _>(
                                    &bytes,
                                    bincode::config::standard()
                                )?
                                .0
                            ),
                        };
                        messages.push(json!({"bot": index, "channel": name, "data": data}));
                    }
                }
                bot.transport.send_packets(&mut bot.client)?;
            }
            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let statuses: Vec<_> = bots
            .iter()
            .map(|bot| json!({"client_id": bot.id, "connected": bot.client.is_connected()}))
            .collect();
        println!(
            "{}",
            json!({"bots": statuses, "messages": messages, "received_payload_bytes": payload_bytes})
        );
        io::stdout().flush()?;
    }
    for bot in &mut bots {
        bot.client.disconnect();
        bot.transport.send_packets(&mut bot.client)?;
    }
    Ok(())
}
