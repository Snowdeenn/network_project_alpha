use std::net::{SocketAddr, UdpSocket};

use renet::{RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};

use shared::net::{CHANNEL_EVENT, CHANNEL_INPUT, CHANNEL_SHOP, CHANNEL_STATE, connection_config};
use shared::protocol::{GameEvent, InputPacket, ShopAction, StateSnapshot};

const MAX_CLIENTS: usize = 4;
const SERVER_ADDR: &str = "127.0.0.1:7777";

pub struct GameNetServer {
    server: RenetServer,
    transport: NetcodeServerTransport,
}

impl GameNetServer {
    pub fn new() -> Self {
        let addr: SocketAddr = SERVER_ADDR.parse().unwrap();
        let socket = UdpSocket::bind(addr).unwrap();

        let server_config = ServerConfig {
            current_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap(),
            max_clients: MAX_CLIENTS,
            protocol_id: 1337,
            public_addresses: vec![addr],
            authentication: ServerAuthentication::Unsecure,
        };

        let transport = NetcodeServerTransport::new(server_config, socket).unwrap();
        let server = RenetServer::new(connection_config());

        Self { server, transport }
    }

    pub fn update(&mut self, delta: std::time::Duration) {
        self.server.update(delta);
        self.transport.update(delta, &mut self.server).unwrap();
    }

    pub fn drain_events(&mut self) -> Vec<ServerEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.server.get_event() {
            events.push(event);
        }
        events
    }

    pub fn drain_inputs(&mut self) -> Vec<(u64, InputPacket)> {
        let mut inputs = Vec::new();
        for client_id in self.server.clients_id() {
            while let Some(bytes) = self.server.receive_message(client_id, CHANNEL_INPUT) {
                if let Ok((packet, _)) =
                    bincode::decode_from_slice(&bytes, bincode::config::standard())
                {
                    inputs.push((client_id, packet));
                }
            }
        }
        inputs
    }

    pub fn drain_shop_actions(&mut self) -> Vec<(u64, ShopAction)> {
        let mut actions = Vec::new();
        for client_id in self.server.clients_id() {
            while let Some(bytes) = self.server.receive_message(client_id, CHANNEL_SHOP) {
                if let Ok((action, _)) =
                    bincode::decode_from_slice(&bytes, bincode::config::standard())
                {
                    actions.push((client_id, action));
                }
            }
        }
        actions
    }

    pub fn broadcast_snapshot(&mut self, snapshot: &StateSnapshot) {
        let bytes = bincode::encode_to_vec(snapshot, bincode::config::standard()).unwrap();
        self.server.broadcast_message(CHANNEL_STATE, bytes);
    }

    pub fn broadcast_event(&mut self, event: &GameEvent) {
        let bytes = bincode::encode_to_vec(event, bincode::config::standard()).unwrap();
        self.server.broadcast_message(CHANNEL_EVENT, bytes);
    }

    pub fn send_snapshot(&mut self, client_id: u64, snapshot: &StateSnapshot) {
        let bytes = bincode::encode_to_vec(snapshot, bincode::config::standard()).unwrap();
        self.server.send_message(client_id, CHANNEL_STATE, bytes);
    }

    pub fn send_event(&mut self, client_id: u64, event: &GameEvent) {
        let bytes = bincode::encode_to_vec(event, bincode::config::standard()).unwrap();
        self.server.send_message(client_id, CHANNEL_EVENT, bytes);
    }

    pub fn flush(&mut self) {
        self.transport.send_packets(&mut self.server);
    }
}
