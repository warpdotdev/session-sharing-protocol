use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Contains information for identifying the end-user.
///
/// Different [`UserID`]'s might correspond to the same end-user;
/// we use the [`UserID`] to translate to a canonical user.
pub struct UserID {
    /// Randomly generated ID for the user, which exists whether or not they are logged in.
    pub anonymous_id: String,

    /// The client's access token. This is either:
    /// * A short-lived firebase ID token (not refresh token).
    /// * A Warp API key.
    ///
    /// [`Some`] iff we know who the end-user is (i.e. they're logged in).
    #[serde(rename = "firebase_id_token")]
    pub access_token: Option<String>,
}

impl Default for UserID {
    fn default() -> Self {
        Self {
            anonymous_id: Uuid::new_v4().to_string(),
            access_token: None,
        }
    }
}

/// A newtype for a firebase uid.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(transparent)]
pub struct FirebaseUid(String);

impl From<String> for FirebaseUid {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<FirebaseUid> for String {
    fn from(value: FirebaseUid) -> Self {
        value.0
    }
}
