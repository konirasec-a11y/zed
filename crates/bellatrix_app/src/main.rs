mod document;
mod text_input;

use bellatrix_design_system::Palette;
use bellatrix_mde_contracts::{
    model::{Geometry, Project, View},
    ruml::{Class, Contract},
};
use bellatrix_ui_panels::PaneHeader;
use bellatrix_ui_uml::UmlClassifier;
use gpui::{
    App, Bounds, ClickEvent, Context, Entity, FocusHandle, KeyBinding, PathPromptOptions,
    PromptLevel, SharedString, Subscription, Window, WindowBounds, WindowOptions, actions, div,
    prelude::*, px, rgb, size,
};
use std::path::{Path, PathBuf};
use text_input::{Changed, TextInput};

actions!(bellatrix, [Open, Save, SaveAs, NewClass, Close]);

struct Bellatrix {
    document: Project,
    selected_class: Option<String>,
    selected_view: Option<String>,
    name_input: Entity<TextInput>,
    path: Option<PathBuf>,
    saved_bytes: Option<Vec<u8>>,
    recoveries: Vec<PathBuf>,
    recovery_warning: Option<String>,
    dirty: bool,
    busy: bool,
    error: Option<String>,
    status: String,
    focus: FocusHandle,
    _name_subscription: Subscription,
    _quit_subscription: Subscription,
}

