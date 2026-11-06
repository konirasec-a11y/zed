const { Parser } = require("../parser/parser");
const { layoutDiagram, getViewCenter, reverseDirection } = require("../utils");

/**
 * @type {Record<string, {model, view}>}
 */
let nodes = {};

/**
 * Create a UML State Diagram from Mermaid input.
 * @param {string} input - The Mermaid state diagram input.
 * @param {type.UMLPackage? | type.UMLClassifer | type.UMLOperation} base? - The base package to contain the diagram. (optional)
 * @throws {Error} If the input cannot be parsed or if the base is not a UMLPackage, UMLClassifier, or UMLOperation.
 */
function createStateDiagram(input, base) {
  nodes = {};
  const parser = new Parser();
  let db = null;
  try {
    db = parser.parseState(input);
  } catch (err) {
    console.error(err);
    throw new Error("Failed to parse state diagram");
  }

  let direction = reverseDirection("TB");
  const dir = db.rootDoc.find((st) => st.stmt === "dir");
  if (dir) {
    direction = reverseDirection(dir.value);
  }

  // prepare an operation builder
  const builder = app.repository.getOperationBuilder();
  builder.begin("add diagram");

  // prepare base model
  if (!base) {
    base = app.project.project;
  } else if (
    !(base instanceof type.UMLPackage) &&
    !(base instanceof type.UMLClassifier) &&
    !(base instanceof type.UMLOperation)
  ) {
    builder.discard();
    throw new Error("Base element must be a package, classifer or operation");
  }

  // create statechart diagram
  let model = new type.UMLStateMachine();
  model._parent = base;
  model.name = "Statemachine by Mermaid";
  builder.insert(model);
  builder.fieldInsert(base, "ownedElements", model);
  let region = new type.UMLRegion();
  region._parent = model;
  builder.insert(region);
  builder.fieldInsert(model, "regions", region);
  let diagram = new type.UMLStatechartDiagram();
  diagram._parent = model;
  diagram.name = "Statechart Diagram by Mermaid";
  builder.insert(diagram);
  builder.fieldInsert(model, "ownedElements", diagram);

  // create nodes;
  for (const nodeData of db.nodes) {
    if (nodeData.shape !== "noteGroup" && nodeData.shape !== "note") {
      addNode(builder, region, diagram, nodeData);
    }
  }

  // create edges;
  for (const edge of db.edges) {
    const source = nodes[edge.start];
    const target = nodes[edge.end];
    if (source && target) {
      addEdge(
        builder,
        region,
        diagram,
        edge.label,
        source.model,
        target.model,
        source.view,
        target.view,
      );
    }
  }

  // create notes;
  for (const nodeData of db.nodes) {
    if (nodeData.shape === "note") {
      const link1 = db.edges.find((edge) => edge.start === nodeData.id);
      const link2 = db.edges.find((edge) => edge.end === nodeData.id);
      if (link1) addNoteView(builder, diagram, link1.end, nodeData.label);
      if (link2) addNoteView(builder, diagram, link2.start, nodeData.label);
    }
  }

  // excute operation
  builder.end();
  var cmd = builder.getOperation();
  app.repository.doOperation(cmd);
  diagram = app.repository.get(diagram._id);

  // layout diagram
  layoutDiagram(diagram, app.type.Diagram[`LD_${direction}`], {
    node: 50,
    edge: 50,
    rank: 50,
  });

  // collapse all nodes
  // for (const nodeId of Object.keys(nodes)) {
  //   const nodeModel = nodes[nodeId].model;
  //   app.modelExplorer.collapse(nodeModel);
  // }

  // set current diagram
  app.diagrams.setCurrentDiagram(diagram);
}

