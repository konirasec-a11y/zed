const { sanitizeText } = require("../common");
const utils = require("../utils");
const {
  setAccTitle,
  getAccTitle,
  getAccDescription,
  setAccDescription,
  clear: commonClear,
  setDiagramTitle,
  getDiagramTitle,
} = require("../commonDb");
const { ClassMember } = require("./classTypes");

const MERMAID_DOM_ID_PREFIX = "classId-";

let classCounter = 0;

class ClassDB {
  constructor() {
    this.relations = [];
    this.classes = new Map();
    this.styleClasses = new Map();
    this.notes = [];
    this.interfaces = [];
    // private static classCounter = 0;
    this.namespaces = new Map();
    this.namespaceCounter = 0;
    this.functions = [];
    this.lineType = {
      LINE: 0,
      DOTTED_LINE: 1,
    };
    this.relationType = {
      AGGREGATION: 0,
      EXTENSION: 1,
      COMPOSITION: 2,
      DEPENDENCY: 3,
      LOLLIPOP: 4,
    };
    this.direction = "TB";
    this.setAccTitle = setAccTitle;
    this.getAccTitle = getAccTitle;
    this.setAccDescription = setAccDescription;
    this.getAccDescription = getAccDescription;
    this.setDiagramTitle = setDiagramTitle;
    this.getDiagramTitle = getDiagramTitle;
    // this.getConfig = () => getConfig().class;
    // this.functions.push(this.setupToolTips.bind(this));
    this.clear();
    // Needed for JISON since it only supports direct properties
    this.addRelation = this.addRelation.bind(this);
    this.addClassesToNamespace = this.addClassesToNamespace.bind(this);
    this.addNamespace = this.addNamespace.bind(this);
    this.setCssClass = this.setCssClass.bind(this);
    this.addMembers = this.addMembers.bind(this);
    this.addClass = this.addClass.bind(this);
    this.setClassLabel = this.setClassLabel.bind(this);
    this.addAnnotation = this.addAnnotation.bind(this);
    this.addMember = this.addMember.bind(this);
    this.cleanupLabel = this.cleanupLabel.bind(this);
    this.addNote = this.addNote.bind(this);
    this.defineClass = this.defineClass.bind(this);
    this.setDirection = this.setDirection.bind(this);
    this.setLink = this.setLink.bind(this);
    this.bindFunctions = this.bindFunctions.bind(this);
    this.clear = this.clear.bind(this);
    this.setTooltip = this.setTooltip.bind(this);
    this.setClickEvent = this.setClickEvent.bind(this);
    this.setCssStyle = this.setCssStyle.bind(this);
  }

  splitClassNameAndType(_id) {
    const id = sanitizeText(_id);
    let genericType = "";
    let className = id;
    if (id.indexOf("~") > 0) {
      const split = id.split("~");
      className = sanitizeText(split[0]);
      genericType = sanitizeText(split[1]);
    }
    return { className: className, type: genericType };
  }

  setClassLabel(_id, label) {
    const id = sanitizeText(_id);
    if (label) {
      label = sanitizeText(label);
    }
    const { className } = this.splitClassNameAndType(id);
    this.classes.get(className).label = label;
    this.classes.get(className).text = `${label}${
      this.classes.get(className).type
        ? `<${this.classes.get(className).type}>`
        : ""
    }`;
  }

  /**
   * Function called by parser when a node definition has been found.
   *
   * @param id - ID of the class to add
   * @public
   */
  addClass(_id) {
    const id = sanitizeText(_id);
    const { className, type } = this.splitClassNameAndType(id);
    // Only add class if not exists
    if (this.classes.has(className)) {
      return;
    }
    // alert('Adding class: ' + className);
    const name = sanitizeText(className);
    // alert('Adding class after: ' + name);
    this.classes.set(name, {
      id: name,
      type: type,
      label: name,
      text: `${name}${type ? `&lt;${type}&gt;` : ""}`,
      shape: "classBox",
      cssClasses: "default",
      methods: [],
      members: [],
      annotations: [],
      styles: [],
      domId: MERMAID_DOM_ID_PREFIX + name + "-" + classCounter,
    });
    classCounter++;
  }

  addInterface(label, classId) {
    const classInterface = {
      id: `interface${this.interfaces.length}`,
      label,
      classId,
    };
    this.interfaces.push(classInterface);
  }

  /**
   * Function to lookup domId from id in the graph definition.
   *
   * @param id - class ID to lookup
   * @public
   */
  lookUpDomId(_id) {
    const id = sanitizeText(_id);
    if (this.classes.has(id)) {
      return this.classes.get(id).domId;
    }
    throw new Error("Class not found: " + id);
  }

