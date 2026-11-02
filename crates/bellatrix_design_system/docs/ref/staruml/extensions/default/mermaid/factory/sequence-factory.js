const { Parser } = require("../parser/parser");
const { getViewCenter } = require("../utils");

// TODO: Actor
// TODO: Group/Box

const LIFELINE_INTERVAL = 150;
const MESSAGE_INTERVAL = 60;

/**
 * @type {Record<string, {model, view}>}
 */
let lifelines = {};

/**
 * @type {Record<string, {model, view}>}
 */
let messages = {};

/**
 * @type {Array<{model, view}>}
 */
let AllCombinedFragments = [];

/**
 * combined fragment stack
 */
let combinedFragmentStack = [];

/**
 * X coordinate of the lifeline
 * @type {number}
 */
let lifelineX = 25;

/**
 * Y coordinate of the message
 * @type {number}
 */
let messageY = 110;

/**
 * Create a UML Sequence Diagram from Mermaid input.
 * @param {string} input - The Mermaid sequence diagram input.
 * @param {type.UMLPackage? | type.UMLClassifer | type.UMLOperation} base? - The base package to contain the diagram. (optional)
 * @throws {Error} If the input cannot be parsed or if the base is not a UMLPackage, UMLClassifier, or UMLOperation.
 */
function createSequenceDiagram(input, base) {
  lifelines = [];
  combinedFragmentStack = [];
  lifelineX = 25;
  messageY = 110;
  const parser = new Parser();
  let db = null;
  try {
    db = parser.parseSequence(input);
  } catch (err) {
    console.error(err);
    throw new Error("Failed to parse sequence diagram");
  }
  const data = db.state.records;

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

  // create collaboration
  let collaboration = new type.UMLCollaboration();
  collaboration._parent = base;
  collaboration.name = "Collaboration by Mermaid";
  builder.insert(collaboration);
  builder.fieldInsert(base, "ownedElements", collaboration);

  // create interaction
  let interaction = new type.UMLInteraction();
  interaction._parent = collaboration;
  interaction.name = "Interaction by Mermaid";
  builder.insert(interaction);
  builder.fieldInsert(collaboration, "ownedElements", interaction);

  // create sequence diagram
  let hasActivation = data.messages.some(
    (msg) => msg.type === 17 || msg.type === 18, // activation start/end
  );
  let hasSequenceNumber = data.messages.some((msg) => msg.type === 26); // autonumber
  let diagram = new type.UMLSequenceDiagram();
  diagram._parent = interaction;
  diagram.name = "Sequence Diagram by Mermaid";
  diagram.showActivation = hasActivation;
  diagram.showSequenceNumber = hasSequenceNumber;
  builder.insert(diagram);
  builder.fieldInsert(interaction, "ownedElements", diagram);

  // create lifelines
  for (const parId of data.actors.keys()) {
    const parData = data.actors.get(parId);
    addLifeline(builder, collaboration, interaction, diagram, parId, parData);
  }

  // create messages
  for (const msgData of data.messages) {
    let isCreate = false;
    let isDestroy = false;
    if (
      data.createdActors.has(msgData.to) &&
      msgData.id === data.createdActors.get(msgData.to).toString()
    ) {
      isCreate = true;
    }
    if (
      data.destroyedActors.has(msgData.to) &&
      msgData.id === data.destroyedActors.get(msgData.to).toString()
    ) {
      isDestroy = true;
    }
    msgData.isCreate = isCreate;
    msgData.isDestroy = isDestroy;
    addMessage(builder, interaction, diagram, msgData.id, msgData);
  }

  // adjust lifeline heights
  adjustLifelineHeights(builder, diagram);

  // excute operation
  builder.end();
  var cmd = builder.getOperation();
  app.repository.doOperation(cmd);
  diagram = app.repository.get(diagram._id);

  // collapse all combined fragments
  for (const cb of AllCombinedFragments) {
    app.modelExplorer.collapse(cb.model);
  }

  // set current diagram
  app.diagrams.setCurrentDiagram(diagram);
}

