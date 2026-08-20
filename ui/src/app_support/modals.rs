use super::*;

#[derive(Clone)]
pub(crate) enum FolderModal {
    Create,
    Rename(String),
}

#[derive(Clone)]
pub(crate) enum FileEntryModal {
    CreateFile,
    CreateDirectory,
    Rename { path: String, is_dir: bool },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionTransferMode {
    Copy,
    Move,
}

impl SessionTransferMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
        }
    }
}

#[derive(Clone)]
pub(crate) struct SessionTransfer {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) mode: SessionTransferMode,
    pub(crate) target_project_id: String,
    pub(crate) from_demo: bool,
}

#[derive(Clone)]
pub(crate) enum UiConfirm {
    EnableFullPermission,
    DeleteFolder(String),
    DeleteSessions(Vec<String>),
    AbandonExploration(String),
    DeleteFileEntry { path: String, is_dir: bool },
    ReloadProjectRules(String),
    SaveAgentContext,
    DeleteUserDemo(String),
}

#[derive(Clone)]
pub(crate) enum UpdateCheckModal {
    Checking,
    Available {
        version: String,
        notes: String,
        install_supported: bool,
        downloading: bool,
        force_update: bool,
    },
    Downloading {
        version: String,
        downloaded_bytes: RwSignal<u64>,
        total_bytes: RwSignal<Option<u64>>,
        force_update: bool,
    },
    ReadyToInstall {
        version: String,
        force_update: bool,
    },
    Installing {
        version: String,
    },
    UpToDate {
        version: String,
    },
    Failed {
        message: String,
    },
}

impl UpdateCheckModal {
    pub(crate) fn dismissible(&self) -> bool {
        match self {
            Self::Downloading { .. } => false,
            Self::Available { force_update, .. } | Self::ReadyToInstall { force_update, .. } => {
                !*force_update
            }
            _ => true,
        }
    }
}

/// A newer release found by the auto-check, surfaced as the sidebar prompt card.
#[derive(Clone)]
pub(crate) struct AvailableUpdate {
    pub(crate) version: String,
}