impl Bellatrix {
    fn new(path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(TextInput::new);
        let subscription = cx.subscribe(&name_input, |this, _, event: &Changed, cx| {
            if this.busy {
                return;
            }
            let Some(id) = this.selected_class.clone() else {
                return;
            };
            if this.class(&id).and_then(|class| class.name.as_deref()) == Some(event.0.as_str()) {
                return;
            }
            match this
                .document
                .model
                .registry
                .transaction(|registry| Class::set_name(registry, id, event.0.clone()))
            {
                Ok(()) => {
                    this.dirty = true;
                    this.error = None;
                    this.status = "Name updated in the UML contract".into();
                }
                Err(error) => this.error = Some(error),
            }
            cx.notify();
        });
        let quit_subscription = cx.on_app_quit(|this, _| {
            // Native termination can bypass the window-close confirmation. Do the
            // write before returning a future, while this canonical document exists.
            if this.dirty {
                match document::write_recovery(&this.document) {
                    Ok(path) => eprintln!("Bellatrix recovery saved at {}", path.display()),
                    Err(error) => {
                        eprintln!("Bellatrix could not preserve unsaved changes: {error}")
                    }
                }
            }
            async {}
        });
        let (recoveries, recovery_warning) = match document::recovery_files() {
            Ok(paths) => (paths, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        let mut this = Self {
            document: document::blank(),
            selected_class: None,
            selected_view: None,
            name_input,
            path: None,
            saved_bytes: None,
            recoveries,
            recovery_warning,
            dirty: false,
            busy: false,
            error: None,
            status: "Create a class or open a schema-2 project".into(),
            focus: cx.focus_handle(),
            _name_subscription: subscription,
            _quit_subscription: quit_subscription,
        };
        let weak = cx.weak_entity();
        window.on_window_should_close(cx, move |window, cx| {
            match weak.update(cx, |this, cx| this.close(window, cx)) {
                Ok(()) => false,
                Err(error) => {
                    eprintln!("Cannot access closing Bellatrix window: {error}");
                    true
                }
            }
        });
        window.focus(&this.focus, cx);
        if let Some(path) = path {
            this.load(path, false, window, cx);
        }
        this
    }

    fn set_busy(&mut self, busy: bool, cx: &mut Context<Self>) {
        self.busy = busy;
        self.name_input
            .update(cx, |input, cx| input.set_enabled(!busy, cx));
    }

    fn class(&self, id: &str) -> Option<&Class> {
        match &self.document.model.registry.get_entry(id)?.contract {
            Contract::Class(class) => Some(class),
            _ => None,
        }
    }

    fn select(
        &mut self,
        id: String,
        view: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        let Some(class) = self.class(&id) else {
            return;
        };
        let name = class.name.clone().unwrap_or_default();
        self.selected_class = Some(id);
        self.selected_view = view;
        self.name_input
            .update(cx, |input, cx| input.set_text(name, window, cx));
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn create_class(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let number = self
            .document
            .model
            .registry
            .entries()
            .values()
            .filter(|entry| matches!(entry.contract, Contract::Class(_)))
            .count()
            + 1;
        let created = self.document.model.registry.transaction(|registry| {
            let id = Class::create(registry, "false".into(), "false".into(), "false".into())?;
            Class::set_name(registry, id.clone(), format!("Class{number}"))?;
            Ok(id)
        });
        match created {
            Ok(id) => {
                let view = self.place(&id);
                self.dirty = true;
                self.error = None;
                self.status = "Class created in model.registry".into();
                self.select(id, Some(view), window, cx);
            }
            Err(error) => {
                self.error = Some(error);
                cx.notify();
            }
        }
    }

    fn place(&mut self, subject: &str) -> String {
        if self.document.model.diagrams.is_empty() {
            self.document.model.diagrams.push(document::diagram());
        }
        let mut number = 1;
        let id = loop {
            let id = format!("view-{subject}-{number}");
            if !self
                .document
                .model
                .diagrams
                .iter()
                .any(|diagram| has_view(&diagram.views, &id))
            {
                break id;
            }
            number += 1;
        };
        if let Some(diagram) = self.document.model.diagrams.first_mut() {
            let index = diagram.views.len();
            diagram.views.push(View {
                id: id.clone(),
                subject: Some(subject.to_owned()),
                draft: None,
                endpoints: None,
                timing: None,
                label: None,
                geometry: Geometry::Rect {
                    x: 40.0 + (index % 5) as f64 * 240.0,
                    y: 40.0 + (index / 5) as f64 * 160.0,
                    width: 200.0,
                    height: 120.0,
                },
                children: Vec::new(),
                presentation: Default::default(),
                style: Default::default(),
                z_index: 0,
            });
        }
        id
    }

    fn place_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if let Some(id) = self.selected_class.clone() {
            let view = self.place(&id);
            self.dirty = true;
            self.select(id, Some(view), window, cx);
        }
    }

    fn move_selected(&mut self, dx: f64, dy: f64, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if let Some(view) = self
            .document
            .model
            .diagrams
            .first_mut()
            .and_then(|diagram| {
                diagram
                    .views
                    .iter_mut()
                    .find(|view| Some(&view.id) == self.selected_view.as_ref())
            })
            && let Geometry::Rect { x, y, .. } = &mut view.geometry
        {
            *x = (*x + dx).max(0.0);
            *y = (*y + dy).max(0.0);
            self.dirty = true;
            self.error = None;
            self.status = "Position updated in diagram presentation".into();
            cx.notify();
        }
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if !self.dirty {
            self.choose_open(window, cx);
            return;
        }
        self.set_busy(true, cx);
        cx.notify();
        let answer = window.prompt(
            PromptLevel::Warning,
            "Open another project?",
            Some("This document has unsaved changes. Cancel and save first to keep them."),
            &["Cancel", "Discard changes and open"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            let answer = answer.await;
            if let Err(error) = cx
                .update(|window, cx| {
                    this.update(cx, |this, cx| {
                        this.set_busy(false, cx);
                        match answer {
                            Ok(1) => this.choose_open(window, cx),
                            Ok(_) => cx.notify(),
                            Err(error) => {
                                this.error = Some(error.to_string());
                                cx.notify();
                            }
                        }
                    })
                })
                .and_then(|result| result)
            {
                eprintln!("Cannot finish open confirmation: {error}");
            }
        })
        .detach();
    }

    fn choose_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_busy(true, cx);
        self.status = "Choose a schema-2 JSON project".into();
        cx.notify();
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open Bellatrix project".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let selected = match paths.await {
                Ok(Ok(paths)) => Ok(paths.and_then(|paths| paths.into_iter().next())),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            if let Err(error) = cx
                .update(|window, cx| {
                    this.update(cx, |this, cx| {
                        this.set_busy(false, cx);
                        match selected {
                            Ok(Some(path)) => this.load(path, false, window, cx),
                            Ok(None) => {
                                this.status = "Open cancelled; document unchanged".into();
                                cx.notify();
                            }
                            Err(error) => {
                                this.error = Some(error);
                                cx.notify();
                            }
                        }
                    })
                })
                .and_then(|result| result)
            {
                eprintln!("Cannot finish file selection: {error}");
            }
        })
        .detach();
    }

    fn recover(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if !self.dirty {
            self.load(path, true, window, cx);
            return;
        }
        self.set_busy(true, cx);
        cx.notify();
        let answer = window.prompt(
            PromptLevel::Warning,
            "Open a recovery copy?",
            Some("The current document has unsaved changes. Cancel and save first to keep them."),
            &["Cancel", "Discard current changes and recover"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            let answer = answer.await;
            if let Err(error) = cx
                .update(|window, cx| {
                    this.update(cx, |this, cx| {
                        this.set_busy(false, cx);
                        match answer {
                            Ok(1) => this.load(path, true, window, cx),
                            Ok(_) => cx.notify(),
                            Err(error) => {
                                this.error = Some(error.to_string());
                                cx.notify();
                            }
                        }
                    })
                })
                .and_then(|result| result)
            {
                eprintln!("Cannot finish recovery selection: {error}");
            }
        })
        .detach();
    }

    fn load(
        &mut self,
        path: PathBuf,
        recovering: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_busy(true, cx);
        self.status = format!("Opening {}", path.display());
        cx.notify();
        let read = cx.background_spawn(async move {
            let path = document::destination(&path)?;
            let (document, bytes) = document::read(&path)?;
            Ok::<_, String>((document, bytes, path))
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = read.await;
            if let Err(error) = cx.update(|window, cx| this.update(cx, |this, cx| {
                this.set_busy(false, cx);
                match result {
                    Ok((document, bytes, path)) => {
                        this.document = document;
                        let recovering = recovering || document::is_recovery_path(&path);
                        if recovering {
                            this.path = None;
                            this.saved_bytes = None;
                            this.status = format!("Recovered {}. Save As to keep working; the recovery copy is unchanged.", path.display());
                        } else {
                            this.path = Some(path);
                            this.saved_bytes = Some(bytes);
                            this.status = "Project opened. All contract data retained; this prototype edits classes.".into();
                        }
                        this.dirty = recovering;
                        this.error = None;
                        this.selected_class = None;
                        this.selected_view = None;
                        this.name_input.update(cx, |input, cx| input.set_text(String::new(), window, cx));
                    }
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            })).and_then(|result| result) { eprintln!("Cannot finish opening project: {error}"); }
        }).detach();
    }

    fn save(&mut self, save_as: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let bytes = match document::encode(&self.document) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        self.set_busy(true, cx);
        self.status = "Saving project".into();
        cx.notify();
        let previous_path = self.path.clone();
        let previous_bytes = self.saved_bytes.clone();
        let path = if save_as { None } else { self.path.clone() };
        let prompt = path.is_none().then(|| {
            cx.prompt_for_new_path(
                self.path
                    .as_deref()
                    .and_then(Path::parent)
                    .unwrap_or(Path::new(".")),
                Some("bellatrix-project.json"),
            )
        });
        cx.spawn_in(window, async move |this, cx| {
            let selected = match (path, prompt) {
                (Some(path), _) => Ok(Some(path)),
                (None, Some(prompt)) => match prompt.await {
                    Ok(Ok(path)) => Ok(path),
                    Ok(Err(error)) => Err(error.to_string()),
                    Err(error) => Err(error.to_string()),
                },
                _ => Err("No save destination is available".into()),
            };
            let result = match selected {
                Ok(Some(path)) => {
                    let write_bytes = bytes.clone();
                    cx.background_spawn(async move {
                        let path = document::destination(&path)?;
                        if document::is_recovery_path(&path) {
                            return Err("Choose a destination outside Bellatrix Recovery; recovery copies must remain unchanged".into());
                        }
                        let expected = (previous_path.as_ref() == Some(&path))
                            .then_some(previous_bytes)
                            .flatten();
                        document::write(&path, &write_bytes, expected.as_deref())?;
                        Ok(Some(path))
                    })
                    .await
                }
                Ok(None) => Ok(None),
                Err(error) => Err(error),
            };
            if let Err(error) = cx
                .update(|_, cx| {
                    this.update(cx, |this, cx| {
                        this.set_busy(false, cx);
                        match result {
                            Ok(Some(path)) => {
                                this.status = format!("Saved {}", path.display());
                                this.path = Some(path);
                                    this.saved_bytes = Some(bytes);
                                this.dirty = false;
                                this.error = None;
                            }
                            Ok(None) => {
                                this.status = "Save cancelled; unsaved changes retained".into()
                            }
                            Err(error) => this.error = Some(error),
                        }
                        cx.notify();
                    })
                })
                .and_then(|result| result)
            {
                eprintln!("Cannot finish saving project: {error}");
            }
        })
        .detach();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if !self.dirty {
            window.remove_window();
            return;
        }
        self.set_busy(true, cx);
        cx.notify();
        let answer = window.prompt(
            PromptLevel::Warning,
            "Close without saving?",
            Some("Cancel and save your project to keep the changes."),
            &["Cancel", "Discard changes and close"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            let answer = answer.await;
            if let Err(error) = cx
                .update(|window, cx| {
                    this.update(cx, |this, cx| {
                        this.set_busy(false, cx);
                        match answer {
                            Ok(1) => {
                                this.dirty = false;
                                window.remove_window();
                            }
                            Ok(_) => cx.notify(),
                            Err(error) => {
                                this.error = Some(error.to_string());
                                cx.notify();
                            }
                        }
                    })
                })
                .and_then(|result| result)
            {
                eprintln!("Cannot finish closing confirmation: {error}");
            }
        })
        .detach();
    }
}