function addLifeline(
  builder,
  collaboration,
  interaction,
  diagram,
  parId,
  parData,
) {
  const name = parData.description || parId;
  const isActor = parData.type === "actor";

  // create actor if type is actor
  let actorModel = null;
  if (isActor) {
    actorModel = new type.UMLActor();
    actorModel.name = name;
    actorModel._parent = collaboration;
    builder.insert(actorModel);
    builder.fieldInsert(collaboration, "ownedElements", actorModel);
  }

  // create role model
  let roleModel = new type.UMLAttribute();
  roleModel.name = `${name}Role`;
  if (isActor) roleModel.type = actorModel;
  roleModel._parent = collaboration;
  builder.insert(roleModel);
  builder.fieldInsert(collaboration, "attributes", roleModel);

  // create lifeline model
  let lifelineModel = new type.UMLLifeline();
  lifelineModel.name = name;
  lifelineModel.represent = roleModel;
  lifelineModel._parent = interaction;
  builder.insert(lifelineModel);
  builder.fieldInsert(interaction, "participants", lifelineModel);

  // create lifeline view
  let lifelineView = new type.UMLSeqLifelineView();
  lifelineView._parent = diagram;
  lifelineView.model = lifelineModel;
  if (isActor) lifelineView.stereotypeDisplay = type.UMLGeneralNodeView.SD_ICON;
  lifelineView.initialize(null, lifelineX, 40, lifelineX + 70, 2040);
  builder.insert(lifelineView);
  builder.fieldInsert(diagram, "ownedViews", lifelineView);
  lifelineX += LIFELINE_INTERVAL;

  // register class
  lifelines[parId] = { model: lifelineModel, view: lifelineView };
}

