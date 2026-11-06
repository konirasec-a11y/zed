use crate::pages::mde_page::MdeWorkbenchItem;
use crate::view_model::canvas_presentation;
use bellatrix_mde_cordis::Geometry;
use bellatrix_mde_cordis::ModelingTool;
use gpui::Context;
use gpui::MouseButton;
use gpui::MouseMoveEvent;
use gpui::Pixels;
use gpui::Point;
use gpui::ScrollEvent;
use gpui::px;

#[derive(Clone, Copy)]
pub(crate) enum PathHandle {
    Vertex(usize),
    Control1(usize),
    Control2(usize),
    Segment(usize),
}

#[derive(Clone)]
pub(crate) enum CanvasGesture {
    Move {
        view_id: String,
        start: Point<Pixels>,
        dx: f64,
        dy: f64,
        revision: u64,
        began: bool,
        submitted: bool,
    },
    Resize {
        view_id: String,
        start: Point<Pixels>,
        width: f64,
        height: f64,
        dx: f64,
        dy: f64,
        revision: u64,
        began: bool,
        submitted: bool,
    },
    EditPath {
        diagram_id: String,
        view_id: String,
        geometry: Geometry,
        handle: PathHandle,
        start: Point<Pixels>,
        dx: f64,
        dy: f64,
        revision: u64,
        began: bool,
        submitted: bool,
    },
}
impl CanvasGesture {
    pub(crate) fn submission(&self) -> (u64, bool) {
        match self {
            Self::Move {
                revision,
                submitted,
                ..
            }
            | Self::Resize {
                revision,
                submitted,
                ..
            }
            | Self::EditPath {
                revision,
                submitted,
                ..
            } => (*revision, *submitted),
        }
    }

    pub(crate) fn mark_submitted(&mut self) {
        match self {
            Self::Move { submitted, .. }
            | Self::Resize { submitted, .. }
            | Self::EditPath { submitted, .. } => *submitted = true,
        }
    }
}

