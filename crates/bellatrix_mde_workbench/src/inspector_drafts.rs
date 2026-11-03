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

#[cfg(test)]
mod tests {
    use super::DraftState;

    #[test]
    fn inline_commit_does_not_restore_an_untouched_stale_value() {
        let mut draft = DraftState::new("Original".into(), 1);
        assert_eq!(
            draft.reconcile_for_commit("Original", "Renamed", 2),
            Ok("Renamed".into())
        );
        assert!(!draft.conflict);
    }

    #[test]
    fn inline_commit_preserves_conflicting_text_without_rebasing_it() {
        let mut draft = DraftState::new("Original".into(), 1);
        assert!(draft.reconcile_for_commit("My edit", "Remote edit", 2).is_err());
        assert!(draft.conflict);
        assert_eq!(draft.baseline, "Original");
        assert_eq!(draft.revision, 1);
        assert!(draft.reconcile_for_commit("My edit", "Remote edit", 3).is_err());
        assert_eq!(
            draft.reconcile_for_commit("Remote edit", "Remote edit", 3),
            Ok("Remote edit".into())
        );
        assert!(!draft.conflict);
    }

    #[test]
    fn inline_commit_accepts_unrelated_changes_but_rejects_older_snapshots() {
        let mut draft = DraftState::new("Original".into(), 2);
        assert!(draft.reconcile_for_commit("My edit", "Original", 1).is_err());
        assert_eq!(draft.revision, 2);
        assert_eq!(
            draft.reconcile_for_commit("My edit", "Original", 3),
            Ok("My edit".into())
        );
        assert_eq!(draft.revision, 3);
        assert!(!draft.conflict);
    }

    #[test]
    fn realtime_editing_preserves_newer_input_through_receipts_and_conflicts() {
        let mut draft = DraftState::new("Original".into(), 1);
        let text = draft.reconcile("Original", "Remote rename", 2);
        assert_eq!(text, "Remote rename");
        assert!(!draft.dirty(&text));

        let text = draft.reconcile("My edit", "Remote rename", 3);
        assert_eq!(text, "My edit");
        assert!(!draft.conflict);
        assert_eq!(draft.revision, 3);
        let text = draft.reconcile(&text, "Someone else's edit", 4);
        assert_eq!(text, "My edit");
        assert!(draft.conflict);
        assert_eq!(draft.revision, 3);

        draft.reload("Someone else's edit", 4);
        assert!(!draft.dirty("Someone else's edit"));
        assert!(!draft.conflict);
        draft.edited();
        assert!(draft.queued);
        draft.queued = false; // The first value is now in flight.
        draft.edited(); // More typing while its receipt is pending.
        draft.acknowledge("Applied edit", Ok(5));
        assert!(draft.queued);
        let text = draft.reconcile("Typed while applying", "Applied edit", 5);
        assert_eq!(text, "Typed while applying");
        assert!(draft.dirty(&text));
        assert!(!draft.conflict);
        assert_eq!(draft.reconcile(&text, "Old snapshot", 4), text);

        draft.acknowledge(&text, Err("Rejected by validation".into()));
        assert_eq!(draft.reconcile(&text, "Applied edit", 5), text);
        assert_eq!(draft.error.as_deref(), Some("Rejected by validation"));
        draft.reload("Applied edit", 5);
        draft.queued = false;
        draft.acknowledge("Applied edit", Ok(5));
        assert_eq!(
            draft.reconcile("Applied edit", "Applied edit", 5),
            "Applied edit"
        );
        assert!(!draft.dirty("Applied edit"));
        assert!(draft.error.is_none());
    }
}
