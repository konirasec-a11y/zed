const { Parser } = require("../parser/parser");
const { parseGenericTypes } = require("../parser/common");
const { layoutDiagram, getViewCenter, createViewOf } = require("../utils");

// TODO: show namespaces (packages) in the diagram

/**
 * @type {Record<string, {model, view}>}
 */
let namespaces = {};

/**
 * @type {Record<string, {model, view}>}
 */
let classes = {};

/**
 * Create a UML Class Diagram from Mermaid input.
 * @param {string} input - The Mermaid class diagram input.
 * @param {type.UMLPackage?} base? - The base package to contain the diagram. (optional)
 * @throws {Error} If the input cannot be parsed or if the base is not a UMLPackage.
 */
function createClassDiagram(input, base) {
  namespaces = {};
  classes = {};
  const parser = new Parser();
  let db = null;
  try {
    db = parser.parseClass(input);
  } catch (err) {
    console.error(err);
    throw new Error("Failed to parse class diagram");
  }
  const direction = db.direction || "TB";

  // prepare an operation builder
  const builder = app.repository.getOperationBuilder();
  builder.begin("add diagram");

  // prepare base model
  if (!base || base instanceof type.Project) {
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
  let diagram = new type.UMLClassDiagram();
  diagram._parent = base;
  diagram.name = "Class Diagram by Mermaid";
  builder.insert(diagram);
  builder.fieldInsert(base, "ownedElements", diagram);

  // create namespaces
  for (const nsId of db.namespaces.keys()) {
    const nsData = db.namespaces.get(nsId);
    addNamespace(builder, base, diagram, nsData);
  }

  // create classes
  for (const classId of db.classes.keys()) {
    const classData = db.classes.get(classId);
    let container = base;
    if (classData.parent && namespaces[classData.parent]) {
      container = namespaces[classData.parent].model;
    }
    addClass(builder, container, diagram, classData);
  }

  // create interfaces (for lollipop)
  for (const intf of db.interfaces) {
    addInterface(builder, base, diagram, intf);
  }

  // create relationships
  for (const rel of db.relations) {
    const source = classes[rel.id1];
    const target = classes[rel.id2];
    // type1 or type2: 0 is diamond, 1 is triangle, 2 is filled-diamond, 3 is arrow
    // lineType: 0 is solid, 1 is dashed
    if (rel.relation.lineType === 0) {
      if (rel.relation.type2 === 1) {
        addGeneralization(
          builder,
          diagram,
          rel.title,
          source.model,
          target.model,
          source.view,
          target.view,
        );
      } else if (rel.relation.type1 === 1) {
        addGeneralization(
          builder,
          diagram,
          rel.title,
          target.model,
          source.model,
          target.view,
          source.view,
        );
      } else if (rel.relation.type2 === 4) {
        addRealization(
          builder,
          diagram,
          rel.title,
          source.model,
          target.model,
          source.view,
          target.view,
        );
      } else if (rel.relation.type1 === 4) {
        addRealization(
          builder,
          diagram,
          rel.title,
          target.model,
          source.model,
          target.view,
          source.view,
        );
      } else {
        let end1Navigable = "unspecified";
        let end2Navigable = "unspecified";
        let end1Aggregation = "none";
        let end2Aggregation = "none";
        if (rel.relation.type1 === 0) end1Aggregation = "shared";
        if (rel.relation.type2 === 0) end2Aggregation = "shared";
        if (rel.relation.type1 === 2) end1Aggregation = "composite";
        if (rel.relation.type2 === 2) end2Aggregation = "composite";
        if (rel.relation.type1 === 3) end1Navigable = "navigable";
        if (rel.relation.type2 === 3) end2Navigable = "navigable";
        addAssociation(
          builder,
          diagram,
          rel.title,
          source.model,
          target.model,
          source.view,
          target.view,
          end1Navigable,
          end2Navigable,
          end1Aggregation,
          end2Aggregation,
          rel.relationTitle1,
          rel.relationTitle2,
        );
      }
    } else if (rel.relation.lineType === 1) {
      if (rel.relation.type2 === 1) {
        addRealization(
          builder,
          diagram,
          rel.title,
          source.model,
          target.model,
          source.view,
          target.view,
        );
      } else if (rel.relation.type1 === 1) {
        addRealization(
          builder,
          diagram,
          rel.title,
          target.model,
          source.model,
          target.view,
          source.view,
        );
      } else if (rel.relation.type2 === 3) {
        addDependency(
          builder,
          diagram,
          rel.title,
          source.model,
          target.model,
          source.view,
          target.view,
        );
      } else if (rel.relation.type1 === 3) {
        addDependency(
          builder,
          diagram,
          rel.title,
          target.model,
          source.model,
          target.view,
          source.view,
        );
      } else {
        // use dependency for other types
        addDependency(
          builder,
          diagram,
          rel.title,
          source.model,
          target.model,
          source.view,
          target.view,
        );
      }
    }
  }

  // create notes
  for (const note of db.notes) {
    addNote(builder, diagram, note);
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

  // collapse all classes
  for (const classId of db.classes.keys()) {
    const classModel = classes[classId].model;
    app.modelExplorer.collapse(classModel);
  }

  // set current diagram
  app.diagrams.setCurrentDiagram(diagram);
}

function addNamespace(builder, base, diagram, data) {
  const nsId = data.id.trim();
  let nsModel = new type.UMLPackage();
  nsModel.name = nsId;
  nsModel._parent = base;
  builder.insert(nsModel);
  builder.fieldInsert(base, "ownedElements", nsModel);
  namespaces[nsId] = { model: nsModel };
}

function addClass(builder, base, diagram, data) {
  const classId = data.id.trim();
  const annotations = data.annotations.map((a) => a.toLowerCase().trim());
  // create class model
  let classModel = null;
  if (annotations.includes("interface")) {
    classModel = new type.UMLInterface();
  } else if (annotations.includes("enumeration")) {
    classModel = new type.UMLEnumeration();
  } else {
    classModel = new type.UMLClass();
    if (annotations.length > 0) classModel.stereotype = annotations[0];
  }
  if (annotations.includes("abstract")) {
    classModel.isAbstract = true;
  }
  classModel.name = data.label || classId;
  classModel._parent = base;
  builder.insert(classModel);
  builder.fieldInsert(base, "ownedElements", classModel);
  // create generic type
  if (data.type.length > 0) {
    const typeParams = data.type.split(",").map((t) => t.trim());
    for (let i = 0; i < typeParams.length; i++) {
      let typeParameter = new type.UMLTemplateParameter();
      typeParameter.name = typeParams[i];
      typeParameter._parent = classModel;
      builder.insert(typeParameter);
      builder.fieldInsert(classModel, "templateParameters", typeParameter);
    }
  }
  // create class attributes
  for (const attr of data.members) {
    addAttribute(builder, classModel, attr);
  }
  // create class operations
  for (const op of data.methods) {
    addOperation(builder, classModel, op);
  }
  // create class view
  let classView = createViewOf(builder, diagram, classModel, false);
  // register class
  classes[classId] = { model: classModel, view: classView };
}

function addInterface(builder, base, diagram, data) {
  const id = data.id.trim();
  // create interface model
  let intfModel = null;
  intfModel = new type.UMLInterface();
  intfModel.name = data.label || id;
  intfModel._parent = base;
  builder.insert(intfModel);
  builder.fieldInsert(base, "ownedElements", intfModel);
  // create interface view
  let intfView = new type.UMLInterfaceView();
  intfView._parent = diagram;
  intfView.model = intfModel;
  intfView.stereotypeDisplay = type.UMLGeneralNodeView.SD_ICON;
  builder.insert(intfView);
  builder.fieldInsert(diagram, "ownedViews", intfView);
  // register interface
  classes[id] = { model: intfModel, view: intfView };
}

function addAttribute(builder, classModel, attrData) {
  const id = attrData.id.trim();
  const varDef = parseVarDef(id);
  let attrModel = null;
  if (classModel instanceof type.UMLEnumeration) {
    attrModel = new type.UMLEnumerationLiteral();
  } else {
    attrModel = new type.UMLAttribute();
  }
  attrModel.name = varDef.name;
  attrModel.type = parseGenericTypes(varDef.type);
  attrModel.visibility = getVisibility(attrData.visibility);
  attrModel._parent = classModel;
  builder.insert(attrModel);
  if (classModel instanceof type.UMLEnumeration) {
    builder.fieldInsert(classModel, "literals", attrModel);
  } else {
    builder.fieldInsert(classModel, "attributes", attrModel);
  }
}

function addOperation(builder, classModel, opData) {
  const id = opData.id.trim();
  const params = opData.parameters.trim();
  const returnType = opData.returnType.trim();
  let opModel = new type.UMLOperation();
  opModel.name = id;
  opModel.visibility = getVisibility(opData.visibility);
  opModel.returnType = returnType;
  opModel._parent = classModel;
  builder.insert(opModel);
  builder.fieldInsert(classModel, "operations", opModel);
  if (params.length > 0) {
    const paramsList = params.split(",");
    for (const param of paramsList) {
      const varDef = parseVarDef(param.trim());
      let paramModel = new type.UMLParameter();
      paramModel.direction = type.UMLParameter.DK_IN;
      paramModel.name = varDef.name;
      paramModel.type = parseGenericTypes(varDef.type);
      paramModel._parent = opModel;
      builder.insert(paramModel);
      builder.fieldInsert(opModel, "parameters", paramModel);
    }
  }
  if (returnType.length > 0) {
    let returnModel = new type.UMLParameter();
    returnModel.direction = type.UMLParameter.DK_RETURN;
    returnModel.type = parseGenericTypes(returnType);
    returnModel._parent = opModel;
    builder.insert(returnModel);
    builder.fieldInsert(opModel, "parameters", returnModel);
  }
}

function addGeneralization(
  builder,
  diagram,
  name,
  source,
  target,
  sourceView,
  targetView,
) {
  let model = new type.UMLGeneralization();
  model.name = name;
  model.source = source;
  model.target = target;
  model._parent = source;
  builder.insert(model);
  builder.fieldInsert(source, "ownedElements", model);
  // create view element
  let view = new type.UMLGeneralizationView();
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

function addRealization(
  builder,
  diagram,
  name,
  source,
  target,
  sourceView,
  targetView,
) {
  let model = new type.UMLInterfaceRealization();
  model.name = name;
  model.source = source;
  model.target = target;
  model._parent = source;
  builder.insert(model);
  builder.fieldInsert(source, "ownedElements", model);
  // create view element
  let view = new type.UMLInterfaceRealizationView();
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

function addDependency(
  builder,
  diagram,
  name,
  source,
  target,
  sourceView,
  targetView,
) {
  let model = new type.UMLDependency();
  model.name = name;
  model.source = source;
  model.target = target;
  model._parent = source;
  builder.insert(model);
  builder.fieldInsert(source, "ownedElements", model);
  // create view element
  let view = new type.UMLDependencyView();
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

function addAssociation(
  builder,
  diagram,
  name,
  end1,
  end2,
  end1View,
  end2View,
  end1Navigable,
  end2Navigable,
  end1Aggregation,
  end2Aggregation,
  end1Multiplicity,
  end2Multiplicity,
) {
  let model = new type.UMLAssociation();
  model.name = name;
  model.end1.reference = end1;
  model.end1.navigable = end1Navigable;
  model.end1.aggregation = end1Aggregation;
  model.end1.multiplicity = end1Multiplicity;
  model.end2.reference = end2;
  model.end2.navigable = end2Navigable;
  model.end2.aggregation = end2Aggregation;
  model.end2.multiplicity = end2Multiplicity;
  model._parent = end1;
  builder.insert(model);
  builder.fieldInsert(end1, "ownedElements", model);
  // create view element
  let view = new type.UMLAssociationView();
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

function addNote(builder, diagram, noteData) {
  let noteView = new type.UMLNoteView();
  noteView._parent = diagram;
  noteView.wordWrap = false;
  noteView.autoResize = true;
  noteView.text = noteData.text;
  builder.insert(noteView);
  builder.fieldInsert(diagram, "ownedViews", noteView);
  if (noteData.class && classes[noteData.class]) {
    const classView = classes[noteData.class].view;
    if (classView) {
      let noteLinkView = new type.UMLNoteLinkView();
      noteLinkView._parent = diagram;
      noteLinkView.head = noteView;
      noteLinkView.tail = classView;
      let tailPoint = getViewCenter(classView);
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

function getVisibility(visibilityData) {
  switch (visibilityData) {
    case "+":
      return type.UMLModelElement.VK_PUBLIC;
    case "#":
      return type.UMLModelElement.VK_PROTECTED;
    case "-":
      return type.UMLModelElement.VK_PRIVATE;
    case "~":
      return type.UMLModelElement.VK_PACKAGE;
    default:
      return type.UMLModelElement.VK_PACKAGE;
  }
}

/**
 * Parse variable definition (e.g. "int a", "a")
 */
function parseVarDef(variableDef) {
  const result = {
    name: "",
    type: "",
  };
  const terms = variableDef.split(" ");
  if (terms.length === 1) {
    result.name = terms[0];
  } else if (terms.length > 1) {
    result.type = terms[0];
    result.name = terms[1];
  }
  return result;
}

module.exports = {
  createClassDiagram,
};
