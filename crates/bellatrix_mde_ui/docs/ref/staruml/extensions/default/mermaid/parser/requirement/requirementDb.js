const {
  setAccTitle,
  getAccTitle,
  getAccDescription,
  setAccDescription,
  clear: commonClear,
  setDiagramTitle,
  getDiagramTitle,
} = require("../commonDb");

class RequirementDB {
  constructor() {
    this.relations = [];
    this.latestRequirement = this.getInitialRequirement();
    this.requirements = new Map();
    this.latestElement = this.getInitialElement();
    this.elements = new Map();
    this.classes = new Map();
    this.direction = "TB";
    this.RequirementType = {
      REQUIREMENT: "Requirement",
      FUNCTIONAL_REQUIREMENT: "Functional Requirement",
      INTERFACE_REQUIREMENT: "Interface Requirement",
      PERFORMANCE_REQUIREMENT: "Performance Requirement",
      PHYSICAL_REQUIREMENT: "Physical Requirement",
      DESIGN_CONSTRAINT: "Design Constraint",
    };
    this.RiskLevel = {
      LOW_RISK: "Low",
      MED_RISK: "Medium",
      HIGH_RISK: "High",
    };
    this.VerifyType = {
      VERIFY_ANALYSIS: "Analysis",
      VERIFY_DEMONSTRATION: "Demonstration",
      VERIFY_INSPECTION: "Inspection",
      VERIFY_TEST: "Test",
    };
    this.Relationships = {
      CONTAINS: "contains",
      COPIES: "copies",
      DERIVES: "derives",
      SATISFIES: "satisfies",
      VERIFIES: "verifies",
      REFINES: "refines",
      TRACES: "traces",
    };
    this.setAccTitle = setAccTitle;
    this.getAccTitle = getAccTitle;
    this.setAccDescription = setAccDescription;
    this.getAccDescription = getAccDescription;
    this.setDiagramTitle = setDiagramTitle;
    this.getDiagramTitle = getDiagramTitle;
    this.clear();
    // Needed for JISON since it only supports direct properties
    this.setDirection = this.setDirection.bind(this);
    this.addRequirement = this.addRequirement.bind(this);
    this.setNewReqId = this.setNewReqId.bind(this);
    this.setNewReqRisk = this.setNewReqRisk.bind(this);
    this.setNewReqText = this.setNewReqText.bind(this);
    this.setNewReqVerifyMethod = this.setNewReqVerifyMethod.bind(this);
    this.addElement = this.addElement.bind(this);
    this.setNewElementType = this.setNewElementType.bind(this);
    this.setNewElementDocRef = this.setNewElementDocRef.bind(this);
    this.addRelationship = this.addRelationship.bind(this);
    this.setCssStyle = this.setCssStyle.bind(this);
    this.setClass = this.setClass.bind(this);
    this.defineClass = this.defineClass.bind(this);
    this.setAccTitle = this.setAccTitle.bind(this);
    this.setAccDescription = this.setAccDescription.bind(this);
  }
  getDirection() {
    return this.direction;
  }
  setDirection(dir) {
    this.direction = dir;
  }
  resetLatestRequirement() {
    this.latestRequirement = this.getInitialRequirement();
  }
  resetLatestElement() {
    this.latestElement = this.getInitialElement();
  }
  getInitialRequirement() {
    return {
      requirementId: "",
      text: "",
      risk: "",
      verifyMethod: "",
      name: "",
      type: "",
      cssStyles: [],
      classes: ["default"],
    };
  }
  getInitialElement() {
    return {
      name: "",
      type: "",
      docRef: "",
      cssStyles: [],
      classes: ["default"],
    };
  }
  addRequirement(name, type) {
    if (!this.requirements.has(name)) {
      this.requirements.set(name, {
        name,
        type,
        requirementId: this.latestRequirement.requirementId,
        text: this.latestRequirement.text,
        risk: this.latestRequirement.risk,
        verifyMethod: this.latestRequirement.verifyMethod,
        cssStyles: [],
        classes: ["default"],
      });
    }
    this.resetLatestRequirement();
    return this.requirements.get(name);
  }
  getRequirements() {
    return this.requirements;
  }
  setNewReqId(id) {
    if (this.latestRequirement !== undefined) {
      this.latestRequirement.requirementId = id;
    }
  }
  setNewReqText(text) {
    if (this.latestRequirement !== undefined) {
      this.latestRequirement.text = text;
    }
  }
  setNewReqRisk(risk) {
    if (this.latestRequirement !== undefined) {
      this.latestRequirement.risk = risk;
    }
  }
  setNewReqVerifyMethod(verifyMethod) {
    if (this.latestRequirement !== undefined) {
      this.latestRequirement.verifyMethod = verifyMethod;
    }
  }
  addElement(name) {
    if (!this.elements.has(name)) {
      this.elements.set(name, {
        name,
        type: this.latestElement.type,
        docRef: this.latestElement.docRef,
        cssStyles: [],
        classes: ["default"],
      });
    }
    this.resetLatestElement();
    return this.elements.get(name);
  }
  getElements() {
    return this.elements;
  }
  setNewElementType(type) {
    if (this.latestElement !== undefined) {
      this.latestElement.type = type;
    }
  }
  setNewElementDocRef(docRef) {
    if (this.latestElement !== undefined) {
      this.latestElement.docRef = docRef;
    }
  }
  addRelationship(type, src, dst) {
    this.relations.push({
      type,
      src,
      dst,
    });
  }
  getRelationships() {
    return this.relations;
  }
  clear() {
    this.relations = [];
    this.resetLatestRequirement();
    this.requirements = new Map();
    this.resetLatestElement();
    this.elements = new Map();
    this.classes = new Map();
    commonClear();
  }
  setCssStyle(ids, styles) {
    var _a;
    for (const id of ids) {
      const node =
        (_a = this.requirements.get(id)) !== null && _a !== void 0
          ? _a
          : this.elements.get(id);
      if (!styles || !node) {
        return;
      }
      for (const s of styles) {
        if (s.includes(",")) {
          node.cssStyles.push(...s.split(","));
        } else {
          node.cssStyles.push(s);
        }
      }
    }
  }
  setClass(ids, classNames) {
    var _a, _b;
    for (const id of ids) {
      const node =
        (_a = this.requirements.get(id)) !== null && _a !== void 0
          ? _a
          : this.elements.get(id);
      if (node) {
        for (const _class of classNames) {
          node.classes.push(_class);
          const styles =
            (_b = this.classes.get(_class)) === null || _b === void 0
              ? void 0
              : _b.styles;
          if (styles) {
            node.cssStyles.push(...styles);
          }
        }
      }
    }
  }
  defineClass(ids, style) {
    for (const id of ids) {
      let styleClass = this.classes.get(id);
      if (styleClass === undefined) {
        styleClass = { id, styles: [], textStyles: [] };
        this.classes.set(id, styleClass);
      }
      if (style) {
        style.forEach(function (s) {
          if (/color/.exec(s)) {
            const newStyle = s.replace("fill", "bgFill"); // .replace('color', 'fill');
            styleClass.textStyles.push(newStyle);
          }
          styleClass.styles.push(s);
        });
      }
      this.requirements.forEach((value) => {
        if (value.classes.includes(id)) {
          value.cssStyles.push(...style.flatMap((s) => s.split(",")));
        }
      });
      this.elements.forEach((value) => {
        if (value.classes.includes(id)) {
          value.cssStyles.push(...style.flatMap((s) => s.split(",")));
        }
      });
    }
  }
  getClasses() {
    return this.classes;
  }
  getData() {
    var _a, _b, _c, _d, _e, _f;
    const nodes = [];
    const edges = [];
    for (const requirement of this.requirements.values()) {
      const node = requirement;
      node.id = requirement.name;
      node.cssStyles = requirement.cssStyles;
      node.cssClasses = requirement.classes.join(" ");
      node.shape = "requirementBox";
      node.look = "default"; // config.look;
      nodes.push(node);
    }
    for (const element of this.elements.values()) {
      const node = element;
      node.shape = "requirementBox";
      node.look = "default"; // config.look;
      node.id = element.name;
      node.cssStyles = element.cssStyles;
      node.cssClasses = element.classes.join(" ");
      nodes.push(node);
    }
    for (const relation of this.relations) {
      let counter = 0;
      const isContains = relation.type === this.Relationships.CONTAINS;
      const edge = {
        id: `${relation.src}-${relation.dst}-${counter}`,
        start:
          (_b =
            (_a = this.requirements.get(relation.src)) === null || _a === void 0
              ? void 0
              : _a.name) !== null && _b !== void 0
            ? _b
            : (_c = this.elements.get(relation.src)) === null || _c === void 0
              ? void 0
              : _c.name,
        end:
          (_e =
            (_d = this.requirements.get(relation.dst)) === null || _d === void 0
              ? void 0
              : _d.name) !== null && _e !== void 0
            ? _e
            : (_f = this.elements.get(relation.dst)) === null || _f === void 0
              ? void 0
              : _f.name,
        label: `&lt;&lt;${relation.type}&gt;&gt;`,
        classes: "relationshipLine",
        style: ["fill:none", isContains ? "" : "stroke-dasharray: 10,7"],
        labelpos: "c",
        thickness: "normal",
        type: "normal",
        pattern: isContains ? "normal" : "dashed",
        arrowTypeStart: isContains ? "requirement_contains" : "",
        arrowTypeEnd: isContains ? "" : "requirement_arrow",
        look: "default", // config.look,
      };
      edges.push(edge);
      counter++;
    }
    return { nodes, edges, other: {}, config, direction: this.getDirection() };
  }
}

module.exports = { RequirementDB };