  clear() {
    this.relations = [];
    this.classes = new Map();
    this.notes = [];
    this.interfaces = [];
    this.functions = [];
    // this.functions.push(this.setupToolTips.bind(this));
    this.namespaces = new Map();
    this.namespaceCounter = 0;
    this.direction = "TB";
    commonClear();
  }

  getClass(id) {
    return this.classes.get(id);
  }

  getClasses() {
    return this.classes;
  }

  getRelations() {
    return this.relations;
  }

  getNotes() {
    return this.notes;
  }

  addRelation(classRelation) {
    // Due to relationType cannot just check if it is equal to 'none' or it complains, can fix this later
    const invalidTypes = [
      this.relationType.LOLLIPOP,
      this.relationType.AGGREGATION,
      this.relationType.COMPOSITION,
      this.relationType.DEPENDENCY,
      this.relationType.EXTENSION,
    ];
    if (
      classRelation.relation.type1 === this.relationType.LOLLIPOP &&
      !invalidTypes.includes(classRelation.relation.type2)
    ) {
      this.addClass(classRelation.id2);
      this.addInterface(classRelation.id1, classRelation.id2);
      classRelation.id1 = `interface${this.interfaces.length - 1}`;
    } else if (
      classRelation.relation.type2 === this.relationType.LOLLIPOP &&
      !invalidTypes.includes(classRelation.relation.type1)
    ) {
      this.addClass(classRelation.id1);
      this.addInterface(classRelation.id2, classRelation.id1);
      classRelation.id2 = `interface${this.interfaces.length - 1}`;
    } else {
      this.addClass(classRelation.id1);
      this.addClass(classRelation.id2);
    }
    classRelation.id1 = this.splitClassNameAndType(classRelation.id1).className;
    classRelation.id2 = this.splitClassNameAndType(classRelation.id2).className;
    classRelation.relationTitle1 = sanitizeText(
      classRelation.relationTitle1.trim(),
    );
    classRelation.relationTitle2 = sanitizeText(
      classRelation.relationTitle2.trim(),
    );
    this.relations.push(classRelation);
  }

  /**
   * Adds an annotation to the specified class Annotations mark special properties of the given type
   * (like 'interface' or 'service')
   *
   * @param className - The class name
   * @param annotation - The name of the annotation without any brackets
   * @public
   */
  addAnnotation(className, annotation) {
    const validatedClassName = this.splitClassNameAndType(className).className;
    this.classes.get(validatedClassName).annotations.push(annotation);
  }

  /**
   * Adds a member to the specified class
   *
   * @param className - The class name
   * @param member - The full name of the member. If the member is enclosed in `<<brackets>>` it is
   *   treated as an annotation If the member is ending with a closing bracket ) it is treated as a
   *   method Otherwise the member will be treated as a normal property
   * @public
   */
  addMember(className, member) {
    this.addClass(className);
    const validatedClassName = this.splitClassNameAndType(className).className;
    const theClass = this.classes.get(validatedClassName);
    if (typeof member === "string") {
      // Member can contain white spaces, we trim them out
      const memberString = member.trim();
      if (memberString.startsWith("<<") && memberString.endsWith(">>")) {
        // its an annotation
        theClass.annotations.push(
          sanitizeText(memberString.substring(2, memberString.length - 2)),
        );
      } else if (memberString.indexOf(")") > 0) {
        //its a method
        theClass.methods.push(new ClassMember(memberString, "method"));
      } else if (memberString) {
        theClass.members.push(new ClassMember(memberString, "attribute"));
      }
    }
  }

  addMembers(className, members) {
    if (Array.isArray(members)) {
      members.reverse();
      members.forEach((member) => this.addMember(className, member));
    }
  }

  addNote(text, className) {
    const note = {
      id: `note${this.notes.length}`,
      class: className,
      text: text,
    };
    this.notes.push(note);
  }

  cleanupLabel(label) {
    if (label.startsWith(":")) {
      label = label.substring(1);
    }
    return sanitizeText(label.trim());
  }

  /**
   * Called by parser when assigning cssClass to a class
   *
   * @param ids - Comma separated list of ids
   * @param className - Class to add
   */
  setCssClass(ids, className) {
    ids.split(",").forEach((_id) => {
      let id = _id;
      if (/\d/.exec(_id[0])) {
        id = MERMAID_DOM_ID_PREFIX + id;
      }
      const classNode = this.classes.get(id);
      if (classNode) {
        classNode.cssClasses += " " + className;
      }
    });
  }

