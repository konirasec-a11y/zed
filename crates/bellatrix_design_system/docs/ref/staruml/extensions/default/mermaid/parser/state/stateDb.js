const { generateId } = require("../utils");
const common = require("../common");
const {
  clear: commonClear,
  getAccDescription,
  getAccTitle,
  getDiagramTitle,
  setAccDescription,
  setAccTitle,
  setDiagramTitle,
} = require("../commonDb");
const { dataFetcher, reset: resetDataFetcher } = require("./dataFetcher");
const {
  DEFAULT_DIAGRAM_DIRECTION,
  DEFAULT_STATE_TYPE,
  DIVIDER_TYPE,
  STMT_APPLYCLASS,
  STMT_CLASSDEF,
  STMT_RELATION,
  STMT_ROOT,
  STMT_DIRECTION,
  STMT_STATE,
  STMT_STYLEDEF,
} = require("./stateCommon");

const CONSTANTS = {
  START_NODE: "[*]",
  START_TYPE: "start",
  END_NODE: "[*]",
  END_TYPE: "end",
  COLOR_KEYWORD: "color",
  FILL_KEYWORD: "fill",
  BG_FILL: "bgFill",
  STYLECLASS_SEP: ",",
};

/**
 * Get the direction from the statement items.
 * Look through all of the documents (docs) in the parsedItems
 * Because is a _document_ direction, the default direction is not necessarily the same as the overall default _diagram_ direction.
 * @param parsedItem - the parsed statement item to look through
 * @param defaultDir - the direction to use if none is found
 * @returns The direction to use
 */
const getDir = (parsedItem, defaultDir = DEFAULT_NESTED_DOC_DIR) => {
  if (!parsedItem.doc) {
    return defaultDir;
  }

  let dir = defaultDir;

  for (const parsedItemDoc of parsedItem.doc) {
    if (parsedItemDoc.stmt === "dir") {
      dir = parsedItemDoc.value;
    }
  }

  return dir;
};

/**
 * Returns a new list of classes.
 * In the future, this can be replaced with a class common to all diagrams.
 * ClassDef information = \{ id: id, styles: [], textStyles: [] \}
 */
