impl ServerState {
    /// Remove a player from all server state (peers, players_in_lobbies, players_to_peers, and all lobbies)
    pub fn remove_player(&self, player_id: &str) {
        // Remove from peers (by PeerId)
        if let Some(peer_id) = self.players_to_peers.write().unwrap().remove(player_id) {
            self.peers.lock().unwrap().remove(&peer_id);
        }
        // Remove from players_in_lobbies
        let lobby_id_opt = self.players_in_lobbies.write().unwrap().remove(player_id);
        // Remove from all lobbies
        if let Some(lobby_id) = lobby_id_opt {
            if let Ok(mut lobby_manager) = self.lobby_manager.try_write() {
                lobby_manager.remove_player_from_lobby(&lobby_id, &player_id.to_string());
            }
        } else {
            // Remove from any lobby where present
            if let Ok(mut lobby_manager) = self.lobby_manager.try_write() {
                for lobby in lobby_manager.lobbies.values_mut() {
                    lobby.players.remove(player_id);
                }
            }
        }
    }
}
use crate::auth::ChallengeManager;
use crate::lobby::Lobby;
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
pub struct Peer {
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

    /// Create a lobby and add an initial owner/creator into the players set atomically.
    pub fn create_lobby_with_owner(
        &mut self,
        is_private: bool,
        owner: String,
        whitelist: Option<Vec<String>>,
    ) -> Lobby {
        let mut lobby = Lobby {
            id: Uuid::new_v4(),
            players: Default::default(),
            status: crate::lobby::LobbyStatus::Waiting,
            is_private,
            whitelist: whitelist.map(|w| w.into_iter().collect()),
        };
        lobby.players.insert(owner);
        self.lobbies.insert(lobby.id, lobby.clone());
        lobby
    }

    pub fn create_lobby_with_whitelist(
        &mut self,
        is_private: bool,
        whitelist: Option<Vec<String>>,
    ) -> Lobby {
        let lobby = Lobby {
            id: Uuid::new_v4(),
            players: Default::default(),
            status: crate::lobby::LobbyStatus::Waiting,
            is_private,
            whitelist: whitelist.map(|w| w.into_iter().collect()),
        };
        self.lobbies.insert(lobby.id, lobby.clone());
        lobby
    }

    pub fn get_lobby(&self, id: &Uuid) -> Option<Lobby> {
        self.lobbies.get(id).cloned()
    }

    pub fn get_lobbies_for_player(&self, player_pubkey: Option<String>) -> Vec<Lobby> {
        self.lobbies
            .values()
            .filter(|lobby| {
                // If lobby is public, always show
                if !lobby.is_private && lobby.status == crate::lobby::LobbyStatus::Waiting {
                    return true;
                }
                // If the player is already in the lobby (e.g., the creator), always show it to them
                if let Some(ref pk) = player_pubkey {
                    if lobby.players.contains(pk) {
                        return true;
                    }
                }
                // If lobby is private and has a whitelist, only show if player is whitelisted
                if lobby.is_private {
                    if let Some(whitelist) = &lobby.whitelist {
                        if let Some(ref pk) = player_pubkey {
                            return whitelist.contains(pk);
                        } else {
                            return false;
                        }
                    }
                }
                false
            })
            .cloned()
            .collect()
    }

    pub fn add_player_to_lobby(
        &mut self,
        lobby_id: &Uuid,
        player_id: String,
    ) -> Result<(), SignalingError> {
        if let Some(lobby) = self.lobbies.get_mut(lobby_id) {
            // Check whitelist if it exists
            if let Some(whitelist) = &lobby.whitelist {
                if !whitelist.contains(&player_id) {
                    return Err(SignalingError::UnknownPeer); // Using UnknownPeer to indicate "not allowed"
                }
            }
            lobby.players.insert(player_id);
            Ok(())
        } else {
            // Log available lobbies for debugging when a lobby is unexpectedly missing
            let ids: Vec<String> = self.lobbies.keys().map(|u| u.to_string()).collect();
            tracing::debug!(?ids, ?lobby_id, "add_player_to_lobby: lobby not found");
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
pub struct ServerState {
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
