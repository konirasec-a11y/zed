const {
  setAccTitle,
  getAccTitle,
  getAccDescription,
  setAccDescription,
  clear: commonClear,
  setDiagramTitle,
  getDiagramTitle,
} = require("../commonDb");
const { getEdgeId } = require("../utils");

class ErDB {
  constructor() {
    this.entities = new Map();
    this.relationships = [];
    this.classes = new Map();
    this.direction = "TB";
    this.Cardinality = {
      ZERO_OR_ONE: "ZERO_OR_ONE",
      ZERO_OR_MORE: "ZERO_OR_MORE",
      ONE_OR_MORE: "ONE_OR_MORE",
      ONLY_ONE: "ONLY_ONE",
      MD_PARENT: "MD_PARENT",
    };
    this.Identification = {
      NON_IDENTIFYING: "NON_IDENTIFYING",
      IDENTIFYING: "IDENTIFYING",
    };
    this.setAccTitle = setAccTitle;
    this.getAccTitle = getAccTitle;
    this.setAccDescription = setAccDescription;
    this.getAccDescription = getAccDescription;
    this.setDiagramTitle = setDiagramTitle;
    this.getDiagramTitle = getDiagramTitle;
    this.clear();
    this.addEntity = this.addEntity.bind(this);
    this.addAttributes = this.addAttributes.bind(this);
    this.addRelationship = this.addRelationship.bind(this);
    this.setDirection = this.setDirection.bind(this);
    this.addCssStyles = this.addCssStyles.bind(this);
    this.addClass = this.addClass.bind(this);
    this.setClass = this.setClass.bind(this);
    this.setAccTitle = this.setAccTitle.bind(this);
    this.setAccDescription = this.setAccDescription.bind(this);
  }
  /**
   * Add entity
   * @param name - The name of the entity
   * @param alias - The alias of the entity
   */
  addEntity(name, alias = "") {
    var _a, _b;
    if (!this.entities.has(name)) {
      this.entities.set(name, {
        id: `entity-${name}-${this.entities.size}`,
        label: name,
        attributes: [],
        alias,
        shape: "erBox",
        look: "default",
        // look:
        //   (_a = getConfig().look) !== null && _a !== void 0 ? _a : "default",
        cssClasses: "default",
        cssStyles: [],
      });
    } else if (
      !((_b = this.entities.get(name)) === null || _b === void 0
        ? void 0
        : _b.alias) &&
      alias
    ) {
      this.entities.get(name).alias = alias;
    }
    return this.entities.get(name);
  }
  getEntity(name) {
    return this.entities.get(name);
  }
  getEntities() {
    return this.entities;
  }
  getClasses() {
    return this.classes;
  }
  addAttributes(entityName, attribs) {
    const entity = this.addEntity(entityName); // May do nothing (if entity has already been added)
    // Process attribs in reverse order due to effect of recursive construction (last attribute is first)
    let i;
    for (i = attribs.length - 1; i >= 0; i--) {
      if (!attribs[i].keys) {
        attribs[i].keys = [];
      }
      if (!attribs[i].comment) {
        attribs[i].comment = "";
      }
      entity.attributes.push(attribs[i]);
    }
  }
  /**
   * Add a relationship
   *
   * @param entA - The first entity in the relationship
   * @param rolA - The role played by the first entity in relation to the second
   * @param entB - The second entity in the relationship
   * @param rSpec - The details of the relationship between the two entities
   */
  addRelationship(entA, rolA, entB, rSpec) {
    const entityA = this.entities.get(entA);
    const entityB = this.entities.get(entB);
    if (!entityA || !entityB) {
      return;
    }
    const rel = {
      entityA: entityA.id,
      roleA: rolA,
      entityB: entityB.id,
      relSpec: rSpec,
    };
    this.relationships.push(rel);
  }
  getRelationships() {
    return this.relationships;
  }
  getDirection() {
    return this.direction;
  }
  setDirection(dir) {
    this.direction = dir;
  }
  getCompiledStyles(classDefs) {
    var _a, _b;
    let compiledStyles = [];
    for (const customClass of classDefs) {
      const cssClass = this.classes.get(customClass);
      if (cssClass === null || cssClass === void 0 ? void 0 : cssClass.styles) {
        compiledStyles = [
          ...compiledStyles,
          ...((_a = cssClass.styles) !== null && _a !== void 0 ? _a : []),
        ].map((s) => s.trim());
      }
      if (
        cssClass === null || cssClass === void 0 ? void 0 : cssClass.textStyles
      ) {
        compiledStyles = [
          ...compiledStyles,
          ...((_b = cssClass.textStyles) !== null && _b !== void 0 ? _b : []),
        ].map((s) => s.trim());
      }
    }
    return compiledStyles;
  }
  addCssStyles(ids, styles) {
    for (const id of ids) {
      const entity = this.entities.get(id);
      if (!styles || !entity) {
        return;
      }
      for (const style of styles) {
        entity.cssStyles.push(style);
      }
    }
  }
  addClass(ids, style) {
    ids.forEach((id) => {
      let classNode = this.classes.get(id);
      if (classNode === undefined) {
        classNode = { id, styles: [], textStyles: [] };
        this.classes.set(id, classNode);
      }
      if (style) {
        style.forEach(function (s) {
          if (/color/.exec(s)) {
            const newStyle = s.replace("fill", "bgFill");
            classNode.textStyles.push(newStyle);
          }
          classNode.styles.push(s);
        });
      }
    });
  }
  setClass(ids, classNames) {
    for (const id of ids) {
      const entity = this.entities.get(id);
      if (entity) {
        for (const className of classNames) {
          entity.cssClasses += " " + className;
        }
      }
    }
  }
  clear() {
    this.entities = new Map();
    this.classes = new Map();
    this.relationships = [];
    commonClear();
  }
  getData() {
    const nodes = [];
    const edges = [];
    for (const entityKey of this.entities.keys()) {
      const entityNode = this.entities.get(entityKey);
      if (entityNode) {
        entityNode.cssCompiledStyles = this.getCompiledStyles(
          entityNode.cssClasses.split(" "),
        );
        nodes.push(entityNode);
      }
    }
    let count = 0;
    for (const relationship of this.relationships) {
      const edge = {
        id: getEdgeId(relationship.entityA, relationship.entityB, {
          prefix: "id",
          counter: count++,
        }),
        type: "normal",
        curve: "basis",
        start: relationship.entityA,
        end: relationship.entityB,
        label: relationship.roleA,
        labelpos: "c",
        thickness: "normal",
        classes: "relationshipLine",
        arrowTypeStart: relationship.relSpec.cardB.toLowerCase(),
        arrowTypeEnd: relationship.relSpec.cardA.toLowerCase(),
        pattern:
          relationship.relSpec.relType == "IDENTIFYING" ? "solid" : "dashed",
        look: "default", // config.look,
      };
      edges.push(edge);
    }
    return { nodes, edges, other: {}, config, direction: "TB" };
  }
}

module.exports = { ErDB };
