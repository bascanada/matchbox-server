use crate::{auth::ChallengeManager, lobby::Lobby};
use axum::{extract::ws::Message, Error};
use matchbox_protocol::PeerId;
use matchbox_signaling::{
    common_logic::{self, StateObj},
    SignalingError, SignalingState,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub(crate) struct Peer {
    pub id: PeerId,
    pub sender: UnboundedSender<Result<Message, Error>>,
}

#[derive(Default, Debug, Clone)]
pub struct LobbyManager {
    lobbies: HashMap<Uuid, Lobby>,
}

impl LobbyManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn create_lobby(&mut self, is_private: bool) -> Lobby {
        let lobby = Lobby {
            id: Uuid::new_v4(),
            players: Default::default(),
            status: crate::lobby::LobbyStatus::Waiting,
            is_private,
        };
        self.lobbies.insert(lobby.id, lobby.clone());
        lobby
    }

    pub fn get_lobby(&self, id: &Uuid) -> Option<Lobby> {
        self.lobbies.get(id).cloned()
    }

    pub fn get_public_lobbies(&self) -> Vec<Lobby> {
        self.lobbies
            .values()
            .filter(|lobby| !lobby.is_private && lobby.status == crate::lobby::LobbyStatus::Waiting)
            .cloned()
            .collect()
    }

    pub fn add_player_to_lobby(
        &mut self,
        lobby_id: &Uuid,
        player_id: String,
    ) -> Result<(), SignalingError> {
        if let Some(lobby) = self.lobbies.get_mut(lobby_id) {
            lobby.players.insert(player_id);
            Ok(())
        } else {
            Err(SignalingError::UnknownPeer)
        }
    }

    pub fn remove_player_from_lobby(&mut self, lobby_id: &Uuid, player_id: &String) {
        if let Some(lobby) = self.lobbies.get_mut(lobby_id) {
            lobby.players.remove(player_id);
        }
    }
}

#[derive(Default, Debug, Clone)]
pub(crate) struct ServerState {
    pub lobby_manager: Arc<RwLock<LobbyManager>>,
    pub peers: StateObj<HashMap<PeerId, Peer>>,
    pub players_in_lobbies: Arc<RwLock<HashMap<String, Uuid>>>,
    pub challenge_manager: ChallengeManager,
    pub players_to_peers: Arc<RwLock<HashMap<String, PeerId>>>,
    pub waiting_players: Arc<RwLock<HashMap<SocketAddr, String>>>,
}

impl SignalingState for ServerState {}

impl ServerState {
    pub fn add_peer(&mut self, peer: Peer) {
        self.peers.lock().unwrap().insert(peer.id, peer);
    }

    pub fn remove_peer(&mut self, peer_id: &PeerId) -> Option<Peer> {
        self.peers.lock().unwrap().remove(peer_id)
    }

    pub fn get_peer(&self, peer_id: &PeerId) -> Option<Peer> {
        self.peers.lock().unwrap().get(peer_id).cloned()
    }

    pub fn try_send(&self, id: PeerId, message: Message) -> Result<(), SignalingError> {
        let clients = self.peers.lock().unwrap();
        match clients.get(&id) {
            Some(peer) => Ok(common_logic::try_send(&peer.sender, message)?),
            None => Err(SignalingError::UnknownPeer),
        }
    }
}
