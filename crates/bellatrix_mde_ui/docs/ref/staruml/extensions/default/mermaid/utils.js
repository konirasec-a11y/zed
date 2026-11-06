function getDiagramType(input) {
  const trimed = input.trim();
  if (trimed.startsWith("classDiagram")) {
    return "classDiagram";
  } else if (trimed.startsWith("sequenceDiagram")) {
    return "sequenceDiagram";
  } else if (trimed.startsWith("flowchart")) {
    return "flowchart";
  } else if (trimed.startsWith("erDiagram")) {
    return "erDiagram";
  } else if (trimed.startsWith("mindmap")) {
    return "mindmap";
  } else if (trimed.startsWith("requirementDiagram")) {
    return "requirementDiagram";
  } else if (trimed.startsWith("stateDiagram")) {
    return "stateDiagram";
  }
  return "unknown";
}

function layoutDiagram(diagram, direction, separation) {
  var hiddenEditor = app.diagrams.getHiddenEditor();
  hiddenEditor.diagram = diagram;
  hiddenEditor.repaint();
  hiddenEditor.repaint(); // why should I run it twice?
  app.engine.layoutDiagram(hiddenEditor, diagram, direction, separation);
  hiddenEditor.repaint();
}

function getViewCenter(view) {
  const x = view.left + view.width / 2;
  const y = view.top + view.height / 2;
  return [x, y];
}

function createViewOf(builder, diagram, model, suppressCompartments) {
  var view = null;
  if (!(model instanceof type.Relationship)) {
    var ViewType = model.getViewType();
    if (ViewType) {
      view = new ViewType();
      view._parent = diagram;
      view.model = model;
      view.initialize(null, 10, 10, 10, 10);
      // In case of InterfaceView
      if (view instanceof type.UMLInterfaceView) {
        view.stereotypeDisplay = type.UMLGeneralNodeView.SD_LABEL;
        view.suppressAttributes = false;
        view.suppressOperations = false;
      }
      // Suppress Compartments
      if (suppressCompartments) {
        if (typeof view.suppressAttributes !== "undefined") {
          view.suppressAttributes = true;
        }
        if (typeof view.suppressOperations !== "undefined") {
          view.suppressOperations = true;
        }
        if (typeof view.suppressLiterals !== "undefined") {
          view.suppressLiterals = true;
        }
      }
      builder.insert(view);
      builder.fieldInsert(diagram, "ownedViews", view);
    }
  }
  return view;
}

function reverseDirection(direction, defaultDirection = "TB") {
  switch ((direction || defaultDirection).trim()) {
    case "TB":
    case "TD":
      return "BT";
    case "BT":
      return "TB";
    case "LR":
      return "RL";
    case "RL":
      return "LR";
    default:
      return defaultDirection;
  }
}

module.exports = {
  getDiagramType,
  layoutDiagram,
  getViewCenter,
  createViewOf,
  reverseDirection,
};
