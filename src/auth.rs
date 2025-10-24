use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use matchbox_auth_common::{
    issue_jwt, verify_signature, AuthError, AuthSecret, Claims,
};

pub const CHALLENGE_EXPIRATION: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Default)]
pub struct ChallengeManager {
    challenges: Arc<Mutex<HashMap<String, Instant>>>,
}

impl ChallengeManager {
    /// Remove expired challenges from the map
    pub fn cleanup_expired(&self) {
        let mut challenges = self.challenges.lock().unwrap();
        let now = Instant::now();
        challenges.retain(|_, &mut timestamp| now.duration_since(timestamp) < CHALLENGE_EXPIRATION);
    }
    pub fn new() -> Self {
        Default::default()
    }

    pub fn generate_challenge(&self) -> String {
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        let challenge: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let mut challenges = self.challenges.lock().unwrap();
        challenges.insert(challenge.clone(), Instant::now());
        challenge
    }

    pub fn verify_challenge(&self, challenge: &str) -> bool {
        let mut challenges = self.challenges.lock().unwrap();
        if let Some(timestamp) = challenges.get(challenge) {
            if timestamp.elapsed() < CHALLENGE_EXPIRATION {
                challenges.remove(challenge);
                return true;
            }
        }
        false
    }
}
