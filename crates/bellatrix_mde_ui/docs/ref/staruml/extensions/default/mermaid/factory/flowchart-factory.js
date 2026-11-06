const { Parser } = require("../parser/parser");
const { layoutDiagram, getViewCenter, reverseDirection } = require("../utils");

/**
 * @type {Record<string, {model, view}>}
 */
let nodes = {};

/**
 * Create a Flowchart Diagram from Mermaid input.
 * @param {string} input - The Mermaid flowchart input.
 * @param {type.ExtensibleModel?} base? - The base element to contain the diagram. (optional)
 * @throws {Error} If the input cannot be parsed or if the base is not a model element.
 */
function createFlowchart(input, base) {
  nodes = {};
  const parser = new Parser();
  let db = null;
  try {
    db = parser.parseFlowchart(input);
  } catch (err) {
    console.error(err);
    throw new Error("Failed to parse flowchart");
  }
  const direction = reverseDirection(db.direction);

  // prepare an operation builder
  const builder = app.repository.getOperationBuilder();
  builder.begin("add diagram");

  // prepare base model
  if (!base) {
    base = app.project.project;
  } else if (!(base instanceof type.ExtensibleModel)) {
    builder.discard();
    throw new Error("Base element must be a model element");
  }

  // create flowchart diagram
  let model = new type.FCFlowchart();
  model._parent = base;
  model.name = "Flowchart by Mermaid";
  builder.insert(model);
  builder.fieldInsert(base, "ownedElements", model);
  let diagram = new type.FCFlowchartDiagram();
  diagram._parent = model;
  diagram.name = "Flowchart Diagram by Mermaid";
  builder.insert(diagram);
  builder.fieldInsert(model, "ownedElements", diagram);

  // create nodes;
  for (const nodeId of db.vertices.keys()) {
    const nodeData = db.vertices.get(nodeId);
    addNode(builder, model, diagram, nodeData);
  }

  // create edges;
  for (const edge of db.edges) {
    const source = nodes[edge.start];
    const target = nodes[edge.end];
    addEdge(
      builder,
      model,
      diagram,
      edge.text,
      source.model,
      target.model,
      source.view,
      target.view,
    );
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
  for (const nodeId of Object.keys(nodes)) {
    const nodeModel = nodes[nodeId].model;
    app.modelExplorer.collapse(nodeModel);
  }

  // set current diagram
  app.diagrams.setCurrentDiagram(diagram);
}

function addNode(builder, model, diagram, data) {
  // create node model
  const id = data.id;
  const type = data.type || "square";
  let nodeModel = null;
  nodeModel = createNode(type);
  nodeModel.name = data.text || id;
  nodeModel._parent = model;
  builder.insert(nodeModel);
  builder.fieldInsert(model, "ownedElements", nodeModel);
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
  model,
  diagram,
  name,
  source,
  target,
  sourceView,
  targetView,
) {
  // create edge model
  let edgeModel = null;
  edgeModel = new type.FCFlow();
  edgeModel.name = name;
  edgeModel.source = source;
  edgeModel.target = target;
  edgeModel._parent = source;
  builder.insert(edgeModel);
  builder.fieldInsert(source, "ownedElements", edgeModel);
  // create edge view
  let edgeView = new type.FCFlowView();
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

function createNode(typeName) {
  switch (typeName.toLowerCase().trim()) {
    case "square":
      return new type.FCProcess();
    case "diamond":
      return new type.FCDecision();
    case "stadium":
      return new type.FCTerminator();
    case "round":
      return new type.FCAlternateProcess();
    case "subroutine":
      return new type.FCPredefinedProcess();
    case "cylinder":
      return new type.FCDatabase();
    case "circle":
      return new type.FCConnector();
    case "odd":
      // TODO: not matched node type
      break;
    case "hexagon":
      return new type.FCPreparation();
    case "lean_right":
      return new type.FCData();
    case "lean_left":
      // TODO: not matched node type
      break;
    case "trapezoid":
      // TODO: not matched node type
      break;
    case "inv_trapezoid":
      return new type.FCManualOperation();
    case "doublecircle":
      // TODO: not matched node type
      break;
  }
  return new type.FCProcess();
}

function createNodeView(typeName) {
  switch (typeName.toLowerCase().trim()) {
    case "square":
      return new type.FCProcessView();
    case "diamond":
      return new type.FCDecisionView();
    case "stadium":
      return new type.FCTerminatorView();
    case "round":
      return new type.FCAlternateProcessView();
    case "subroutine":
      return new type.FCPredefinedProcessView();
    case "cylinder":
      return new type.FCDatabaseView();
    case "circle":
      return new type.FCConnectorView();
    case "odd":
      // TODO: not matched node type
      break;
    case "hexagon":
      return new type.FCPreparationView();
    case "lean_right":
      return new type.FCDataView();
    case "lean_left":
      // TODO: not matched node type
      break;
    case "trapezoid":
      // TODO: not matched node type
      break;
    case "inv_trapezoid":
      return new type.FCManualOperationView();
    case "doublecircle":
      // TODO: not matched node type
      break;
  }
  return new type.FCProcessView();
}

module.exports = {
  createFlowchart,
};
