const { sanitizeText } = require("../common");

let nodes = [];
let cnt = 0;
let elements = {};

const clear = () => {
  nodes = [];
  cnt = 0;
  elements = {};
};

const getParent = function (level) {
  for (let i = nodes.length - 1; i >= 0; i--) {
    if (nodes[i].level < level) {
      return nodes[i];
    }
  }
  // No parent found
  return null;
};

const getMindmap = () => {
  return nodes.length > 0 ? nodes[0] : null;
};

const addNode = (level, id, descr, type) => {
  var _a, _b, _c, _d;
  let padding = 8;
  // (_b =
  //   (_a = conf.mindmap) === null || _a === void 0 ? void 0 : _a.padding) !==
  //   null && _b !== void 0
  //   ? _b
  //   : defaultConfig.mindmap.padding;
  switch (type) {
    case nodeType.ROUNDED_RECT:
    case nodeType.RECT:
    case nodeType.HEXAGON:
      padding *= 2;
  }
  const node = {
    id: cnt++,
    nodeId: sanitizeText(id),
    level,
    descr: sanitizeText(descr),
    type,
    children: [],
    width: 300,
    // (_d =
    //   (_c = conf.mindmap) === null || _c === void 0
    //     ? void 0
    //     : _c.maxNodeWidth) !== null && _d !== void 0
    //   ? _d
    //   : defaultConfig.mindmap.maxNodeWidth,
    padding,
  };
  const parent = getParent(level);
  if (parent) {
    parent.children.push(node);
    // Keep all nodes in the list
    nodes.push(node);
  } else {
    if (nodes.length === 0) {
      // First node, the root
      nodes.push(node);
    } else {
      // Syntax error ... there can only bee one root
      throw new Error(
        'There can be only one root. No parent could be found for ("' +
          node.descr +
          '")',
      );
    }
  }
};

const nodeType = {
  DEFAULT: 0,
  NO_BORDER: 0,
  ROUNDED_RECT: 1,
  RECT: 2,
  CIRCLE: 3,
  CLOUD: 4,
  BANG: 5,
  HEXAGON: 6,
};

const getType = (startStr, endStr) => {
  switch (startStr) {
    case "[":
      return nodeType.RECT;
    case "(":
      return endStr === ")" ? nodeType.ROUNDED_RECT : nodeType.CLOUD;
    case "((":
      return nodeType.CIRCLE;
    case ")":
      return nodeType.CLOUD;
    case "))":
      return nodeType.BANG;
    case "{{":
      return nodeType.HEXAGON;
    default:
      return nodeType.DEFAULT;
  }
};

const setElementForId = (id, element) => {
  elements[id] = element;
};

const decorateNode = (decoration) => {
  if (!decoration) {
    return;
  }
  const node = nodes[nodes.length - 1];
  if (decoration.icon) {
    node.icon = sanitizeText(decoration.icon);
  }
  if (decoration.class) {
    node.class = sanitizeText(decoration.class);
  }
};

const type2Str = (type) => {
  switch (type) {
    case nodeType.DEFAULT:
      return "no-border";
    case nodeType.RECT:
      return "rect";
    case nodeType.ROUNDED_RECT:
      return "rounded-rect";
    case nodeType.CIRCLE:
      return "circle";
    case nodeType.CLOUD:
      return "cloud";
    case nodeType.BANG:
      return "bang";
    case nodeType.HEXAGON:
      return "hexgon"; // cspell: disable-line
    default:
      return "no-border";
  }
};

// Expose logger to grammar
const getElementById = (id) => elements[id];

const getLogger = () => {
  return {
    error: (msg) => {},
    warn: (msg) => {},
    debug: (msg) => {},
    trace: (msg) => {},
    info: (msg) => {},
    log: (msg) => {},
  };
};

const db = {
  clear,
  addNode,
  getMindmap,
  nodeType,
  getType,
  setElementForId,
  decorateNode,
  type2Str,
  getElementById,
  getLogger,
};

module.exports = db;
