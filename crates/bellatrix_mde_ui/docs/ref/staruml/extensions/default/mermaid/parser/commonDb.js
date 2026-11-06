let accTitle = "";
let diagramTitle = "";
let accDescription = "";

const sanitizeText = (txt) => {
  return txt; // _sanitizeText(txt, getConfig());
};

const clear = () => {
  accTitle = "";
  accDescription = "";
  diagramTitle = "";
};

const setAccTitle = (txt) => {
  accTitle = sanitizeText(txt).replace(/^\s+/g, "");
};

const getAccTitle = () => accTitle;

const setAccDescription = (txt) => {
  accDescription = sanitizeText(txt).replace(/\n\s+/g, "\n");
};

const getAccDescription = () => accDescription;

const setDiagramTitle = (txt) => {
  diagramTitle = sanitizeText(txt);
};

const getDiagramTitle = () => diagramTitle;

module.exports = {
  clear,
  setAccTitle,
  getAccTitle,
  setAccDescription,
  getAccDescription,
  setDiagramTitle,
  getDiagramTitle,
};
