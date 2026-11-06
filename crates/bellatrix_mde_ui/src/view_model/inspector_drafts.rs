//! Ephemeral edits relative to an authoritative project snapshot, never a model store.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DraftKey {
    pub project: String,
    pub element: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub(crate) struct DraftState {
    pub baseline: String,
    pub revision: u64,
    pub conflict: bool,
    pub error: Option<String>,
    pub queued: bool,
}

impl DraftState {
    pub fn new(canonical: String, revision: u64) -> Self {
        Self {
            baseline: canonical,
            revision,
            conflict: false,
            error: None,
            queued: false,
        }
    }

    pub fn edited(&mut self) {
        self.queued = true;
        self.error = None;
    }

    pub fn dirty(&self, text: &str) -> bool {
        text != self.baseline
    }

    pub fn reconcile(&mut self, text: &str, canonical: &str, revision: u64) -> String {
        if revision < self.revision {
            return text.to_owned();
        }
        if !self.dirty(text) || text == canonical {
            self.reload(canonical, revision);
            canonical.to_owned()
        } else {
            self.conflict = canonical != self.baseline;
            if !self.conflict {
                self.revision = revision;
            }
            text.to_owned()
        }
    }

    pub fn reconcile_for_commit(
        &mut self,
        text: &str,
        canonical: &str,
        revision: u64,
    ) -> Result<String, String> {
        if revision < self.revision {
            return Err("The model snapshot is out of date; try again when it refreshes".into());
        }
        let reconciled = self.reconcile(text, canonical, revision);
        if self.conflict {
            return Err("This value changed elsewhere. Your text was preserved; cancel and reopen the editor to use the latest value".into());
        }
        Ok(reconciled)
    }

    pub fn reload(&mut self, canonical: &str, revision: u64) {
        self.baseline = canonical.to_owned();
        self.revision = revision;
        self.conflict = false;
        self.error = None;
    }

    pub fn acknowledge(&mut self, submitted: &str, receipt: Result<u64, String>) {
        match receipt {
            Ok(revision) => self.reload(submitted, revision),
            Err(error) => self.error = Some(error),
        }
    }
}