function addNode(builder, region, diagram, data) {
  // create node model
  const id = data.id;
  const type = data.shape;
  let nodeModel = null;
  nodeModel = createNode(type);
  nodeModel.name = data.label || id;
  nodeModel._parent = region;
  builder.insert(nodeModel);
  builder.fieldInsert(region, "vertices", nodeModel);
  // create node view
  let nodeView = createNodeView(type);
  nodeView._parent = diagram;
  nodeView.model = nodeModel;
  nodeView.initialize(null, 10, 10, 10, 10);
  builder.insert(nodeView);
  builder.fieldInsert(diagram, "ownedViews", nodeView);
  // register node
  nodes[id] = { model: nodeModel, view: nodeView };
  return nodeView;
}

function addEdge(
  builder,
  region,
  diagram,
  name,
  source,
  target,
  sourceView,
  targetView,
) {
  // create edge model
  let edgeModel = null;
  edgeModel = new type.UMLTransition();
  edgeModel.name = name;
  edgeModel.source = source;
  edgeModel.target = target;
  edgeModel._parent = source;
  builder.insert(edgeModel);
  builder.fieldInsert(region, "transitions", edgeModel);
  // create edge view
  let edgeView = new type.UMLTransitionView();
  edgeView._parent = diagram;
  edgeView.model = edgeModel;
  edgeView.tail = sourceView;
  edgeView.head = targetView;
  let sourcePoint = getViewCenter(sourceView);
  let targetPoint = getViewCenter(targetView);
  edgeView.initialize(
    null,
    sourcePoint[0],
    sourcePoint[1],
    targetPoint[0],
    targetPoint[1],
  );
  builder.insert(edgeView);
  builder.fieldInsert(diagram, "ownedViews", edgeView);
}

function addNoteView(builder, diagram, nodeId, text) {
  let noteView = new type.UMLNoteView();
  noteView._parent = diagram;
  noteView.wordWrap = false;
  noteView.autoResize = true;
  noteView.text = text;
  builder.insert(noteView);
  builder.fieldInsert(diagram, "ownedViews", noteView);
  if (nodes[nodeId]) {
    const nodeView = nodes[nodeId].view;
    if (nodeView) {
      let noteLinkView = new type.UMLNoteLinkView();
      noteLinkView._parent = diagram;
      noteLinkView.head = noteView;
      noteLinkView.tail = nodeView;
      let tailPoint = getViewCenter(nodeView);
      let headPoint = getViewCenter(noteView);
      noteLinkView.initialize(
        null,
        tailPoint[0],
        tailPoint[1],
        headPoint[0],
        headPoint[1],
      );
      builder.insert(noteLinkView);
      builder.fieldInsert(diagram, "ownedViews", noteLinkView);
    }
  }
}
function createNode(typeName) {
  switch (typeName.trim()) {
    case "stateStart": {
      const state = new type.UMLPseudostate();
      state.kind = type.UMLPseudostate.PSK_INITIAL;
      return state;
    }
    case "stateEnd": {
      return new type.UMLFinalState();
    }
    case "choice": {
      const state = new type.UMLPseudostate();
      state.kind = type.UMLPseudostate.PSK_CHOICE;
      return state;
    }
    case "fork": {
      const state = new type.UMLPseudostate();
      state.kind = type.UMLPseudostate.PSK_FORK;
      return state;
    }
    case "join": {
      const state = new type.UMLPseudostate();
      state.kind = type.UMLPseudostate.PSK_JOIN;
      return state;
    }
    case "rect": {
      return new type.UMLState();
    }
  }
  return new type.UMLState();
}

function createNodeView(typeName) {
  switch (typeName.trim()) {
    case "stateStart":
      return new type.UMLPseudostateView();
    case "stateEnd":
      return new type.UMLFinalStateView();
    case "choice":
      return new type.UMLPseudostateView();
    case "fork":
      return new type.UMLPseudostateView();
    case "join":
      return new type.UMLPseudostateView();
    case "rect":
      return new type.UMLStateView();
  }
  return new type.UMLStateView();
}

module.exports = {
  createStateDiagram,
};