  defineClass(ids, style) {
    for (const id of ids) {
      let styleClass = this.styleClasses.get(id);
      if (styleClass === undefined) {
        styleClass = { id, styles: [], textStyles: [] };
        this.styleClasses.set(id, styleClass);
      }
      if (style) {
        style.forEach((s) => {
          if (/color/.exec(s)) {
            const newStyle = s.replace("fill", "bgFill"); // .replace('color', 'fill');
            styleClass.textStyles.push(newStyle);
          }
          styleClass.styles.push(s);
        });
      }
      this.classes.forEach((value) => {
        if (value.cssClasses.includes(id)) {
          value.styles.push(...style.flatMap((s) => s.split(",")));
        }
      });
    }
  }

  /**
   * Called by parser when a tooltip is found, e.g. a clickable element.
   *
   * @param ids - Comma separated list of ids
   * @param tooltip - Tooltip to add
   */
  setTooltip(ids, tooltip) {
    ids.split(",").forEach((id) => {
      if (tooltip !== undefined) {
        this.classes.get(id).tooltip = sanitizeText(tooltip);
      }
    });
  }

  getTooltip(id, namespace) {
    if (namespace && this.namespaces.has(namespace)) {
      return this.namespaces.get(namespace).classes.get(id).tooltip;
    }
    return this.classes.get(id).tooltip;
  }

  /**
   * Called by parser when a link is found. Adds the URL to the vertex data.
   *
   * @param ids - Comma separated list of ids
   * @param linkStr - URL to create a link for
   * @param target - Target of the link, _blank by default as originally defined in the svgDraw.js file
   */
  setLink(ids, linkStr, target) {
    ids.split(",").forEach((_id) => {
      let id = _id;
      if (/\d/.exec(_id[0])) {
        id = MERMAID_DOM_ID_PREFIX + id;
      }
      const theClass = this.classes.get(id);
      if (theClass) {
        theClass.link = utils.formatUrl(linkStr, conffig);
        if (typeof target === "string") {
          theClass.linkTarget = sanitizeText(target);
        } else {
          theClass.linkTarget = "_blank";
        }
      }
    });
    this.setCssClass(ids, "clickable");
  }

  /**
   * Called by parser when a click definition is found. Registers an event handler.
   *
   * @param ids - Comma separated list of ids
   * @param functionName - Function to be called on click
   * @param functionArgs - Function args the function should be called with
   */
  setClickEvent(ids, functionName, functionArgs) {
    ids.split(",").forEach((id) => {
      this.setClickFunc(id, functionName, functionArgs);
      this.classes.get(id).haveCallback = true;
    });
    this.setCssClass(ids, "clickable");
  }

