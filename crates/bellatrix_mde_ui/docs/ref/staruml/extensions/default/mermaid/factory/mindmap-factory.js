const { Parser } = require("../parser/parser");
const { layoutDiagram, getViewCenter } = require("../utils");

/**
 * @type {Record<string, {model, view}>}
 */
let nodes = {};

/**
 * Create a Mindmap Diagram from Mermaid input.
 * @param {string} input - The Mermaid mindmap input.
 * @param {type.ExtensibleModel?} base? - The base element to contain the diagram. (optional)
 * @throws {Error} If the input cannot be parsed or if the base is not a model element.
 */
function createMindmap(input, base) {
  nodes = {};
  const parser = new Parser();
  let db = null;
  try {
    const result = parser.parseMindmap(input);
    db = result.getMindmap();
  } catch (err) {
    console.error(err);
    throw new Error("Failed to parse mindmap");
  }
  const direction = db.direction || "TB";

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

  // create mindmap diagram
  let model = new type.MMMindmap();
  model._parent = base;
  model.name = "Mindmap by Mermaid";
  builder.insert(model);
  builder.fieldInsert(base, "ownedElements", model);
  let diagram = new type.MMMindmapDiagram();
  diagram._parent = model;
  diagram.name = "Mindmap Diagram by Mermaid";
  builder.insert(diagram);
  builder.fieldInsert(model, "ownedElements", diagram);

  // create nodes;
  if (db) {
    addNode(builder, model, diagram, db);
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
  let nodeModel = null;
  nodeModel = new type.MMNode();
  nodeModel.name = data.descr || id;
  nodeModel._parent = model;
  builder.insert(nodeModel);
  builder.fieldInsert(model, "ownedElements", nodeModel);
  // create node view
  let nodeView = new type.MMNodeView();
  nodeView._parent = diagram;
  nodeView.model = nodeModel;
  nodeView.initialize(null, 10, 10, 10, 10);
  builder.insert(nodeView);
  builder.fieldInsert(diagram, "ownedViews", nodeView);
  // register node
  nodes[id] = { model: nodeModel, view: nodeView };
  // add children (recursive)
  if (Array.isArray(data.children)) {
    for (const child of data.children) {
      const childView = addNode(builder, model, diagram, child);
      let edgeModel = new type.MMEdge();
      edgeModel._parent = nodeModel;
      edgeModel.source = childView.model;
      edgeModel.target = nodeModel;
      builder.insert(edgeModel);
      builder.fieldInsert(nodeModel, "ownedElements", edgeModel);
      let edgeView = new type.MMEdgeView();
      edgeView._parent = diagram;
      edgeView.model = edgeModel;
      edgeView.tail = childView;
      edgeView.head = nodeView;
      let sourcePoint = getViewCenter(childView);
      let targetPoint = getViewCenter(nodeView);
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
  }
  return nodeView;
}

module.exports = {
  createMindmap,
};
