use renet::{ChannelConfig, ConnectionConfig, SendType};
use std::time::Duration;

pub const CHANNEL_STATE: u8 = 0;
pub const CHANNEL_EVENT: u8 = 1;
pub const CHANNEL_INPUT: u8 = 2;
pub const CHANNEL_SHOP: u8 = 3;
pub const CHANNEL_LOBBY: u8 = 4;

fn channels() -> Vec<ChannelConfig> {
    vec![
        ChannelConfig {
            channel_id: CHANNEL_STATE,
            max_memory_usage_bytes: 10 * 1024,
            send_type: SendType::Unreliable,
        },
        ChannelConfig {
            channel_id: CHANNEL_EVENT,
            max_memory_usage_bytes: 5 * 1024,
            send_type: SendType::ReliableOrdered {
                resend_time: Duration::from_millis(200),
            },
        },
        ChannelConfig {
            channel_id: CHANNEL_INPUT,
            max_memory_usage_bytes: 5 * 1024,
            send_type: SendType::Unreliable,
        },
        ChannelConfig {
            channel_id: CHANNEL_SHOP,
            max_memory_usage_bytes: 5 * 1024,
            send_type: SendType::ReliableOrdered {
                resend_time: Duration::from_millis(200),
            },
        },
        ChannelConfig {
            channel_id: CHANNEL_LOBBY,
            max_memory_usage_bytes: 5 * 1024,
            send_type: SendType::ReliableOrdered {
                resend_time: Duration::from_millis(200),
            },
        },
    ]
}

pub fn connection_config() -> ConnectionConfig {
    ConnectionConfig {
        server_channels_config: channels(),
        client_channels_config: channels(),
        ..Default::default()
    }
}