function addMessage(builder, interaction, diagram, msgId, msgData) {
  // create combined fragments
  if (msgData.type === 10 /* LOOP_START */) {
    messageY += 30;
    let cbView = addCombinedFragment(
      builder,
      interaction,
      diagram,
      type.UMLCombinedFragment.IOK_LOOP,
    );
    addOperand(builder, cbView, msgData.message);
    combinedFragmentStack.push(cbView);
  } else if (msgData.type === 11 /* LOOP_END */) {
    let cbView = combinedFragmentStack.pop();
    if (cbView) cbView.bottom = messageY + 20;
  } else if (msgData.type === 12 /* ALT_START */) {
    messageY += 30;
    let cbView = addCombinedFragment(
      builder,
      interaction,
      diagram,
      type.UMLCombinedFragment.IOK_ALT,
    );
    addOperand(builder, cbView, msgData.message);
    combinedFragmentStack.push(cbView);
  } else if (msgData.type === 13 /* ALT_ELSE */) {
    let cbView = combinedFragmentStack[combinedFragmentStack.length - 1];
    if (cbView) addOperand(builder, cbView, msgData.message);
  } else if (msgData.type === 14 /* ALT_END */) {
    let cbView = combinedFragmentStack.pop();
    if (cbView) cbView.bottom = messageY + 20;
  } else if (msgData.type === 15 /* OPT_START */) {
    messageY += 30;
    let cbView = addCombinedFragment(
      builder,
      interaction,
      diagram,
      type.UMLCombinedFragment.IOK_OPT,
    );
    addOperand(builder, cbView, msgData.message);
    combinedFragmentStack.push(cbView);
  } else if (msgData.type === 16 /* OPT_END */) {
    let cbView = combinedFragmentStack.pop();
    if (cbView) cbView.bottom = messageY + 20;
  } else if (msgData.type === 19 /* PAR_START */) {
    messageY += 30;
    let cbView = addCombinedFragment(
      builder,
      interaction,
      diagram,
      type.UMLCombinedFragment.IOK_PAR,
    );
    addOperand(builder, cbView, msgData.message);
    combinedFragmentStack.push(cbView);
  } else if (msgData.type === 20 /* PAR_AND */) {
    let cbView = combinedFragmentStack[combinedFragmentStack.length - 1];
    if (cbView) addOperand(builder, cbView, msgData.message);
  } else if (msgData.type === 21 /* PAR_END */) {
    let cbView = combinedFragmentStack.pop();
    if (cbView) cbView.bottom = messageY + 20;
  } else if (msgData.type === 27 /* CRITICAL_START */) {
    messageY += 30;
    let cbView = addCombinedFragment(
      builder,
      interaction,
      diagram,
      type.UMLCombinedFragment.IOK_CRITICAL,
    );
    addOperand(builder, cbView, msgData.message);
    combinedFragmentStack.push(cbView);
  } else if (msgData.type === 28 /* CRITICAL_OPTION */) {
    let cbView = combinedFragmentStack[combinedFragmentStack.length - 1];
    if (cbView) addOperand(builder, cbView, msgData.message);
  } else if (msgData.type === 29 /* CRITICAL_END */) {
    let cbView = combinedFragmentStack.pop();
    if (cbView) cbView.bottom = messageY + 20;
  } else if (msgData.type === 30 /* BREAK_START */) {
    messageY += 30;
    let cbView = addCombinedFragment(
      builder,
      interaction,
      diagram,
      type.UMLCombinedFragment.IOK_BREAK,
    );
    addOperand(builder, cbView, msgData.message);
    combinedFragmentStack.push(cbView);
  } else if (msgData.type === 31 /* BREAK_END */) {
    let cbView = combinedFragmentStack.pop();
    if (cbView) cbView.bottom = messageY + 20;
  } else if (msgData.type === 2 /* NOTE */) {
    const sv = lifelines[msgData.from].view;
    const tv = lifelines[msgData.to].view;
    const sp = getViewCenter(sv);
    const tp = getViewCenter(tv);
    let x0 = Math.min(sp[0], tp[0]) - 15;
    let x1 = Math.max(sp[0], tp[0]) + 15;
    if (msgData.placement === 0 /* LEFTOF */) {
      const w = 100;
      x0 = Math.min(sp[0], tp[0]) - w - 15;
      x1 = x0 + w;
    } else if (msgData.placement === 1 /* RIGHTOF */) {
      const w = 100;
      x0 = Math.max(sp[0], tp[0]) + 15;
      x1 = x0 + w;
    }
    addNote(builder, diagram, msgData.message, x0, messageY, x1, messageY + 40);
    messageY += MESSAGE_INTERVAL;
  }

  // return if no source or target
  if (!msgData.from || !msgData.to || msgData.type === 2) {
    return;
  }
  const name = msgData.message || msgId;

  // create message model
  let msgModel = new type.UMLMessage();
  msgModel.name = name;
  if (msgData.isCreate) {
    msgModel.messageSort = type.UMLMessage.MS_CREATEMESSAGE;
  } else if (msgData.isDestroy) {
    msgModel.messageSort = type.UMLMessage.MS_DELETEMESSAGE;
  } else if (msgData.type === 1) {
    // convert dotted line with arrowhead to async signal
    msgModel.messageSort = type.UMLMessage.MS_ASYNCHSIGNAL;
  } else if (msgData.type === 24) {
    msgModel.messageSort = type.UMLMessage.MS_ASYNCHCALL;
  } else if (msgData.type === 25) {
    msgModel.messageSort = type.UMLMessage.MS_REPLY;
  }
  msgModel.source = lifelines[msgData.from].model;
  msgModel.target = lifelines[msgData.to].model;
  msgModel._parent = interaction;
  builder.insert(msgModel);
  builder.fieldInsert(interaction, "messages", msgModel);

  // create message view
  const sourceView = lifelines[msgData.from].view.linePart;
  const targetView = lifelines[msgData.to].view.linePart;
  let msgView = new type.UMLSeqMessageView();
  msgView._parent = diagram;
  msgView.model = msgModel;
  msgView.activation.height = MESSAGE_INTERVAL;
  msgView.tail = sourceView;
  msgView.head = targetView;
  let sourcePoint = getViewCenter(sourceView);
  let targetPoint = getViewCenter(targetView);
  msgView.initialize(
    null,
    sourcePoint[0],
    sourcePoint[1],
    targetPoint[0],
    targetPoint[1],
  );
  msgView.points.points.forEach((point) => {
    point.y = messageY;
  });
  builder.insert(msgView);
  builder.fieldInsert(diagram, "ownedViews", msgView);
  messageY += MESSAGE_INTERVAL;

  // adjust combined fragment
  const cbView = combinedFragmentStack[combinedFragmentStack.length - 1];
  if (cbView) {
    const sp = getViewCenter(sourceView._parent);
    const tp = getViewCenter(targetView._parent);
    const x1 = Math.min(sp[0], tp[0]);
    const x2 = Math.max(sp[0], tp[0]);
    const left = x1 - 15;
    const width = Math.abs(x2 - x1) + 30;
    builder.fieldAssign(cbView, "left", left);
    if (cbView.width < width) {
      builder.fieldAssign(cbView, "width", width);
      cbView.width = width;
    }
  }

  // register class
  messages[msgId] = { model: msgModel, view: msgView };
}

