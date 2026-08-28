use renet::{RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use std::net::{SocketAddr, UdpSocket};

use utils::net::{
    CHANNEL_EVENT, CHANNEL_INPUT, CHANNEL_LOBBY, CHANNEL_SHOP, CHANNEL_STATE, connection_config,
};
use utils::protocol::{GameEvent, InputPacket, LobbyMessage, ShopAction, StateSnapshot};

const MAX_CLIENTS: usize = 4;
const SERVER_ADDR: &str = "127.0.0.1:7777";

pub struct GameNetServer {
    server: RenetServer,
    transport: NetcodeServerTransport,
}

#[allow(dead_code)]
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

    pub fn drain_game_event_into(&mut self, buf: &mut Vec<(u64, GameEvent)>) {
        for client_id in self.server.clients_id().into_iter() {
            while let Some(byte) = self.server.receive_message(client_id, CHANNEL_EVENT) {
                if let Ok((event, _)) = bincode::decode_from_slice(&byte, bincode::config::standard()) {
                    buf.push((client_id, event));
                }
            }
        }
    }

    pub fn drain_inputs_into(&mut self, buf: &mut Vec<(u64, InputPacket)>) {
    for client_id in self.server.clients_id().into_iter() {
        while let Some(bytes) = self.server.receive_message(client_id, CHANNEL_INPUT) {
            if let Ok((packet, _)) = bincode::decode_from_slice(&bytes, bincode::config::standard()) {
                buf.push((client_id, packet));
            }
        }
    }
}

    pub fn drain_shop_actions_into(&mut self, buf: &mut Vec<(u64, ShopAction)>) {
        for client_id in self.server.clients_id().into_iter() {
            while let Some(bytes) = self.server.receive_message(client_id, CHANNEL_SHOP) {
                if let Ok((action, _)) =
                    bincode::decode_from_slice(&bytes, bincode::config::standard())
                {
                    buf.push((client_id, action));
                }
            }
        }
    }

    pub fn drain_lobby_messages(&mut self) -> Vec<(u64, LobbyMessage)> {
        let mut msg = Vec::new();
        for client_id in self.server.clients_id() {
            while let Some(bytes) = self.server.receive_message(client_id, CHANNEL_LOBBY) {
                if let Ok((m, _)) = bincode::decode_from_slice(&bytes, bincode::config::standard())
                {
                    msg.push((client_id, m));
                } else {
                    eprintln!("Impossible de decoder le lobby message");
                }
            }
        }
        msg
    }

    pub fn broadcast_snapshot(&mut self, snapshot: &StateSnapshot) {
        let bytes = bincode::encode_to_vec(snapshot, bincode::config::standard()).unwrap();
        self.server.broadcast_message(CHANNEL_STATE, bytes);
    }

    pub fn broadcast_event(&mut self, event: &GameEvent) {
        let bytes = bincode::encode_to_vec(event, bincode::config::standard()).unwrap();
        self.server.broadcast_message(CHANNEL_EVENT, bytes);
    }

    pub fn broadcast_lobby(&mut self, msg: &LobbyMessage) {
        let bytes = bincode::encode_to_vec(msg, bincode::config::standard()).unwrap();
        self.server.broadcast_message(CHANNEL_LOBBY, bytes);
    }

    pub fn send_snapshot(&mut self, client_id: u64, snapshot: &StateSnapshot) {
        let bytes = bincode::encode_to_vec(snapshot, bincode::config::standard()).unwrap();
        self.server.send_message(client_id, CHANNEL_STATE, bytes);
    }

    pub fn send_event(&mut self, client_id: u64, event: &GameEvent) {
        let bytes = bincode::encode_to_vec(event, bincode::config::standard()).unwrap();
        self.server.send_message(client_id, CHANNEL_EVENT, bytes);
    }

    pub fn send_lobby(&mut self, client_id: u64, msg: &LobbyMessage) {
        let bytes = bincode::encode_to_vec(msg, bincode::config::standard()).unwrap();
        self.server.send_message(client_id, CHANNEL_LOBBY, bytes);
    }

    pub fn flush(&mut self) {
        self.transport.send_packets(&mut self.server);
    }
}
