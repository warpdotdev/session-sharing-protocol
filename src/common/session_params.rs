use serde::{Deserialize, Serialize};

use uuid::Uuid;

/// The canonical identifier for a shared session.
/// A [`SessionId`] on its own cannot be used to access
/// a shared session; you need the corresponding [`SessionSecret`].
/// TODO: consider making the internal type a plain old String.
#[derive(Debug, Hash, Serialize, Deserialize, Eq, PartialEq, Clone, Copy)]
#[serde(transparent)]
pub struct SessionId(Uuid);
impl SessionId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The `warp` server framework uses [`FromStr`] to deserialize
/// the string from the route.
impl std::str::FromStr for SessionId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(SessionId)
    }
}

/// The secret for a shared session.
/// A shared session cannot be accessed without its secret.
/// The client should treat this as some opaque string.
#[derive(Hash, Serialize, Deserialize, Eq, PartialEq, Clone, Default)]
#[serde(transparent)]
pub struct SessionSecret(String);
impl SessionSecret {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// Override the Display impl for the secret to return a mask.
/// This makes it harder to leak the secret by accident.
impl std::fmt::Display for SessionSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***")
    }
}

/// Override the Debug impl for the secret to return a mask.
/// This makes it harder to leak the secret by accident.
impl std::fmt::Debug for SessionSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***")
    }
}

/// The `warp` server framework uses [`FromStr`] to deserialize
/// the string from the route.
impl std::str::FromStr for SessionSecret {
    type Err = core::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        String::from_str(s).map(SessionSecret)
    }
}

/// The parameters needed to attempt to join a shared session.
/// This is different from [`viewer::InitPayload`] which
/// is the state that a viewer must pass up _after_ successfully
/// joining a shared session.
#[derive(Clone)]
pub struct JoinSessionLinkArgs {
    pub session_id: SessionId,
    pub session_secret: SessionSecret,
}

impl JoinSessionLinkArgs {
    // TODO: ideally, the protocol should just generate the full
    // link for the client to consume. This will make more sense
    // once we move away from app URIs.
    pub fn to_join_route(&self) -> String {
        format!(
            "/sessions/join/{}?pwd={}",
            self.session_id,
            self.secret_to_string(),
        )
    }

    /// Returns the [`SessionSecret`] as a [`String`] for joining purposes.
    pub fn secret_to_string(&self) -> String {
        // We can't use the [`SessionSecret`]'s display because it's overriden to be masked.
        self.session_secret.0.to_string()
    }
}