  setClickFunc(_domId, functionName, functionArgs) {
    const domId = sanitizeText(_domId);
    if (functionName === undefined) {
      return;
    }
    const id = domId;
    if (this.classes.has(id)) {
      const elemId = this.lookUpDomId(id);
      let argList = [];
      if (typeof functionArgs === "string") {
        /* Splits functionArgs by ',', ignoring all ',' in double quoted strings */
        argList = functionArgs.split(/,(?=(?:(?:[^"]*"){2})*[^"]*$)/);
        for (let i = 0; i < argList.length; i++) {
          let item = argList[i].trim();
          /* Removes all double quotes at the start and end of an argument */
          /* This preserves all starting and ending whitespace inside */
          if (item.startsWith('"') && item.endsWith('"')) {
            item = item.substr(1, item.length - 2);
          }
          argList[i] = item;
        }
      }
      /* if no arguments passed into callback, default to passing in id */
      if (argList.length === 0) {
        argList.push(elemId);
      }
      this.functions.push(() => {
        const elem = document.querySelector(`[id="${elemId}"]`);
        if (elem !== null) {
          elem.addEventListener(
            "click",
            () => {
              utils.runFunc(functionName, ...argList);
            },
            false,
          );
        }
      });
    }
  }

  bindFunctions(element) {
    this.functions.forEach((fun) => {
      fun(element);
    });
  }

  getDirection() {
    return this.direction;
  }

  setDirection(dir) {
    this.direction = dir;
  }

  /**
   * Function called by parser when a namespace definition has been found.
   *
   * @param id - ID of the namespace to add
   * @public
   */
  addNamespace(id) {
    if (this.namespaces.has(id)) {
      return;
    }
    this.namespaces.set(id, {
      id: id,
      classes: new Map(),
      children: {},
      domId: MERMAID_DOM_ID_PREFIX + id + "-" + this.namespaceCounter,
    });
    this.namespaceCounter++;
  }

  getNamespace(name) {
    return this.namespaces.get(name);
  }

  getNamespaces() {
    return this.namespaces;
  }

  /**
   * Function called by parser when a namespace definition has been found.
   *
   * @param id - ID of the namespace to add
   * @param classNames - IDs of the class to add
   * @public
   */
  addClassesToNamespace(id, classNames) {
    if (!this.namespaces.has(id)) {
      return;
    }
    for (const name of classNames) {
      const { className } = this.splitClassNameAndType(name);
      this.classes.get(className).parent = id;
      this.namespaces
        .get(id)
        .classes.set(className, this.classes.get(className));
    }
  }

  setCssStyle(id, styles) {
    const thisClass = this.classes.get(id);
    if (!styles || !thisClass) {
      return;
    }
    for (const s of styles) {
      if (s.includes(",")) {
        thisClass.styles.push(...s.split(","));
      } else {
        thisClass.styles.push(s);
      }
    }
  }

  /**
   * Gets the arrow marker for a type index
   *
   * @param type - The type to look for
   * @returns The arrow marker
   */
  getArrowMarker(type) {
    let marker;
    switch (type) {
      case 0:
        marker = "aggregation";
        break;
      case 1:
        marker = "extension";
        break;
      case 2:
        marker = "composition";
        break;
      case 3:
        marker = "dependency";
        break;
      case 4:
        marker = "lollipop";
        break;
      default:
        marker = "none";
    }
    return marker;
  }

  getData() {
    var _a, _b, _c, _d;
    const nodes = [];
    const edges = [];
    for (const namespaceKey of this.namespaces.keys()) {
      const namespace = this.namespaces.get(namespaceKey);
      if (namespace) {
        const node = {
          id: namespace.id,
          label: namespace.id,
          isGroup: true,
          // padding:
          //   (_a = config?.class.padding) !== null && _a !== void 0 ? _a : 16,
          // // parent node must be one of [rect, roundedWithTitle, noteGroup, divider]
          shape: "rect",
          // cssStyles: ["fill: none", "stroke: black"],
          // look: config.look,
        };
        nodes.push(node);
      }
    }
    for (const classKey of this.classes.keys()) {
      const classNode = this.classes.get(classKey);
      if (classNode) {
        const node = classNode;
        node.parentId = classNode.parent;
        // node.look = config.look;
        nodes.push(node);
      }
    }
    let cnt = 0;
    for (const note of this.notes) {
      cnt++;
      const noteNode = {
        id: note.id,
        label: note.text,
        isGroup: false,
        shape: "note",
        // padding: (_b = config.class.padding) !== null && _b !== void 0 ? _b : 6,
        // cssStyles: [
        //   "text-align: left",
        //   "white-space: nowrap",
        //   `fill: ${config.themeVariables.noteBkgColor}`,
        //   `stroke: ${config.themeVariables.noteBorderColor}`,
        // ],
        // look: config.look,
      };
      nodes.push(noteNode);
      const noteClassId =
        (_d =
          (_c = this.classes.get(note.class)) === null || _c === void 0
            ? void 0
            : _c.id) !== null && _d !== void 0
          ? _d
          : "";
      if (noteClassId) {
        const edge = {
          id: `edgeNote${cnt}`,
          start: note.id,
          end: noteClassId,
          type: "normal",
          thickness: "normal",
          classes: "relation",
          arrowTypeStart: "none",
          arrowTypeEnd: "none",
          arrowheadStyle: "",
          labelStyle: [""],
          style: ["fill: none"],
          pattern: "dotted",
          // look: config.look,
        };
        edges.push(edge);
      }
    }
    for (const _interface of this.interfaces) {
      const interfaceNode = {
        id: _interface.id,
        label: _interface.label,
        isGroup: false,
        shape: "rect",
        cssStyles: ["opacity: 0;"],
        // look: config.look,
      };
      nodes.push(interfaceNode);
    }
    cnt = 0;
    for (const classRelation of this.relations) {
      cnt++;
      const edge = {
        id: utils.getEdgeId(classRelation.id1, classRelation.id2, {
          prefix: "id",
          counter: cnt,
        }),
        start: classRelation.id1,
        end: classRelation.id2,
        type: "normal",
        label: classRelation.title,
        labelpos: "c",
        thickness: "normal",
        classes: "relation",
        arrowTypeStart: this.getArrowMarker(classRelation.relation.type1),
        arrowTypeEnd: this.getArrowMarker(classRelation.relation.type2),
        startLabelRight:
          classRelation.relationTitle1 === "none"
            ? ""
            : classRelation.relationTitle1,
        endLabelLeft:
          classRelation.relationTitle2 === "none"
            ? ""
            : classRelation.relationTitle2,
        arrowheadStyle: "",
        labelStyle: ["display: inline-block"],
        style: classRelation.style || "",
        pattern: classRelation.relation.lineType == 1 ? "dashed" : "solid",
        // look: config.look,
      };
      edges.push(edge);
    }
    return { nodes, edges, other: {}, config, direction: this.getDirection() };
  }
}

module.exports = { ClassDB };
