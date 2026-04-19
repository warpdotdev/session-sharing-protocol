use serde::{Deserialize, Serialize};

use super::Role;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TeamAclData {
    pub acl: Role,
    pub uid: String,
    pub name: String,
}
