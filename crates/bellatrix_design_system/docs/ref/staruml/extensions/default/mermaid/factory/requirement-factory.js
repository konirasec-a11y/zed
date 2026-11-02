const { Parser } = require("../parser/parser");
const { layoutDiagram, getViewCenter, reverseDirection } = require("../utils");

/**
 * @type {Record<string, {model, view}>}
 */
let objects = {};

/**
 * Create a Requirement Diagram from Mermaid input.
 * @param {string} input - The Mermaid requirement input.
 * @param {type.UMLPackage?} base? - The base package to contain the diagram. (optional)
 * @throws {Error} If the input cannot be parsed or if the base is not a UMLPackage.
 */
function createRequirementDiagram(input, base) {
  objects = {};
  const parser = new Parser();
  let db = null;
  try {
    db = parser.parseRequirement(input);
  } catch (err) {
    console.error(err);
    throw new Error("Failed to parse requirement diagram");
  }
  const direction = reverseDirection(db.direction);

  // prepare an operation builder
  const builder = app.repository.getOperationBuilder();
  builder.begin("add diagram");

  // prepare base model
  if (!base) {
    const project = app.project.project;
    base = new type.UMLPackage();
    base._parent = project;
    base.name = "Package by Mermaid";
    builder.insert(base);
    builder.fieldInsert(project, "ownedElements", base);
  } else if (!(base instanceof type.UMLPackage)) {
    builder.discard();
    throw new Error("Base element must be a UMLPackage");
  }

  // create class diagram
  let diagram = new type.SysMLRequirementDiagram();
  diagram._parent = base;
  diagram.name = "Requirement Diagram by Mermaid";
  builder.insert(diagram);
  builder.fieldInsert(base, "ownedElements", diagram);

  // create requirements
  for (const reqId of db.requirements.keys()) {
    const reqData = db.requirements.get(reqId);
    addRequirement(builder, base, diagram, reqId, reqData);
  }

  // create elements
  for (const elemId of db.elements.keys()) {
    const elemData = db.elements.get(elemId);
    addElement(builder, base, diagram, elemId, elemData);
  }

  // create relations
  for (const rel of db.relations) {
    if (rel.type === "contains") {
      addContainment(
        builder,
        diagram,
        objects[rel.src].model,
        objects[rel.dst].model,
        objects[rel.src].view,
        objects[rel.dst].view,
      );
    } else {
      addRelation(
        builder,
        diagram,
        rel.type,
        objects[rel.src].model,
        objects[rel.dst].model,
        objects[rel.src].view,
        objects[rel.dst].view,
      );
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

  // collapse all objects
  for (const objId of Object.keys(objects)) {
    const objModel = objects[objId].model;
    app.modelExplorer.collapse(objModel);
  }

  // set current diagram
  app.diagrams.setCurrentDiagram(diagram);
}

function addRequirement(builder, base, diagram, reqId, reqData) {
  // create requirement model
  let reqModel = new type.SysMLRequirement();
  reqModel.name = reqData.name || reqId;
  reqModel.id = reqData.requirementId;
  reqModel.text = reqData.text;
  let comment = "";
  if (reqData.risk) {
    comment += `Risk: ${reqData.risk}\n`;
  }
  if (reqData.verifyMethod) {
    comment += `VerifyMethod: ${reqData.verifyMethod}\n`;
  }
  reqModel.documentation = comment;
  reqModel._parent = base;
  switch (reqData.type) {
    case "Functional Requirement": {
      reqModel.stereotype = "functionalRequirement";
      break;
    }
    case "Performance Requirement": {
      reqModel.stereotype = "performanceRequirement";
      break;
    }
    case "Interface Requirement": {
      reqModel.stereotype = "interfaceRequirement";
      break;
    }
    case "Physical Requirement": {
      reqModel.stereotype = "physicalRequirement";
      break;
    }
    case "Design Constraint": {
      reqModel.stereotype = "designConstraint";
      break;
    }
  }
  builder.insert(reqModel);
  builder.fieldInsert(base, "ownedElements", reqModel);
  // create requirement view
  let reqView = new type.SysMLRequirementView();
  reqView._parent = diagram;
  reqView.model = reqModel;
  reqView.initialize(null, 10, 10, 10, 10);
  builder.insert(reqView);
  builder.fieldInsert(diagram, "ownedViews", reqView);
  // register requirement
  objects[reqId] = { model: reqModel, view: reqView };
}

function addElement(builder, base, diagram, elemId, elemData) {
  // create element model
  let elemModel = new type.UMLClass();
  elemModel.name = elemData.name || elemId;
  elemModel.stereotype = "element";
  // TODO: type
  // TODO: docRef
  elemModel._parent = base;
  builder.insert(elemModel);
  builder.fieldInsert(base, "ownedElements", elemModel);
  // add type attribute
  const typeAttr = new type.UMLAttribute();
  typeAttr.name = "Type";
  typeAttr.defaultValue = elemData.type;
  typeAttr._parent = elemModel;
  builder.insert(typeAttr);
  builder.fieldInsert(elemModel, "attributes", typeAttr);
  // add docRef attribute
  const docRefAttr = new type.UMLAttribute();
  docRefAttr.name = "DocRef";
  docRefAttr.defaultValue = elemData.docRef;
  docRefAttr._parent = elemModel;
  builder.insert(docRefAttr);
  builder.fieldInsert(elemModel, "attributes", docRefAttr);
  // create element view
  let elemView = new type.UMLClassView();
  elemView._parent = diagram;
  elemView.model = elemModel;
  elemView.suppressOperations = true;
  elemView.initialize(null, 10, 10, 10, 10);
  builder.insert(elemView);
  builder.fieldInsert(diagram, "ownedViews", elemView);
  // register element
  objects[elemId] = { model: elemModel, view: elemView };
}

function addRelation(
  builder,
  diagram,
  typeName,
  source,
  target,
  sourceView,
  targetView,
) {
  let model = createRelation(typeName);
  model.source = source;
  model.target = target;
  model._parent = source;
  builder.insert(model);
  builder.fieldInsert(source, "ownedElements", model);
  // create view element
  let view = createRelationView(typeName);
  view.model = model;
  view._parent = diagram;
  view.tail = sourceView;
  view.head = targetView;
  let sourcePoint = getViewCenter(sourceView);
  let targetPoint = getViewCenter(targetView);
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

function addContainment(
  builder,
  diagram,
  source,
  target,
  sourceView,
  targetView,
) {
  // move target into source
  builder.fieldRemove(target._parent, "ownedElements", target);
  builder.fieldInsert(source, "ownedElements", target);
  builder.fieldAssign(target, "_parent", source);
  // create containment view
  let view = new type.UMLContainmentView();
  view.model = null;
  view._parent = diagram;
  view.tail = targetView;
  view.head = sourceView;
  let sourcePoint = getViewCenter(targetView);
  let targetPoint = getViewCenter(sourceView);
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

function createRelation(typeName) {
  switch (typeName.trim()) {
    case "satisfies":
      return new type.SysMLSatisfy();
    case "copies":
      return new type.SysMLCopy();
    case "derives":
      return new type.SysMLDeriveReqt();
    case "verifies":
      return new type.SysMLVerify();
    case "refines":
      return new type.SysMLRefine();
    case "traces": {
      const dep = new type.UMLDependency();
      dep.stereotype = "trace";
      return dep;
    }
  }
  return new type.UMLDependency();
}

function createRelationView(typeName) {
  switch (typeName.trim()) {
    case "satisfies":
      return new type.SysMLSatisfyView();
    case "copies":
      return new type.SysMLCopyView();
    case "derives":
      return new type.SysMLDeriveReqtView();
    case "verifies":
      return new type.SysMLVerifyView();
    case "refines":
      return new type.SysMLRefineView();
    case "traces": {
      return new type.UMLDependencyView();
    }
  }
  return new type.UMLDependencyView();
}

module.exports = {
  createRequirementDiagram,
};