impl MdeWorkbenchItem {
    pub fn set_zoom(&mut self, value: f32, cx: &mut Context<Self>) {
        self.zoom = value.clamp(0.25, 3.0);
        cx.notify();
    }
    pub fn reset_viewport(&mut self, cx: &mut Context<Self>) {
        self.zoom = 1.0;
        self.pan = (0.0, 0.0);
        self.pan_origin = None;
        cx.notify();
    }
    pub(crate) fn scroll_viewport(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(20.0));
        let delta_x = f32::from(delta.x);
        let delta_y = f32::from(delta.y);
        if event.modifiers.control {
            self.set_zoom(
                self.zoom * if delta_y > 0.0 { 1.12 } else { 1.0 / 1.12 },
                cx,
            );
        } else {
            self.pan.0 += delta_x;
            self.pan.1 += delta_y;
            cx.notify();
        }
    }
    pub(crate) fn start_pan(&mut self, position: Point<Pixels>) {
        self.pan_origin = Some((position, self.pan));
    }
    pub(crate) fn update_pan(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if event.pressed_button != Some(MouseButton::Middle) {
            return;
        }
        if let Some((position, (x, y))) = &self.pan_origin {
            self.pan = (
                *x + f32::from(event.position.x - position.x),
                *y + f32::from(event.position.y - position.y),
            );
            cx.notify();
        }
    }
    pub(crate) fn start_move(
        &mut self,
        view_id: String,
        subject: String,
        revision: u64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.gesture_submission_pending() {
            return;
        }
        if let Some(ModelingTool::Relation(kind)) = self.presentation.selected_tool() {
            self.connect_relation_view(kind, view_id, None, cx);
            return;
        }
        self.select_view(subject, view_id.clone(), cx);
        if self.source.is_read_only() {
            return;
        }
        self.gesture = Some(CanvasGesture::Move {
            view_id,
            start: position,
            dx: 0.0,
            dy: 0.0,
            revision,
            began: false,
            submitted: false,
        });
    }
    pub(crate) fn start_resize(
        &mut self,
        view_id: String,
        width: f64,
        height: f64,
        revision: u64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() || self.gesture_submission_pending() {
            return;
        }
        self.gesture = Some(CanvasGesture::Resize {
            view_id,
            start: position,
            width,
            height,
            dx: 0.0,
            dy: 0.0,
            revision,
            began: false,
            submitted: false,
        });
        cx.notify();
    }
    pub(crate) fn update_gesture(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.source.is_read_only() || self.gesture_submission_pending() {
            return;
        }
        if event.pressed_button != Some(MouseButton::Left) || !event.dragging() {
            return;
        }
        let Some(gesture) = &mut self.gesture else {
            return;
        };
        let (start, dx, dy, began, view_id, action) = match gesture {
            CanvasGesture::Move {
                start,
                dx,
                dy,
                began,
                view_id,
                ..
            } => (start, dx, dy, began, view_id, "canvas.move.start"),
            CanvasGesture::Resize {
                start,
                dx,
                dy,
                began,
                view_id,
                ..
            } => (start, dx, dy, began, view_id, "canvas.resize.start"),
            CanvasGesture::EditPath {
                start,
                dx,
                dy,
                began,
                view_id,
                ..
            } => (start, dx, dy, began, view_id, "canvas.path_edit.start"),
        };
        *dx = f64::from(event.position.x - start.x) / f64::from(self.zoom);
        *dy = f64::from(event.position.y - start.y) / f64::from(self.zoom);
        if !*began && (dx.abs() >= 1.0 || dy.abs() >= 1.0) {
            *began = true;
            self.source.record_interaction(
                action,
                Some(view_id),
                bellatrix_mde_cordis::TraceOutcome::Success,
            );
        }
        cx.notify();
    }
    pub(crate) fn gesture_submission_pending(&self) -> bool {
        self.gesture
            .as_ref()
            .is_some_and(|gesture| gesture.submission().1)
    }

    pub(crate) fn reconcile_gesture(&mut self, presentation_revision: Option<u64>) {
        let Some(gesture) = &self.gesture else {
            return;
        };
        let (revision, submitted) = gesture.submission();
        // Completion releases the pending count after publishing the document.
        // Read it first so a completed command cannot pair with an older document.
        let pending = self.source.is_pending();
        let retain = self.source.document().is_ok_and(|document| {
            if submitted {
                // Keep only the existing transient preview until the canonical
                // projection replaces it, including the gap after command completion.
                presentation_revision == Some(revision)
                    && (document.revision() != revision || pending)
            } else {
                document.revision() == revision
            }
        });
        if !retain {
            self.gesture = None;
        }
    }

    pub(crate) fn finish_gesture(&mut self, cx: &mut Context<Self>) {
        if self.gesture_submission_pending() {
            return;
        }
        let Some(gesture) = self.gesture.take() else {
            return;
        };
        let (view_id, began) = match &gesture {
            CanvasGesture::Move { view_id, began, .. }
            | CanvasGesture::Resize { view_id, began, .. }
            | CanvasGesture::EditPath { view_id, began, .. } => (view_id, *began),
        };
        if !began {
            cx.notify();
            return;
        }
        if self.source.is_read_only() {
            self.source.record_interaction(
                "canvas.gesture.read_only",
                Some(view_id),
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            cx.notify();
            return;
        }
        let expected_revision = match &gesture {
            CanvasGesture::Move { revision, .. }
            | CanvasGesture::Resize { revision, .. }
            | CanvasGesture::EditPath { revision, .. } => *revision,
        };
        if self
            .source
            .snapshot()
            .ok()
            .map(|snapshot| snapshot.revision)
            != Some(expected_revision)
        {
            let view_id = match &gesture {
                CanvasGesture::Move { view_id, .. }
                | CanvasGesture::Resize { view_id, .. }
                | CanvasGesture::EditPath { view_id, .. } => view_id,
            };
            self.source.record_interaction(
                "canvas.gesture.stale_revision",
                Some(view_id),
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            self.feedback = "Project changed during drag; retry".into();
            cx.notify();
            return;
        }
        let mut submitted_gesture = gesture.clone();
        let result = match gesture {
            CanvasGesture::Move {
                view_id, dx, dy, ..
            } => {
                if dx.abs() < 1.0 && dy.abs() < 1.0 {
                    self.source.record_interaction(
                        "canvas.gesture.no_change",
                        Some(&view_id),
                        bellatrix_mde_cordis::TraceOutcome::Success,
                    );
                    cx.notify();
                    return;
                }
                self.source.move_view(&view_id, dx, dy, expected_revision)
            }
            CanvasGesture::Resize {
                view_id,
                width,
                height,
                dx,
                dy,
                ..
            } => {
                if dx.abs() < 1.0 && dy.abs() < 1.0 {
                    self.source.record_interaction(
                        "canvas.gesture.no_change",
                        Some(&view_id),
                        bellatrix_mde_cordis::TraceOutcome::Success,
                    );
                    cx.notify();
                    return;
                }
                let minimum = self
                    .source
                    .snapshot()
                    .ok()
                    .and_then(|snapshot| {
                        snapshot
                            .section
                            .diagrams
                            .iter()
                            .flat_map(|diagram| diagram.all_views())
                            .find(|view| view.id == view_id)
                            .map(|view| canvas_presentation::minimum_size(&snapshot.section, view))
                    })
                    .unwrap_or((80.0, 48.0));
                self.source.resize_view(
                    &view_id,
                    (width + dx).max(minimum.0),
                    (height + dy).max(minimum.1),
                    expected_revision,
                )
            }
            CanvasGesture::EditPath {
                diagram_id,
                view_id,
                geometry,
                handle,
                dx,
                dy,
                ..
            } => {
                let edited = crate::layouts::body::edited_path_geometry(&geometry, handle, dx, dy);
                if edited == geometry {
                    cx.notify();
                    return;
                }
                self.source
                    .set_view_path(&diagram_id, &view_id, edited, expected_revision)
            }
        };
        self.feedback = match result {
            Ok(()) => {
                submitted_gesture.mark_submitted();
                self.gesture = Some(submitted_gesture);
                "Command sent to Cordis".into()
            }
            Err(error) => format!("Command not sent: {error}"),
        };
        cx.notify();
    }
}
