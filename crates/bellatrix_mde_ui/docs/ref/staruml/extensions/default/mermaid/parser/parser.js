const flowParser = require("./flowchart/flowchart").parser;
const sequenceParser = require("./sequence/sequenceDiagram").parser;
const classParser = require("./class/classDiagram").parser;
const stateParser = require("./state/stateDiagram").parser;
const erParser = require("./er/erDiagram").parser;
const requirementParser = require("./requirement/requirementDiagram").parser;
const mindmapParser = require("./mindmap/mindmap").parser;

const { FlowDB } = require("./flowchart/flowDb");
const { SequenceDB } = require("./sequence/sequenceDb");
const { ClassDB } = require("./class/classDB");
const { StateDB } = require("./state/stateDb");
const { ErDB } = require("./er/erDb");
const { RequirementDB } = require("./requirement/requirementDb");
const MindmapDB = require("./mindmap/mindmapDb");

class Parser {
  parseFlowchart(input) {
    flowParser.yy = new FlowDB();
    flowParser.yy.clear();
    flowParser.parse(input.trim());
    return flowParser.yy;
  }

  parseSequence(input) {
    sequenceParser.yy = new SequenceDB();
    sequenceParser.yy.clear();
    sequenceParser.parse(input.trim());
    return sequenceParser.yy;
  }

  parseClass(input) {
    classParser.yy = new ClassDB();
    classParser.yy.clear();
    classParser.parse(input.trim());
    return classParser.yy;
  }

  parseState(input) {
    stateParser.yy = new StateDB();
    stateParser.yy.clear();
    stateParser.parse(input.trim());
    return stateParser.yy;
  }

  parseER(input) {
    erParser.yy = new ErDB();
    erParser.yy.clear();
    erParser.parse(input.trim());
    return erParser.yy;
  }

  parseRequirement(input) {
    requirementParser.yy = new RequirementDB();
    requirementParser.yy.clear();
    requirementParser.parse(input.trim());
    return requirementParser.yy;
  }

  parseMindmap(input) {
    mindmapParser.yy = MindmapDB;
    mindmapParser.yy.clear();
    mindmapParser.parse(input.trim());
    return mindmapParser.yy;
  }
}

module.exports = {
  Parser,
};
