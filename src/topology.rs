use crate::state::{Peer, ServerState};
use async_trait::async_trait;
use axum::extract::ws::Message;
use futures::StreamExt;
use matchbox_protocol::{JsonPeerEvent, PeerId, PeerRequest};
use matchbox_signaling::{
    common_logic::parse_request, ClientRequestError, NoCallbacks, SignalingTopology, WsStateMeta,
};
use tracing::{error, info, warn};

#[derive(Debug, Default)]
pub struct MatchmakingDemoTopology;

#[async_trait]
impl SignalingTopology<NoCallbacks, ServerState> for MatchmakingDemoTopology {
    async fn state_machine(upgrade: WsStateMeta<NoCallbacks, ServerState>) {
        let WsStateMeta {
            peer_id,
            sender,
            mut receiver,
            mut state,
            ..
        } = upgrade;

        let player_id = {
            let players_to_peers = state.players_to_peers.read().unwrap();
            tracing::debug!(peer_id = ?peer_id, players_to_peers = ?*players_to_peers, "Looking up player_id for peer_id");
            players_to_peers
                .iter()
                .find(|(_, p)| **p == peer_id)
                .map(|(player_id, _)| player_id.clone())
        };

        let player_id = match player_id {
            Some(id) => {
                tracing::info!(peer_id = ?peer_id, player_id = %&id[..8], "Found player_id for peer");
                id
            }
            None => {
                error!(peer_id = ?peer_id, "No player id found for peer, somehow");
                return;
            }
        };

        let lobby_id = {
            let players_in_lobbies = state.players_in_lobbies.read().unwrap();
            tracing::debug!(player_id = %&player_id[..8], players_in_lobbies = ?*players_in_lobbies, "Looking up lobby_id for player");
            players_in_lobbies.get(&player_id).cloned()
        };

        let lobby_id = match lobby_id {
            Some(id) => {
                tracing::info!(player_id = %&player_id[..8], lobby_id = %id, "Found lobby for player");
                id
            }
            None => {
                error!(player_id = %&player_id[..8], "No lobby id found for peer, somehow");
                return;
            }
        };

        let peer = Peer {
            id: peer_id,
            sender: sender.clone(),
        };
        state.add_peer(peer);

        let players = {
            let lobby_manager = state.lobby_manager.read().unwrap();
            lobby_manager.get_lobby(&lobby_id).map(|l| l.players)
        };

        if let Some(players) = players {
            let event = Message::Text(JsonPeerEvent::NewPeer(peer_id).to_string());
            for player_id_str in players {
                if player_id_str != player_id {
                    let players_to_peers = state.players_to_peers.read().unwrap();
                    if let Some(peer_id) = players_to_peers.get(&player_id_str) {
                        if let Err(e) = state.try_send(*peer_id, event.clone()) {
                            error!("error sending to {peer_id:?}: {e:?}");
                        }
                    }
                }
            }
        }

        while let Some(request) = receiver.next().await {
            let request = match parse_request(request) {
                Ok(request) => request,
                Err(e) => {
                    match e {
                        ClientRequestError::Axum(_) => {
                            warn!("Unrecoverable error with {peer_id:?}: {e:?}");
                            break;
                        }
                        ClientRequestError::Close => {
                            info!("Connection closed by {peer_id:?}");
                            break;
                        }
                        ClientRequestError::Json(_) | ClientRequestError::UnsupportedType(_) => {
                            error!("Error with request: {:?}", e);
                            continue;
                        }
                    };
                }
            };

            match request {
                PeerRequest::Signal { receiver, data } => {
                    let event = Message::Text(
                        JsonPeerEvent::Signal {
                            sender: peer_id,
                            data,
                        }
                        .to_string(),
                    );
                    if let Err(e) = state.try_send(receiver, event) {
                        error!("error sending to {receiver:?}: {e:?}");
                    }
                }
                PeerRequest::KeepAlive => {}
            }
        }

        info!("Removing peer: {:?}", peer_id);
        state.remove_peer(&peer_id);
        {
            let mut players_in_lobbies = state.players_in_lobbies.write().unwrap();
            players_in_lobbies.remove(&player_id);
        }
        {
            let mut players_to_peers = state.players_to_peers.write().unwrap();
            players_to_peers.remove(&player_id);
        }
        {
            let mut lobby_manager = state.lobby_manager.write().unwrap();
            lobby_manager.remove_player_from_lobby(&lobby_id, &player_id);
        }

        let players = {
            let lobby_manager = state.lobby_manager.read().unwrap();
            lobby_manager.get_lobby(&lobby_id).map(|l| l.players)
        };

        if let Some(players) = players {
            let event = Message::Text(JsonPeerEvent::PeerLeft(peer_id).to_string());
            for player_id_str in players {
                if player_id_str != player_id {
                    let players_to_peers = state.players_to_peers.read().unwrap();
                    if let Some(peer_id) = players_to_peers.get(&player_id_str) {
                        if let Err(e) = state.try_send(*peer_id, event.clone()) {
                            error!("error sending to {peer_id:?}: {e:?}");
                        }
                    }
                }
            }
        }
    }
}
