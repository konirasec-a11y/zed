use crate::pages::mde_page::MdeWorkbenchItem;
use crate::view_model::canvas_presentation;
use crate::view_model::inspector_drafts;
use crate::view_model::selection;
use gpui::AppContext;
use gpui::Context;
use gpui::Focusable;
use gpui::Window;
use gpui::prelude::*;
use gpui::px;
use ui_input::InputField;

impl MdeWorkbenchItem {
    pub(crate) fn inline_edit_content(
        section: &bellatrix_mde_cordis::projection::ModelingProjection,
        id: &str,
    ) -> Option<(String, &'static str)> {
        // Association is a Classifier, but relation names are properties rather
        // than the object-only stereotype/name-expression command.
        if section.relations.iter().any(|relation| relation.id == id) {
            return selection::inline_relation_name(section, id)
                .map(|name| (name, "properties.name"));
        }
        let element = section.elements.iter().find(|element| element.id == id)?;
        if matches!(
            section.contract(id),
            Some(bellatrix_mde_cordis::projection::Contract::Comment(_))
        ) {
            Some((element.display_label.clone(), "properties.body"))
        } else if section.contracts.conforms_to(id, "Classifier") {
            Some((element.name_expression.clone(), "name_expression"))
        } else {
            let named = element
                .template_parametered_id
                .as_ref()
                .and_then(|id| section.elements.iter().find(|element| &element.id == id))
                .unwrap_or(element);
            Some((named.name.clone(), "name"))
        }
    }

