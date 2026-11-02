/**
 * Constants common to all State Diagram code
 */
// default diagram direction
const DEFAULT_DIAGRAM_DIRECTION = "TB";
// default direction for any nested documents (composites)
const DEFAULT_NESTED_DOC_DIR = "TB";
// parsed statement type for a direction
const STMT_DIRECTION = "dir";
// parsed statement type for a state
const STMT_STATE = "state";
// parsed statement type for a root
const STMT_ROOT = "root";
// parsed statement type for a relation
const STMT_RELATION = "relation";
// parsed statement type for a classDef
const STMT_CLASSDEF = "classDef";
const STMT_STYLEDEF = "style";
// parsed statement type for applyClass
const STMT_APPLYCLASS = "applyClass";
const DEFAULT_STATE_TYPE = "default";
const DIVIDER_TYPE = "divider";
// Graph edge settings
const G_EDGE_STYLE = "fill:none";
const G_EDGE_ARROWHEADSTYLE = "fill: #333";
const G_EDGE_LABELPOS = "c";
const G_EDGE_LABELTYPE = "text";
const G_EDGE_THICKNESS = "normal";
const SHAPE_STATE = "rect";
const SHAPE_STATE_WITH_DESC = "rectWithTitle";
const SHAPE_START = "stateStart";
const SHAPE_END = "stateEnd";
const SHAPE_DIVIDER = "divider";
const SHAPE_GROUP = "roundedWithTitle";
const SHAPE_NOTE = "note";
const SHAPE_NOTEGROUP = "noteGroup";
// CSS classes
const CSS_DIAGRAM = "statediagram";
const CSS_STATE = "state";
const CSS_DIAGRAM_STATE = `${CSS_DIAGRAM}-${CSS_STATE}`;
const CSS_EDGE = "transition";
const CSS_NOTE = "note";
const CSS_NOTE_EDGE = "note-edge";
const CSS_EDGE_NOTE_EDGE = `${CSS_EDGE} ${CSS_NOTE_EDGE}`;
const CSS_DIAGRAM_NOTE = `${CSS_DIAGRAM}-${CSS_NOTE}`;
const CSS_CLUSTER = "cluster";
const CSS_DIAGRAM_CLUSTER = `${CSS_DIAGRAM}-${CSS_CLUSTER}`;
const CSS_CLUSTER_ALT = "cluster-alt";
const CSS_DIAGRAM_CLUSTER_ALT = `${CSS_DIAGRAM}-${CSS_CLUSTER_ALT}`;
const PARENT = "parent";
const NOTE = "note";
const DOMID_STATE = "state";
const DOMID_TYPE_SPACER = "----";
const NOTE_ID = `${DOMID_TYPE_SPACER}${NOTE}`;
const PARENT_ID = `${DOMID_TYPE_SPACER}${PARENT}`;
// --------------------------------------
module.exports = {
  DEFAULT_DIAGRAM_DIRECTION,
  DEFAULT_NESTED_DOC_DIR,
  STMT_DIRECTION,
  STMT_STATE,
  STMT_ROOT,
  STMT_RELATION,
  STMT_CLASSDEF,
  STMT_STYLEDEF,
  STMT_APPLYCLASS,
  DEFAULT_STATE_TYPE,
  DIVIDER_TYPE,
  G_EDGE_STYLE,
  G_EDGE_ARROWHEADSTYLE,
  G_EDGE_LABELPOS,
  G_EDGE_LABELTYPE,
  G_EDGE_THICKNESS,
  SHAPE_STATE,
  SHAPE_STATE_WITH_DESC,
  SHAPE_START,
  SHAPE_END,
  SHAPE_DIVIDER,
  SHAPE_GROUP,
  SHAPE_NOTE,
  SHAPE_NOTEGROUP,
  CSS_EDGE,
  CSS_DIAGRAM,
  CSS_STATE,
  CSS_DIAGRAM_STATE,
  CSS_NOTE,
  CSS_NOTE_EDGE,
  CSS_EDGE_NOTE_EDGE,
  CSS_DIAGRAM_NOTE,
  CSS_CLUSTER,
  CSS_DIAGRAM_CLUSTER,
  CSS_CLUSTER_ALT,
  CSS_DIAGRAM_CLUSTER_ALT,
  PARENT,
  NOTE,
  DOMID_STATE,
  DOMID_TYPE_SPACER,
  NOTE_ID,
  PARENT_ID,
};
