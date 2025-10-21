use crate::state::{Peer, ServerState};
use async_trait::async_trait;
use axum::extract::ws::Message;
use futures::StreamExt;
use matchbox_protocol::{JsonPeerEvent, PeerRequest};
use matchbox_signaling::{
    common_logic::parse_request, ClientRequestError, NoCallbacks, SignalingTopology, WsStateMeta,
};
use tracing::{error, info, warn};
use uuid::Uuid;

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
            path,
            ..
        } = upgrade;

        let lobby_id = path
            .clone()
            .and_then(|p| p.strip_prefix("/ws/").map(String::from))
            .and_then(|s| Uuid::parse_str(&s).ok());

        let lobby_id = match lobby_id {
            Some(id) => id,
            None => {
                error!("No lobby id in path, somehow");
                return;
            }
        };

        let peer = Peer {
            id: peer_id,
            sender: sender.clone(),
        };
        state.add_peer(peer);

        let players = {
            let lobby_manager = state.lobby_manager.lock().unwrap();
            lobby_manager.get_lobby(&lobby_id).map(|l| l.players)
        };

        if let Some(players) = players {
            let event = Message::Text(JsonPeerEvent::NewPeer(peer_id).to_string());
            for player_id_str in players {
                if let Ok(player_peer_id) = player_id_str.parse() {
                    if player_peer_id != peer_id {
                        if let Err(e) = state.try_send(player_peer_id, event.clone()) {
                            error!("error sending to {player_peer_id:?}: {e:?}");
                        }
                    }
                }
            }
        }

        while let Some(request) = receiver.next().await {
            // Handle requests as before, but scoped to the lobby
        }

        info!("Removing peer: {:?}", peer_id);
        state.remove_peer(&peer_id);

        let players = {
            let lobby_manager = state.lobby_manager.lock().unwrap();
            lobby_manager.get_lobby(&lobby_id).map(|l| l.players)
        };

        if let Some(players) = players {
            let event = Message::Text(JsonPeerEvent::PeerLeft(peer_id).to_string());
            for player_id_str in players {
                if let Ok(player_peer_id) = player_id_str.parse() {
                    if player_peer_id != peer_id {
                        if let Err(e) = state.try_send(player_peer_id, event.clone()) {
                            error!("error sending to {player_peer_id:?}: {e:?}");
                        }
                    }
                }
            }
        }
    }
}
