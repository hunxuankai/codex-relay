use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProfileProblem {
    InvalidConfiguration,
    MissingProvider,
    MissingEnvironmentVariable,
    UnsupportedAuthentication,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VariablePresence {
    Set,
    NotDetected,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyRestriction {
    SharedVariable,
    SystemVariable,
    NotDedicated,
    UnsupportedValue,
    EnvironmentUnavailable,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileKeyStatus {
    pub user: VariablePresence,
    pub relay_process: VariablePresence,
    pub restriction: Option<KeyRestriction>,
    pub shared_with: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliProfile {
    pub name: String,
    pub command: Option<String>,
    pub provider_id: Option<String>,
    pub host: Option<String>,
    pub variable: Option<String>,
    pub inherits_default: bool,
    pub problem: Option<ProfileProblem>,
    pub key_status: Option<ProfileKeyStatus>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliProfileList {
    pub codex_home: String,
    pub profiles: Vec<CliProfile>,
    pub pending_write: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileKeyEdit {
    pub profile_name: String,
    pub provider_id: String,
    pub host: Option<String>,
    pub variable: String,
    pub token: String,
    pub replaces_value: bool,
    pub relay_process_has_value: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveProfileKeyInput {
    pub profile_name: String,
    pub token: String,
    pub new_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileKeySaved {
    pub windows_notified: bool,
}
