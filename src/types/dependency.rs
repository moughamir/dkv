use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyKind {
    Cargo,
    Npm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyScope {
    Normal,
    Development,
    Build,
    Optional,
    Peer,
    Workspace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub kind: DependencyKind,
    pub name: String,
    pub requirement: String,
    pub scope: DependencyScope,
}