fn has_view(views: &[View], id: &str) -> bool {
    views
        .iter()
        .any(|view| view.id == id || has_view(&view.children, id))
}

fn button(
    id: &'static str,
    title: &'static str,
    palette: Palette,
    handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.panel_alt))
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(palette.border)))
        .child(title)
        .on_click(handler)
}

impl Render for Bellatrix {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::current(cx);
        window.set_window_title(&format!(
            "{}{} · Bellatrix",
            if self.dirty { "● " } else { "" },
            self.document.name
        ));
        window.set_window_edited(self.dirty);
        let classes = self
            .document
            .model
            .registry
            .entries()
            .values()
            .filter_map(|entry| {
                if let Contract::Class(class) = &entry.contract {
                    Some((
                        class.id.clone(),
                        class
                            .name
                            .clone()
                            .unwrap_or_else(|| "(unnamed class)".into()),
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let mut explorer = div()
            .id("explorer-list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_2()
            .flex()
            .flex_col()
            .gap_1();
        for (id, name) in &classes {
            let id = id.clone();
            let selected = self.selected_class.as_ref() == Some(&id);
            let view = self
                .document
                .model
                .diagrams
                .first()
                .and_then(|diagram| {
                    diagram.views.iter().find(|view| {
                        view.subject.as_deref() == Some(id.as_str())
                            && matches!(view.geometry, Geometry::Rect { .. })
                    })
                })
                .map(|view| view.id.clone());
            explorer = explorer.child(
                div()
                    .id(SharedString::from(format!("explorer-{id}")))
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(rgb(if selected {
                        palette.border
                    } else {
                        palette.panel
                    }))
                    .child(format!("C  {name}"))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select(id.clone(), view.clone(), window, cx)
                    })),
            );
        }
        let diagram = self.document.model.diagrams.first();
        let (canvas_width, canvas_height) = diagram.map_or((1600.0_f64, 1000.0_f64), |diagram| {
            diagram.views.iter().fold(
                (diagram.width.max(800.0), diagram.height.max(600.0)),
                |(width, height), view| {
                    if let Geometry::Rect {
                        x,
                        y,
                        width: node_width,
                        height: node_height,
                    } = view.geometry
                    {
                        (
                            width.max(x + node_width + 40.0),
                            height.max(y + node_height + 40.0),
                        )
                    } else {
                        (width, height)
                    }
                },
            )
        });
        let mut diagram_content = div()
            .relative()
            .flex_none()
            .w(px(canvas_width.min(100_000.0) as f32))
            .h(px(canvas_height.min(100_000.0) as f32));
        let mut visible_count = 0;
        if let Some(diagram) = diagram {
            for view in &diagram.views {
                let Some(subject) = view.subject.as_deref() else {
                    continue;
                };
                let Some(class) = self.class(subject) else {
                    continue;
                };
                let Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } = view.geometry
                else {
                    continue;
                };
                if view.presentation.visible == Some(false) {
                    continue;
                }
                visible_count += 1;
                let class_id = class.id.clone();
                let view_id = view.id.clone();
                let title = class
                    .name
                    .clone()
                    .unwrap_or_else(|| "(unnamed class)".into());
                let width = width.clamp(100.0, 2000.0);
                let height = height.clamp(60.0, 2000.0);
                let mut classifier = UmlClassifier::new(title)
                    .class_id(class_id.clone())
                    .selected(self.selected_view.as_deref() == Some(view.id.as_str()))
                    .view_size(width, height, 1.0)
                    .presentation(
                        bellatrix_mde_contracts::protocol::presentation::ClassifierPresentation {
                            name_is_italic: class.is_abstract == "true",
                            double_sided: class.is_active == "true",
                        },
                    );
                for id in &class.owned_attribute {
                    if let Some(property) = self
                        .document
                        .model
                        .registry
                        .get::<bellatrix_mde_contracts::ruml::Property>(id)
                    {
                        classifier = classifier.attribute_with_id(
                            id.clone(),
                            property.name.unwrap_or_else(|| "(unnamed)".into()),
                        );
                    }
                }
                for id in &class.owned_operation {
                    if let Some(operation) = self
                        .document
                        .model
                        .registry
                        .get::<bellatrix_mde_contracts::ruml::Operation>(id)
                    {
                        classifier = classifier.operation_with_id(
                            id.clone(),
                            operation.name.unwrap_or_else(|| "(unnamed)".into()),
                        );
                    }
                }
                diagram_content = diagram_content.child(
                    div()
                        .id(SharedString::from(format!("canvas-{view_id}")))
                        .absolute()
                        .left(px(x.clamp(0.0, 100_000.0) as f32))
                        .top(px(y.clamp(0.0, 100_000.0) as f32))
                        .w(px(width as f32))
                        .h(px(height as f32))
                        .cursor_pointer()
                        .child(classifier)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select(class_id.clone(), Some(view_id.clone()), window, cx)
                        })),
                );
            }
        }
        if visible_count == 0 {
            diagram_content = diagram_content.child(
                div()
                    .absolute()
                    .left_8()
                    .top_8()
                    .text_color(rgb(palette.muted))
                    .child("Create a class, or select one and place it on this diagram."),
            );
        }
        let mut inspector = div().flex().flex_col().gap_3().p_3();
        if let Some(id) = &self.selected_class {
            inspector = inspector
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.muted))
                        .child("Class · UML contract"),
                )
                .child(div().text_xs().child("Name"));
            inspector = if self.busy {
                inspector.child(
                    div().child(
                        self.class(id)
                            .and_then(|class| class.name.clone())
                            .unwrap_or_default(),
                    ),
                )
            } else {
                inspector.child(self.name_input.clone())
            };
            inspector = inspector
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.muted))
                        .child("Changes apply immediately to the contract."),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.muted))
                        .child(format!("ID: {id}")),
                )
                .child(button(
                    "place-class",
                    "Place on canvas",
                    palette,
                    cx.listener(|this, _, window, cx| this.place_selected(window, cx)),
                ));
            if self.selected_view.is_some() {
                inspector = inspector
                    .child(div().text_xs().child("Move selected occurrence · 20 px"))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(button(
                                "move-left",
                                "←",
                                palette,
                                cx.listener(|this, _, _, cx| this.move_selected(-20.0, 0.0, cx)),
                            ))
                            .child(button(
                                "move-up",
                                "↑",
                                palette,
                                cx.listener(|this, _, _, cx| this.move_selected(0.0, -20.0, cx)),
                            ))
                            .child(button(
                                "move-down",
                                "↓",
                                palette,
                                cx.listener(|this, _, _, cx| this.move_selected(0.0, 20.0, cx)),
                            ))
                            .child(button(
                                "move-right",
                                "→",
                                palette,
                                cx.listener(|this, _, _, cx| this.move_selected(20.0, 0.0, cx)),
                            )),
                    );
                if let Some(view) = self.document.model.diagrams.first().and_then(|diagram| {
                    diagram
                        .views
                        .iter()
                        .find(|view| Some(&view.id) == self.selected_view.as_ref())
                }) && let Geometry::Rect { x, y, .. } = view.geometry
                {
                    inspector =
                        inspector.child(div().text_xs().child(format!("x: {x:.0}    y: {y:.0}")));
                }
            }
        } else {
            inspector = inspector.child(
                div()
                    .text_color(rgb(palette.muted))
                    .child("Select a class to edit its name and position."),
            );
        }
        let mut recovery_panel = div()
            .id("recovery-copies")
            .max_h(px(140.0))
            .overflow_y_scroll()
            .flex_none()
            .p_2()
            .bg(rgb(palette.panel_alt))
            .border_b_1()
            .border_color(rgb(palette.border));
        if let Some(warning) = &self.recovery_warning {
            recovery_panel = recovery_panel.child(
                div()
                    .text_color(rgb(0xf38ba8))
                    .child(format!("Quit recovery unavailable: {warning}")),
            );
        }
        if !self.recoveries.is_empty() {
            recovery_panel = recovery_panel.child(div().text_xs().text_color(rgb(palette.accent))
                .child("Unsaved recovery copies · select one to open as an unsaved document. Copies are retained."));
            for (index, path) in self.recoveries.iter().enumerate() {
                let path = path.clone();
                recovery_panel = recovery_panel.child(
                    div()
                        .id(SharedString::from(format!("recovery-{index}")))
                        .py_1()
                        .cursor_pointer()
                        .text_xs()
                        .child(format!("Recover {}: {}", index + 1, path.display()))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.recover(path.clone(), window, cx)
                        })),
                );
            }
        }
        let path_label = self
            .path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "Unsaved project".into());
        div()
            .key_context("Bellatrix")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(palette.canvas))
            .text_color(rgb(palette.text))
            .text_size(px(13.0))
            .on_action(cx.listener(|this, _: &Open, window, cx| this.open(window, cx)))
            .on_action(cx.listener(|this, _: &Save, window, cx| this.save(false, window, cx)))
            .on_action(cx.listener(|this, _: &SaveAs, window, cx| this.save(true, window, cx)))
            .on_action(cx.listener(|this, _: &NewClass, window, cx| this.create_class(window, cx)))
            .on_action(cx.listener(|this, _: &Close, window, cx| this.close(window, cx)))
            .child(
                div()
                    .h_12()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .bg(rgb(palette.panel))
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .mr_3()
                            .child("Bellatrix"),
                    )
                    .child(button(
                        "open",
                        "Open",
                        palette,
                        cx.listener(|this, _, window, cx| this.open(window, cx)),
                    ))
                    .child(button(
                        "save",
                        "Save",
                        palette,
                        cx.listener(|this, _, window, cx| this.save(false, window, cx)),
                    ))
                    .child(button(
                        "save-as",
                        "Save As",
                        palette,
                        cx.listener(|this, _, window, cx| this.save(true, window, cx)),
                    ))
                    .child(button(
                        "new-class",
                        "+ Class",
                        palette,
                        cx.listener(|this, _, window, cx| this.create_class(window, cx)),
                    ))
                    .child(div().flex_1())
                    .child(if self.busy {
                        "Working…"
                    } else if self.dirty {
                        "● Unsaved changes"
                    } else if self.path.is_none() {
                        "New project"
                    } else {
                        "Saved"
                    }),
            )
            .when(
                !self.recoveries.is_empty() || self.recovery_warning.is_some(),
                |root| root.child(recovery_panel),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .w(px(230.0))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .bg(rgb(palette.panel))
                            .border_r_1()
                            .border_color(rgb(palette.border))
                            .child(PaneHeader::new(format!(
                                "MODEL · {} classes",
                                classes.len()
                            )))
                            .child(explorer)
                            .child(div().p_3().text_xs().text_color(rgb(palette.muted)).child(
                                format!(
                                    "{} contracts",
                                    self.document.model.registry.entries().len()
                                ),
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(PaneHeader::new(
                                diagram
                                    .map_or("Class diagram", |diagram| diagram.name.as_str())
                                    .to_owned(),
                            ))
                            .child(
                                div()
                                    .id("diagram-scroll")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_scroll()
                                    .child(diagram_content),
                            ),
                    )
                    .child(
                        div()
                            .w(px(290.0))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .bg(rgb(palette.panel))
                            .border_l_1()
                            .border_color(rgb(palette.border))
                            .child(PaneHeader::new("INSPECTOR"))
                            .child(
                                div()
                                    .id("inspector-scroll")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .child(inspector),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .text_xs()
                    .text_color(rgb(if self.error.is_some() {
                        0xf38ba8
                    } else {
                        palette.muted
                    }))
                    .child(self.error.clone().unwrap_or_else(|| self.status.clone()))
                    .child(div().text_color(rgb(palette.muted)).child(path_label)),
            )
    }
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next().map(PathBuf::from);
    if arguments.next().is_some() {
        eprintln!("Usage: bellatrix [project.json]");
        return;
    }
    gpui_platform::application().run(move |cx: &mut App| {
        cx.set_app_identity("dev.bellatrix.app", "Bellatrix");
        text_input::init(cx);
        cx.bind_keys([
            KeyBinding::new(
                if cfg!(target_os = "macos") {
                    "cmd-o"
                } else {
                    "ctrl-o"
                },
                Open,
                Some("Bellatrix"),
            ),
            KeyBinding::new(
                if cfg!(target_os = "macos") {
                    "cmd-s"
                } else {
                    "ctrl-s"
                },
                Save,
                Some("Bellatrix"),
            ),
            KeyBinding::new(
                if cfg!(target_os = "macos") {
                    "cmd-shift-s"
                } else {
                    "ctrl-shift-s"
                },
                SaveAs,
                Some("Bellatrix"),
            ),
            KeyBinding::new(
                if cfg!(target_os = "macos") {
                    "cmd-n"
                } else {
                    "ctrl-n"
                },
                NewClass,
                Some("Bellatrix"),
            ),
            KeyBinding::new(
                if cfg!(target_os = "macos") {
                    "cmd-q"
                } else {
                    "ctrl-q"
                },
                Close,
                Some("Bellatrix"),
            ),
        ]);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);
        if let Err(error) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Bellatrix::new(path, window, cx)),
        ) {
            eprintln!("Cannot open Bellatrix: {error}");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}
