# Bellatrix desktop

A separate GPUI application in the same repository. Its window, explorer, canvas,
inspector, document lifecycle, and text input do not instantiate or depend on the
Zed application, workspace, project service, editor, or Cordis engine.

## Run locally

From the repository root:

```sh
cargo run -p bellatrix_app --bin bellatrix
cargo run -p bellatrix_app --bin bellatrix -- /path/to/project.json
```

The pinned repository Rust toolchain and the native GPUI platform prerequisites
still apply. The binary enables the X11/Wayland backends on Linux and font-kit
where supported. It uses platform fonts and native GPUI file dialogs, without
Zed theme or asset initialization.

No build, check, or test was run for this implementation. Local compilation and
interactive validation remain necessary. The source was formatted with the
repository's pinned rustfmt.

## Working slice

- Create a class with `+ Class` or Ctrl/Cmd+N. Creation and naming use
  `Registry::transaction`, `ruml::Class::create`, and `Class::set_name`.
- Select a class in the explorer or its occurrence in the canvas. The inspector,
  explorer label, and classifier title all read the same `Project.model.registry`.
- Edit the name in the inspector. Each input change updates the typed contract.
  The local GPUI input supports native IME, UTF-16 platform offsets, grapheme-aware
  navigation/deletion, mouse selection, and clipboard shortcuts. It is adapted
  from `gpui/examples/input.rs`, not `ui_input` or a Zed editor.
- Place an existing class on the first diagram, including a second occurrence
  of the same semantic class. Move the selected occurrence by 20 pixels using
  the inspector's arrow buttons. Only its `View.geometry` changes.
- Open schema-2 JSON with the native dialog or the optional command-line path.
  Save and Save As use native dialogs when a destination is needed. Ctrl/Cmd+O,
  Ctrl/Cmd+S, and Ctrl/Cmd+Shift+S are available.
- Dirty status is shown in the window title and toolbar. Opening another document
  or closing the window prompts before discarding changes. Cancelled or failed
  reads/writes keep the current document and dirty state. Editing and repeated
  file operations are blocked while an operation or confirmation is pending.
- Files are written beside the destination and renamed only after a successful
  write and sync. Saving back to the opened path compares its current bytes with
  the last read/saved version and reports external changes instead of knowingly
  overwriting them. Resolved paths also cover relative paths and symlinks.

## State boundary

The only persistent model is `bellatrix_mde_contracts::model::Project`.
`model.registry` holds typed UML contracts, and `model.diagrams` holds view
occurrences and geometry. No second element/relation model, backend, domain
service, or document store is introduced. The shell retains only interaction
state, the current file path, dirty/error status, a text widget, and last-saved
bytes used for conflict detection. Render projections are temporary.

Portable Bellatrix components are reused: `Palette`, `PaneHeader`, and
`UmlClassifier`. Their optional legacy editor/theme features are not enabled.

## Deliberate prototype limits

- The explorer edits exact `Class` contracts. Other typed contracts remain in
  the loaded document and are saved, but have no editing UI here yet.
- The canvas renders visible, top-level rectangular class occurrences in the
  first diagram. Nested views, other geometry, relationships, diagram switching,
  layout/style inheritance, and the full UMLDI notation are not implemented.
  Existing unrendered data is not rewritten into an alternative model.
- Existing owned attribute/operation names are projected into the classifier;
  their editing and full UML signatures are not implemented.
- Position controls are incremental buttons, not drag/resize. Canvas display
  bounds are capped for rendering without rewriting the authored geometry.
- The name input clips long names rather than horizontally scrolling. Keyboard
  navigation and selection continue to operate on the complete string.
- There is no undo/redo, automatic recovery, import/export adapter, project
  browser, or formal UML semantic validation. Loading checks JSON/schema,
  registry identities, unique diagram/view IDs, and finite geometry only.
- Save conflict detection is a pre-write check, not a cross-process file lock.
  Simultaneous external writes during the final rename are not coordinated.
- This is a native desktop entry point, not a web/mobile application bundle.
