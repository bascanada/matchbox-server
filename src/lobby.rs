use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

pub type PlayerId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LobbyStatus {
    Waiting,
    InProgress,
}

#[derive(Debug, Clone, Serialize)]
pub struct Lobby {
    pub id: Uuid,
    pub players: HashSet<PlayerId>,
    pub status: LobbyStatus,
    pub is_private: bool,
}