function addCombinedFragment(builder, interaction, diagram, op) {
  let cb = new type.UMLCombinedFragment();
  cb._parent = interaction;
  cb.interactionOperator = op;
  builder.insert(cb);
  builder.fieldInsert(interaction, "fragments", cb);
  let cbView = new type.UMLCombinedFragmentView();
  cbView._parent = diagram;
  cbView.model = cb;
  cbView.initialize(null, 10, 10, 10, 10);
  cbView.top = messageY - 55;
  builder.insert(cbView);
  builder.fieldInsert(diagram, "ownedViews", cbView);
  AllCombinedFragments.push({ model: cb, view: cbView });
  return cbView;
}

function addOperand(builder, cbView, guard) {
  let operand = new type.UMLInteractionOperand();
  operand._parent = cbView.model;
  operand.guard = guard;
  builder.insert(operand);
  builder.fieldInsert(cbView.model, "operands", operand);
  let operandView = new type.UMLInteractionOperandView();
  operandView._parent = cbView.operandCompartment;
  operandView.model = operand;
  operandView.initialize(null, 10, 10, 10, 10);
  operandView.height = MESSAGE_INTERVAL;
  builder.insert(operandView);
  builder.fieldInsert(cbView.operandCompartment, "subViews", operandView);
  return operandView;
}

function addNote(builder, diagram, text, x0, y0, x1, y1) {
  let noteView = new type.UMLNoteView();
  noteView._parent = diagram;
  noteView.wordWrap = true;
  noteView.autoResize = false;
  noteView.text = text;
  noteView.initialize(null, x0, y0, x1, y1);
  builder.insert(noteView);
  builder.fieldInsert(diagram, "ownedViews", noteView);
}

function adjustLifelineHeights(builder, diagram) {
  const messageViews = Object.values(messages).map((m) => m.view);
  for (const lifelineId of Object.keys(lifelines)) {
    const lifeline = lifelines[lifelineId];
    const msgs = messageViews.filter(
      (m) =>
        m.tail === lifeline.view.linePart || m.head === lifeline.view.linePart,
    );
    const topY =
      msgs.length > 0 ? Math.min(...msgs.map((m) => m.points.points[0].y)) : 0;
    const bottomY =
      msgs.length > 0 ? Math.max(...msgs.map((m) => m.points.points[0].y)) : 0;
    const minHeight = Math.max(Math.abs(bottomY - topY) + 60, 120);
    builder.fieldAssign(lifeline.view, "height", minHeight);
  }
}

module.exports = {
  createSequenceDiagram,
};
