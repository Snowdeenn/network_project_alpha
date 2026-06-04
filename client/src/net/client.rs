use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use renet::RenetClient;
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};

use shared::net::{connection_config, CHANNEL_EVENT, CHANNEL_INPUT, CHANNEL_SHOP, CHANNEL_STATE};
use shared::protocol::{GameEvent, InputPacket, ShopAction, StateSnapshot};

const SERVER_ADDR: &str = "127.0.0.1:7777";

pub struct GameNetClient {
    client:    RenetClient,
    transport: NetcodeClientTransport,
}

impl GameNetClient {
    pub fn new(client_id: u64) -> Self {
        let socket     = UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr: SocketAddr = SERVER_ADDR.parse().unwrap();

        let auth = ClientAuthentication::Unsecure {
            client_id,
            protocol_id:  1337,         // doit matcher le serveur
            server_addr,
            user_data:    None,
        };

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();

        let transport = NetcodeClientTransport::new(current_time, auth, socket).unwrap();
        let client    = RenetClient::new(connection_config());

        Self { client, transport }
    }

    pub fn update(&mut self, delta: Duration) {
        self.client.update(delta);
        self.transport.update(delta, &mut self.client).unwrap();
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn send_input(&mut self, packet: &InputPacket) {
        let bytes = bincode::encode_to_vec(packet, bincode::config::standard()).unwrap();
        self.client.send_message(CHANNEL_INPUT, bytes);
    }

    pub fn send_shop_action(&mut self, action: &ShopAction) {
        let bytes = bincode::encode_to_vec(action, bincode::config::standard()).unwrap();
        self.client.send_message(CHANNEL_SHOP, bytes);
    }

    pub fn recv_snapshot(&mut self) -> Option<StateSnapshot> {
        let bytes = self.client.receive_message(CHANNEL_STATE)?;
        bincode::decode_from_slice(&bytes, bincode::config::standard())
            .ok()
            .map(|(s, _)| -> StateSnapshot { s })
    }

    pub fn recv_event(&mut self) -> Option<GameEvent> {
        let bytes = self.client.receive_message(CHANNEL_EVENT)?;
        bincode::decode_from_slice(&bytes, bincode::config::standard())
            .ok()
            .map(|(e, _)| -> GameEvent { e })
    }

    pub fn flush(&mut self) {
        self.transport.send_packets(&mut self.client).unwrap();
    }
}