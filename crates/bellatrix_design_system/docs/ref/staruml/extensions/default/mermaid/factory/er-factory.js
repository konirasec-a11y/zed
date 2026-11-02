const { Parser } = require("../parser/parser");
const { layoutDiagram, getViewCenter, reverseDirection } = require("../utils");

/**
 * @type {Record<string, {model, view}>}
 */
let entities = {};

/**
 * Create a UML ER Diagram from Mermaid input.
 * @param {string} input - The Mermaid ER diagram input.
 * @param {type.UMLPackage?} base? - The base package to contain the diagram. (optional)
 * @throws {Error} If the input cannot be parsed or if the base is not a UMLPackage.
 */
function createErDiagram(input, base) {
  entities = {};
  const parser = new Parser();
  let db = null;
  try {
    db = parser.parseER(input);
  } catch (err) {
    console.error(err);
    throw new Error("Failed to parse ER diagram");
  }
  const direction = reverseDirection(db.direction);

  // prepare an operation builder
  const builder = app.repository.getOperationBuilder();
  builder.begin("add diagram");

  // prepare base model
  if (!base) {
    base = app.project.project;
  } else if (!(base instanceof type.UMLPackage)) {
    builder.discard();
    throw new Error("Base element must be a UMLPackage");
  }

  // create er diagram
  let model = new type.ERDDataModel();
  model._parent = base;
  model.name = "Data Model by Mermaid";
  builder.insert(model);
  builder.fieldInsert(base, "ownedElements", model);
  let diagram = new type.ERDDiagram();
  diagram._parent = model;
  diagram.name = "ER Diagram by Mermaid";
  builder.insert(diagram);
  builder.fieldInsert(model, "ownedElements", diagram);

  // create entities
  for (const entityId of db.entities.keys()) {
    const entityData = db.entities.get(entityId);
    addEntity(builder, model, diagram, entityData);
  }

  // create relationships
  for (const rel of db.relationships) {
    addRelationship(
      builder,
      diagram,
      rel.roleA,
      rel.relSpec === "IDENTIFYING",
      entities[rel.entityA].model,
      entities[rel.entityB].model,
      entities[rel.entityA].view,
      entities[rel.entityB].view,
      rel.relSpec.cardB, // cardinality of entityA-side
      rel.relSpec.cardA, // cardinality of entityB-side
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

  // collapse all entities
  for (const entityId of Object.keys(entities)) {
    const entityModel = entities[entityId].model;
    app.modelExplorer.collapse(entityModel);
  }

  // set current diagram
  app.diagrams.setCurrentDiagram(diagram);
}

function addEntity(builder, base, diagram, data) {
  const entityId = data.id.trim();
  // create entity model
  let entityModel = null;
  entityModel = new type.ERDEntity();
  entityModel.name = data.label || entityId;
  entityModel._parent = base;
  builder.insert(entityModel);
  builder.fieldInsert(base, "ownedElements", entityModel);
  // create entity columns
  for (const attr of data.attributes) {
    addAttribute(builder, entityModel, attr);
  }
  // create entity view
  let entityView = new type.ERDEntityView();
  entityView._parent = diagram;
  entityView.model = entityModel;
  entityView.initialize(null, 10, 10, 10, 10);
  builder.insert(entityView);
  builder.fieldInsert(diagram, "ownedViews", entityView);
  // register entity
  entities[entityId] = { model: entityModel, view: entityView };
}

function addAttribute(builder, entityModel, attrData) {
  let columnModel = null;
  columnModel = new type.ERDColumn();
  columnModel.name = attrData.name;
  columnModel.type = attrData.type;
  columnModel.primaryKey = attrData.keys.includes("PK");
  columnModel.foreignKey = attrData.keys.includes("FK");
  columnModel.unique = attrData.keys.includes("UK");
  columnModel.documentation = attrData.comment;
  columnModel._parent = entityModel;
  builder.insert(columnModel);
  builder.fieldInsert(entityModel, "columns", columnModel);
}

function addRelationship(
  builder,
  diagram,
  name,
  identifying,
  end1,
  end2,
  end1View,
  end2View,
  end1Cardinality,
  end2Cardinality,
) {
  let model = new type.ERDRelationship();
  model.name = name;
  model.identifying = identifying;
  model.end1.reference = end1;
  model.end1.cardinality = getCardinality(end1Cardinality);
  model.end2.reference = end2;
  model.end2.cardinality = getCardinality(end2Cardinality);
  model._parent = end1;
  builder.insert(model);
  builder.fieldInsert(end1, "ownedElements", model);
  // create view element
  let view = new type.ERDRelationshipView();
  view.model = model;
  view._parent = diagram;
  view.tail = end1View;
  view.head = end2View;
  let sourcePoint = getViewCenter(end1View);
  let targetPoint = getViewCenter(end2View);
  view.initialize(
    null,
    sourcePoint[0],
    sourcePoint[1],
    targetPoint[0],
    targetPoint[1],
  );
  builder.insert(view);
  builder.fieldInsert(diagram, "ownedViews", view);
}

function getCardinality(cardinalityData) {
  switch (cardinalityData.trim().toUpperCase()) {
    case "ONLY_ONE":
      return "1";
    case "ZERO_OR_ONE":
      return "0..1";
    case "ZERO_OR_MORE":
      return "0..*";
    case "ONE_OR_MORE":
      return "1..*";
    default:
      return "1";
  }
}

module.exports = {
  createErDiagram,
};
