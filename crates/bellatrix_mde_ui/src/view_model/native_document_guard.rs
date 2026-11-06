#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeBufferState<Version, Identity> {
    pub version: Version,
    pub identity: Identity,
    pub dirty: bool,
    pub conflict: bool,
    pub read_only: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeDocumentConflict {
    TextChanged,
    FileChanged,
    SaveStateChanged,
    FileConflict,
    CapabilityChanged,
}

impl NativeDocumentConflict {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::TextChanged => {
                "The source text changed. This is the last validated preview; close and reopen it to refresh."
            }
            Self::FileChanged => {
                "The source file changed or moved. Close and reopen this preview to refresh its file binding."
            }
            Self::SaveStateChanged => {
                "The source buffer save state changed. Close and reopen this preview to refresh."
            }
            Self::FileConflict => {
                "The source buffer conflicts with its file. Resolve the conflict in the native editor before reopening this preview."
            }
            Self::CapabilityChanged => {
                "The source buffer permissions changed. Close and reopen this preview to refresh."
            }
        }
    }
}

pub(crate) struct NativeDocumentGuard<Version, Identity> {
    baseline: NativeBufferState<Version, Identity>,
}

impl<Version: PartialEq, Identity: PartialEq> NativeDocumentGuard<Version, Identity> {
    pub(crate) fn new(
        baseline: NativeBufferState<Version, Identity>,
    ) -> Result<Self, NativeDocumentConflict> {
        if baseline.conflict {
            return Err(NativeDocumentConflict::FileConflict);
        }
        Ok(Self { baseline })
    }

    pub(crate) fn check(
        &self,
        current: &NativeBufferState<Version, Identity>,
    ) -> Result<(), NativeDocumentConflict> {
        if current.conflict {
            return Err(NativeDocumentConflict::FileConflict);
        }
        if current.identity != self.baseline.identity {
            return Err(NativeDocumentConflict::FileChanged);
        }
        if current.version != self.baseline.version {
            return Err(NativeDocumentConflict::TextChanged);
        }
        if current.dirty != self.baseline.dirty {
            return Err(NativeDocumentConflict::SaveStateChanged);
        }
        if current.read_only != self.baseline.read_only {
            return Err(NativeDocumentConflict::CapabilityChanged);
        }
        Ok(())
    }
}