const newClassesList = () => new Map();
const newDoc = () => ({
  relations: [],
  states: new Map(),
  documents: {},
});
const clone = (o) => JSON.parse(JSON.stringify(o));
class StateDB {
  constructor(version) {
    this.version = version;
    this.nodes = [];
    this.edges = [];
    this.rootDoc = [];
    this.classes = newClassesList();
    this.documents = { root: newDoc() };
    this.currentDocument = this.documents.root;
    this.startEndCount = 0;
    this.dividerCnt = 0;
    this.getAccTitle = getAccTitle;
    this.setAccTitle = setAccTitle;
    this.getAccDescription = getAccDescription;
    this.setAccDescription = setAccDescription;
    this.setDiagramTitle = setDiagramTitle;
    this.getDiagramTitle = getDiagramTitle;
    this.clear();
    // Bind methods used by JISON
    this.setRootDoc = this.setRootDoc.bind(this);
    this.getDividerId = this.getDividerId.bind(this);
    this.setDirection = this.setDirection.bind(this);
    this.trimColon = this.trimColon.bind(this);
  }
  /**
   * Convert all of the statements (stmts) that were parsed into states and relationships.
   * This is done because a state diagram may have nested sections,
   * where each section is a 'document' and has its own set of statements.
   * Ex: the section within a fork has its own statements, and incoming and outgoing statements
   * refer to the fork as a whole (document).
   * See the parser grammar:  the definition of a document is a document then a 'line', where a line can be a statement.
   * This will push the statement into the list of statements for the current document.
   */
  extract(statements) {
    this.clear(true);
    for (const item of Array.isArray(statements)
      ? statements
      : statements.doc) {
      switch (item.stmt) {
        case STMT_STATE:
          this.addState(
            item.id.trim(),
            item.type,
            item.doc,
            item.description,
            item.note,
          );
          break;
        case STMT_RELATION:
          this.addRelation(item.state1, item.state2, item.description);
          break;
        case STMT_CLASSDEF:
          this.addStyleClass(item.id.trim(), item.classes);
          break;
        case STMT_STYLEDEF:
          this.handleStyleDef(item);
          break;
        case STMT_APPLYCLASS:
          this.setCssClass(item.id.trim(), item.styleClass);
          break;
      }
    }
    const diagramStates = this.getStates();
    resetDataFetcher();
    dataFetcher(
      undefined,
      this.getRootDocV2(),
      diagramStates,
      this.nodes,
      this.edges,
      true,
      "default", // config.look,
      this.classes,
    );
    // Process node labels
    for (const node of this.nodes) {
      if (!Array.isArray(node.label)) {
        continue;
      }
      node.description = node.label.slice(1);
      if (node.isGroup && node.description.length > 0) {
        throw new Error(
          `Group nodes can only have label. Remove the additional description for node [${node.id}]`,
        );
      }
      node.label = node.label[0];
    }
  }
  handleStyleDef(item) {
    const ids = item.id.trim().split(",");
    const styles = item.styleClass.split(",");
    for (const id of ids) {
      let state = this.getState(id);
      if (!state) {
        const trimmedId = id.trim();
        this.addState(trimmedId);
        state = this.getState(trimmedId);
      }
      if (state) {
        state.styles = styles.map((s) => {
          var _a;
          return (_a = s.replace(/;/g, "")) === null || _a === void 0
            ? void 0
            : _a.trim();
        });
      }
    }
  }
  setRootDoc(o) {
    this.rootDoc = o;
    if (this.version === 1) {
      this.extract(o);
    } else {
      this.extract(this.getRootDocV2());
    }
  }
  docTranslator(parent, node, first) {
    if (node.stmt === STMT_RELATION) {
      this.docTranslator(parent, node.state1, true);
      this.docTranslator(parent, node.state2, false);
      return;
    }
    if (node.stmt === STMT_STATE) {
      if (node.id === CONSTANTS.START_NODE) {
        node.id = parent.id + (first ? "_start" : "_end");
        node.start = first;
      } else {
        // This is just a plain state, not a start or end
        node.id = node.id.trim();
      }
    }
    if ((node.stmt !== STMT_ROOT && node.stmt !== STMT_STATE) || !node.doc) {
      return;
    }
    const doc = [];
    // Check for concurrency
    let currentDoc = [];
    for (const stmt of node.doc) {
      if (stmt.type === DIVIDER_TYPE) {
        const newNode = clone(stmt);
        newNode.doc = clone(currentDoc);
        doc.push(newNode);
        currentDoc = [];
      } else {
        currentDoc.push(stmt);
      }
    }
    // If any divider was encountered
    if (doc.length > 0 && currentDoc.length > 0) {
      const newNode = {
        stmt: STMT_STATE,
        id: generateId(),
        type: "divider",
        doc: clone(currentDoc),
      };
      doc.push(clone(newNode));
      node.doc = doc;
    }
    node.doc.forEach((docNode) => this.docTranslator(node, docNode, true));
  }
  getRootDocV2() {
    this.docTranslator(
      { id: STMT_ROOT, stmt: STMT_ROOT },
      { id: STMT_ROOT, stmt: STMT_ROOT, doc: this.rootDoc },
      true,
    );
    return { id: STMT_ROOT, doc: this.rootDoc };
  }
  /**
   * Function called by parser when a node definition has been found.
   *
   * @param descr - description for the state. Can be a string or a list or strings
   * @param classes - class styles to apply to this state. Can be a string (1 style) or an array of styles. If it's just 1 class, convert it to an array of that 1 class.
   * @param styles - styles to apply to this state. Can be a string (1 style) or an array of styles. If it's just 1 style, convert it to an array of that 1 style.
   * @param textStyles - text styles to apply to this state. Can be a string (1 text test) or an array of text styles. If it's just 1 text style, convert it to an array of that 1 text style.
   */
  addState(
    id,
    type = DEFAULT_STATE_TYPE,
    doc = undefined,
    descr = undefined,
    note = undefined,
    classes = undefined,
    styles = undefined,
    textStyles = undefined,
  ) {
    const trimmedId = id === null || id === void 0 ? void 0 : id.trim();
    if (!this.currentDocument.states.has(trimmedId)) {
      this.currentDocument.states.set(trimmedId, {
        stmt: STMT_STATE,
        id: trimmedId,
        descriptions: [],
        type,
        doc,
        note,
        classes: [],
        styles: [],
        textStyles: [],
      });
    } else {
      const state = this.currentDocument.states.get(trimmedId);
      if (!state) {
        throw new Error(`State not found: ${trimmedId}`);
      }
      if (!state.doc) {
        state.doc = doc;
      }
      if (!state.type) {
        state.type = type;
      }
    }
    if (descr) {
      const descriptions = Array.isArray(descr) ? descr : [descr];
      descriptions.forEach((des) => this.addDescription(trimmedId, des.trim()));
    }
    if (note) {
      const doc2 = this.currentDocument.states.get(trimmedId);
      if (!doc2) {
        throw new Error(`State not found: ${trimmedId}`);
      }
      doc2.note = note;
      doc2.note.text = common.sanitizeText(doc2.note.text);
    }
    if (classes) {
      const classesList = Array.isArray(classes) ? classes : [classes];
      classesList.forEach((cssClass) =>
        this.setCssClass(trimmedId, cssClass.trim()),
      );
    }
    if (styles) {
      const stylesList = Array.isArray(styles) ? styles : [styles];
      stylesList.forEach((style) => this.setStyle(trimmedId, style.trim()));
    }
    if (textStyles) {
      const textStylesList = Array.isArray(textStyles)
        ? textStyles
        : [textStyles];
      textStylesList.forEach((textStyle) =>
        this.setTextStyle(trimmedId, textStyle.trim()),
      );
    }
  }
  clear(saveCommon) {
    this.nodes = [];
    this.edges = [];
    this.documents = { root: newDoc() };
    this.currentDocument = this.documents.root;
    // number of start and end nodes; used to construct ids
    this.startEndCount = 0;
    this.classes = newClassesList();
    if (!saveCommon) {
      commonClear();
    }
  }
  getState(id) {
    return this.currentDocument.states.get(id);
  }
  getStates() {
    return this.currentDocument.states;
  }
  logDocuments() {}
  getRelations() {
    return this.currentDocument.relations;
  }
  /**
   * If the id is a start node ( [*] ), then return a new id constructed from
   * the start node name and the current start node count.
   * else return the given id
   */
  startIdIfNeeded(id = "") {
    if (id === CONSTANTS.START_NODE) {
      this.startEndCount++;
      return `${CONSTANTS.START_TYPE}${this.startEndCount}`;
    }
    return id;
  }
  /**
   * If the id is a start node ( [*] ), then return the start type ('start')
   * else return the given type
   */
  startTypeIfNeeded(id = "", type = DEFAULT_STATE_TYPE) {
    return id === CONSTANTS.START_NODE ? CONSTANTS.START_TYPE : type;
  }
  /**
   * If the id is an end node ( [*] ), then return a new id constructed from
   * the end node name and the current start_end node count.
   * else return the given id
   */
  endIdIfNeeded(id = "") {
    if (id === CONSTANTS.END_NODE) {
      this.startEndCount++;
      return `${CONSTANTS.END_TYPE}${this.startEndCount}`;
    }
    return id;
  }
  /**
   * If the id is an end node ( [*] ), then return the end type
   * else return the given type
   *
   */
  endTypeIfNeeded(id = "", type = DEFAULT_STATE_TYPE) {
    return id === CONSTANTS.END_NODE ? CONSTANTS.END_TYPE : type;
  }
  addRelationObjs(item1, item2, relationTitle = "") {
    const id1 = this.startIdIfNeeded(item1.id.trim());
    const type1 = this.startTypeIfNeeded(item1.id.trim(), item1.type);
    const id2 = this.startIdIfNeeded(item2.id.trim());
    const type2 = this.startTypeIfNeeded(item2.id.trim(), item2.type);
    this.addState(
      id1,
      type1,
      item1.doc,
      item1.description,
      item1.note,
      item1.classes,
      item1.styles,
      item1.textStyles,
    );
    this.addState(
      id2,
      type2,
      item2.doc,
      item2.description,
      item2.note,
      item2.classes,
      item2.styles,
      item2.textStyles,
    );
    this.currentDocument.relations.push({
      id1,
      id2,
      relationTitle: common.sanitizeText(relationTitle),
    });
  }
  /**
   * Add a relation between two items.  The items may be full objects or just the string id of a state.
   */
  addRelation(item1, item2, title) {
    if (typeof item1 === "object" && typeof item2 === "object") {
      this.addRelationObjs(item1, item2, title);
    } else if (typeof item1 === "string" && typeof item2 === "string") {
      const id1 = this.startIdIfNeeded(item1.trim());
      const type1 = this.startTypeIfNeeded(item1);
      const id2 = this.endIdIfNeeded(item2.trim());
      const type2 = this.endTypeIfNeeded(item2);
      this.addState(id1, type1);
      this.addState(id2, type2);
      this.currentDocument.relations.push({
        id1,
        id2,
        relationTitle: title ? common.sanitizeText(title) : undefined,
      });
    }
  }
  addDescription(id, descr) {
    var _a;
    const theState = this.currentDocument.states.get(id);
    const _descr = descr.startsWith(":")
      ? descr.replace(":", "").trim()
      : descr;
    (_a =
      theState === null || theState === void 0
        ? void 0
        : theState.descriptions) === null || _a === void 0
      ? void 0
      : _a.push(common.sanitizeText(_descr));
  }
  cleanupLabel(label) {
    return label.startsWith(":") ? label.slice(2).trim() : label.trim();
  }
  getDividerId() {
    this.dividerCnt++;
    return `divider-id-${this.dividerCnt}`;
  }
  /**
   * Called when the parser comes across a (style) class definition
   * @example classDef my-style fill:#f96;
   *
   * @param id - the id of this (style) class
   * @param styleAttributes - the string with 1 or more style attributes (each separated by a comma)
   */
  addStyleClass(id, styleAttributes = "") {
    // create a new style class object with this id
    if (!this.classes.has(id)) {
      this.classes.set(id, { id, styles: [], textStyles: [] });
    }
    const foundClass = this.classes.get(id);
    if (styleAttributes && foundClass) {
      styleAttributes.split(CONSTANTS.STYLECLASS_SEP).forEach((attrib) => {
        const fixedAttrib = attrib.replace(/([^;]*);/, "$1").trim();
        if (RegExp(CONSTANTS.COLOR_KEYWORD).exec(attrib)) {
          const newStyle1 = fixedAttrib.replace(
            CONSTANTS.FILL_KEYWORD,
            CONSTANTS.BG_FILL,
          );
          const newStyle2 = newStyle1.replace(
            CONSTANTS.COLOR_KEYWORD,
            CONSTANTS.FILL_KEYWORD,
          );
          foundClass.textStyles.push(newStyle2);
        }
        foundClass.styles.push(fixedAttrib);
      });
    }
  }
  getClasses() {
    return this.classes;
  }
  /**
   * Add a (style) class or css class to a state with the given id.
   * If the state isn't already in the list of known states, add it.
   * Might be called by parser when a style class or CSS class should be applied to a state
   *
   * @param itemIds - The id or a list of ids of the item(s) to apply the css class to
   * @param cssClassName - CSS class name
   */
  setCssClass(itemIds, cssClassName) {
    itemIds.split(",").forEach((id) => {
      var _a;
      let foundState = this.getState(id);
      if (!foundState) {
        const trimmedId = id.trim();
        this.addState(trimmedId);
        foundState = this.getState(trimmedId);
      }
      (_a =
        foundState === null || foundState === void 0
          ? void 0
          : foundState.classes) === null || _a === void 0
        ? void 0
        : _a.push(cssClassName);
    });
  }
  /**
   * Add a style to a state with the given id.
   * @example style stateId fill:#f9f,stroke:#333,stroke-width:4px
   *   where 'style' is the keyword
   *   stateId is the id of a state
   *   the rest of the string is the styleText (all of the attributes to be applied to the state)
   *
   * @param itemId - The id of item to apply the style to
   * @param styleText - the text of the attributes for the style
   */
  setStyle(itemId, styleText) {
    var _a, _b;
    (_b =
      (_a = this.getState(itemId)) === null || _a === void 0
        ? void 0
        : _a.styles) === null || _b === void 0
      ? void 0
      : _b.push(styleText);
  }
  /**
   * Add a text style to a state with the given id
   *
   * @param itemId - The id of item to apply the css class to
   * @param cssClassName - CSS class name
   */
  setTextStyle(itemId, cssClassName) {
    var _a, _b;
    (_b =
      (_a = this.getState(itemId)) === null || _a === void 0
        ? void 0
        : _a.textStyles) === null || _b === void 0
      ? void 0
      : _b.push(cssClassName);
  }
  /**
   * Finds the direction statement in the root document.
   * @returns the direction statement if present
   */
  getDirectionStatement() {
    return this.rootDoc.find((doc) => doc.stmt === STMT_DIRECTION);
  }
  getDirection() {
    var _a, _b;
    return (_b =
      (_a = this.getDirectionStatement()) === null || _a === void 0
        ? void 0
        : _a.value) !== null && _b !== void 0
      ? _b
      : DEFAULT_DIAGRAM_DIRECTION;
  }
  setDirection(dir) {
    const doc = this.getDirectionStatement();
    if (doc) {
      doc.value = dir;
    } else {
      this.rootDoc.unshift({ stmt: STMT_DIRECTION, value: dir });
    }
  }
  trimColon(str) {
    return str.startsWith(":") ? str.slice(1).trim() : str.trim();
  }
  getData() {
    return {
      nodes: this.nodes,
      edges: this.edges,
      other: {},
      config,
      direction: getDir(this.getRootDocV2()),
    };
  }
}
StateDB.relationType = {
  AGGREGATION: 0,
  EXTENSION: 1,
  COMPOSITION: 2,
  DEPENDENCY: 3,
};

module.exports = { StateDB };