    pub(crate) fn begin_inline_edit(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() || self.selected_view_is_inspection_only() {
            return;
        }
        let Ok(snapshot) = self.source.snapshot() else {
            return;
        };
        let Some((display, path)) = Self::inline_edit_content(&snapshot.section, &id) else {
            return;
        };
        if path == "properties.name" {
            let Some(diagram) =
                snapshot.section.diagrams.iter().find(|diagram| {
                    Some(diagram.id.as_str()) == self.presentation.active_diagram()
                })
            else {
                return;
            };
            let Some(view_id) =
                canvas_presentation::ordered_views(&snapshot.section, &diagram.views)
                    .into_iter()
                    .filter(|view| selection::supports_inline_relation_view(view, &id))
                    .min_by_key(|view| {
                        Some(view.id.as_str()) != self.presentation.selected_canvas_view()
                    })
                    .map(|view| view.id.clone())
            else {
                return;
            };
            if self.presentation.selected_canvas_view() != Some(view_id.as_str()) {
                self.select_view(id.clone(), view_id, cx);
            }
        }
        if self.presentation.editing_element() == Some(id.as_str()) {
            if let Some(field) = &self.inline_editor {
                field.read(cx).focus_handle(cx).focus(window, cx);
                self.gesture = None;
                return;
            }
        }
        let hint = match path {
            "properties.body" => "Body",
            "name_expression" => "<< stereotype >> name",
            _ => "Name",
        };
        let field = cx.new(|cx| InputField::new(window, cx, hint).label_min_width(px(96.0)));
        field.update(cx, |field, cx| field.set_text(&display, window, cx));
        if self.selected_element() != Some(id.as_str()) {
            self.select_element(id.clone(), cx);
        }
        self.presentation.begin_element_edit(id);
        self.inline_editor = Some(field.clone());
        self.inline_draft = Some(inspector_drafts::DraftState::new(
            display,
            snapshot.revision,
        ));
        self.gesture = None;
        field.read(cx).focus_handle(cx).focus(window, cx);
        let text_editor = field.read(cx).editor().clone();
        text_editor.select_all(window, cx);
        cx.notify();
    }
    pub(crate) fn commit_inline_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.source.is_read_only() || self.selected_view_is_inspection_only() {
            return;
        }
        let (Some(id), Some(field)) = (
            self.presentation.editing_element().map(str::to_owned),
            self.inline_editor.clone(),
        ) else {
            return;
        };
        if self.inline_draft.as_ref().is_some_and(|draft| draft.queued) {
            return;
        }
        let text = field.read(cx).text(cx);
        let Ok(snapshot) = self.source.snapshot() else {
            self.feedback = "Project snapshot unavailable".into();
            cx.notify();
            return;
        };
        let Some((canonical, path)) = Self::inline_edit_content(&snapshot.section, &id) else {
            self.feedback = "This target is no longer editable; text preserved".into();
            cx.notify();
            return;
        };
        let Some(draft) = self.inline_draft.as_mut() else {
            self.feedback = "Edit baseline unavailable; cancel and reopen the editor".into();
            cx.notify();
            return;
        };
        let reconciled = match draft.reconcile_for_commit(&text, &canonical, snapshot.revision) {
            Ok(text) => text,
            Err(error) => {
                self.feedback = error;
                cx.notify();
                return;
            }
        };
        let name = if matches!(path, "properties.body" | "properties.name") {
            reconciled
        } else {
            reconciled.trim().to_owned()
        };
        let changed = name != canonical;
        if changed && path == "properties.name" {
            let project_id = self.project_id().clone();
            let diagram_id = self.presentation.active_diagram().map(str::to_owned);
            let (completion, mut receipt) = tokio::sync::watch::channel(None);
            if let Err(error) = self.source.set_property_tracked(
                &id,
                path,
                serde_json::Value::String(name.clone()),
                snapshot.revision,
                completion,
            ) {
                self.feedback = format!("Name not sent; text preserved: {error}");
                cx.notify();
                return;
            }
            if let Some(draft) = self.inline_draft.as_mut() {
                draft.queued = true;
            }
            let submitted_text = text.to_string();
            self.feedback = "Saving relationship name…".into();
            cx.spawn_in(window, async move |owner, cx| {
                let waiting = receipt.borrow().is_none();
                if waiting && let Err(error) = receipt.changed().await {
                    eprintln!("MDE relationship name receipt unavailable: {error}");
                }
                let result = receipt
                    .borrow()
                    .clone()
                    .unwrap_or_else(|| Err("Name receipt unavailable; text preserved".into()));
                if let Err(error) = owner.update_in(cx, |item, window, cx| {
                    if item.project_id() != &project_id
                        || item.presentation.active_diagram() != diagram_id.as_deref()
                        || item.presentation.editing_element() != Some(id.as_str())
                        || !item
                            .inline_editor
                            .as_ref()
                            .is_some_and(|current| current.entity_id() == field.entity_id())
                    {
                        return;
                    }
                    if let Some(draft) = item.inline_draft.as_mut() {
                        draft.queued = false;
                        draft.acknowledge(&name, result.clone());
                    }
                    match result {
                        Ok(_) => {
                            if field.read(cx).text(cx).as_str() == submitted_text.as_str() {
                                let focused =
                                    field.read(cx).focus_handle(cx).contains_focused(window, cx);
                                item.presentation.clear_edit();
                                item.inline_editor = None;
                                item.inline_draft = None;
                                if focused {
                                    item.focus_handle.focus(window, cx);
                                }
                            }
                            item.feedback = "Relationship name saved".into();
                        }
                        Err(error) => {
                            item.feedback = format!("Name not saved; text preserved: {error}");
                        }
                    }
                    cx.notify();
                }) {
                    eprintln!("MDE relationship name receipt update failed: {error}");
                }
            })
            .detach();
            cx.notify();
            return;
        }
        self.presentation.clear_edit();
        self.inline_editor = None;
        self.inline_draft = None;
        if changed {
            if path == "name_expression" {
                let result = self
                    .source
                    .set_name_expression(&id, name, snapshot.revision);
                self.report_command_result(result, cx);
            } else {
                let result = self.source.set_property(
                    &id,
                    path,
                    serde_json::Value::String(name),
                    snapshot.revision,
                );
                self.report_command_result(result, cx);
            }
        }
        self.focus_handle.focus(window, cx);
        cx.notify();
    }
    pub(crate) fn cancel_inline_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.presentation.clear_edit();
        self.inline_editor = None;
        self.inline_draft = None;
        self.focus_handle.focus(window, cx);
        cx.notify();
    }
}
