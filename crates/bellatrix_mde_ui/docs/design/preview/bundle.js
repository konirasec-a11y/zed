(function(){
const __modules={};const __cache={};
__modules["src/app/App.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.App = void 0;
const React = __importStar(require("react"));
const Topbar_1 = require("../components/layout/Topbar");
const ActivityRail_1 = require("../components/layout/ActivityRail");
const LeftSidebar_1 = require("../components/layout/LeftSidebar");
const RightInspector_1 = require("../components/layout/RightInspector");
const ResizeHandle_1 = require("../components/dock/ResizeHandle");
const Statusbar_1 = require("../components/layout/Statusbar");
const CanvasShell_1 = require("../components/canvas/CanvasShell");
const ComponentGallery_1 = require("../components/gallery/ComponentGallery");
const DesignSystemView_1 = require("../components/gallery/DesignSystemView");
const catalog_1 = require("../components/palette/catalog");
const diagramData_1 = require("./diagramData");
const defaultLayout = {
    leftWidth: 246, rightWidth: 292, projectsHeight: 216, explorerHeight: 230, bottomHeight: 206,
    flipped: false, leftOpen: true, rightOpen: true, projectsOpen: true, toolboxOpen: true,
    explorerOpen: true, inspectorOpen: true, bottomOpen: false, focus: false, minimap: false,
};
const LAYOUT_KEY = 'staruml-ui-layout-v3';
const THEME_KEY = 'staruml-ui-theme-v3';
function clamp(value, min, max) { return Math.max(min, Math.min(value, Math.max(min, max))); }
function readLayout() {
    try {
        const raw = localStorage.getItem(LAYOUT_KEY);
        if (!raw)
            return { ...defaultLayout, rightOpen: window.innerWidth >= 1150 };
        const saved = JSON.parse(raw);
        const valid = Object.entries(defaultLayout).filter(([key, initial]) => typeof saved[key] === typeof initial);
        return { ...defaultLayout, ...Object.fromEntries(valid.map(([key]) => [key, saved[key]])) };
    }
    catch {
        return { ...defaultLayout, rightOpen: typeof window !== 'undefined' && window.innerWidth >= 1150 };
    }
}
function readTheme() {
    try {
        return localStorage.getItem(THEME_KEY) === 'light' ? 'light' : 'dark';
    }
    catch {
        return 'dark';
    }
}
class App extends React.Component {
    constructor() {
        super(...arguments);
        this.state = {
            family: 'uml', theme: readTheme(), view: 'editor', search: '', chosen: '',
            inspector: 'properties', bottomTab: 'relationships', collapsed: {}, layout: readLayout(),
        };
        this.patchLayout = (patch) => this.setState(s => ({ layout: { ...s.layout, ...patch } }));
        this.resize = (key, delta, min, max) => this.setState(s => ({ layout: { ...s.layout, [key]: clamp(s.layout[key] + delta, min, max) } }));
        this.activateFamily = (family) => this.setState({ family, search: '', chosen: '' });
        this.leftToggle = () => {
            const l = this.state.layout;
            this.patchLayout({ leftOpen: !l.leftOpen || l.focus,
                projectsOpen: !l.projectsOpen && !l.toolboxOpen ? true : l.projectsOpen,
                toolboxOpen: !l.projectsOpen && !l.toolboxOpen ? true : l.toolboxOpen, focus: false });
        };
        this.rightToggle = () => {
            const l = this.state.layout;
            this.patchLayout({ rightOpen: !l.rightOpen || l.focus,
                explorerOpen: !l.explorerOpen && !l.inspectorOpen ? true : l.explorerOpen,
                inspectorOpen: !l.explorerOpen && !l.inspectorOpen ? true : l.inspectorOpen, focus: false });
        };
        this.applyPreset = (preset) => {
            if (preset === 'focus')
                this.patchLayout({ focus: true });
            else if (preset === 'review')
                this.setState(s => ({ bottomTab: 'documentation', layout: { ...s.layout, focus: false, leftOpen: true, projectsOpen: true, toolboxOpen: false, rightOpen: true, explorerOpen: true, inspectorOpen: true, bottomOpen: true } }));
            else
                this.patchLayout({ focus: false, leftOpen: true, rightOpen: true, projectsOpen: true, toolboxOpen: true, explorerOpen: true, inspectorOpen: true, bottomOpen: false });
        };
    }
    componentDidUpdate(_oldProps, previous) {
        try {
            if (previous.layout !== this.state.layout)
                localStorage.setItem(LAYOUT_KEY, JSON.stringify(this.state.layout));
            if (previous.theme !== this.state.theme)
                localStorage.setItem(THEME_KEY, this.state.theme);
        }
        catch { /* File-scheme previews may disable storage. */ }
    }
    render() {
        const { family, theme, view, search, chosen, inspector, bottomTab, collapsed, layout: l } = this.state;
        const selected = catalog_1.catalog[family].flatMap(g => g.items).find(i => i.id === chosen);
        const editor = view === 'editor';
        const showLeft = editor && l.leftOpen && !l.focus && (l.projectsOpen || l.toolboxOpen);
        const showRight = editor && l.rightOpen && !l.focus && (l.explorerOpen || l.inspectorOpen);
        const showBottom = l.bottomOpen && !l.focus;
        const leftLimit = () => Math.max(210, window.innerWidth - 48 - (showRight ? l.rightWidth : 0) - 410);
        const rightLimit = () => Math.max(242, window.innerWidth - 48 - (showLeft ? l.leftWidth : 0) - 410);
        const startWidth = l.flipped ? (showRight ? l.rightWidth : 0) : (showLeft ? l.leftWidth : 0);
        const endWidth = l.flipped ? (showLeft ? l.leftWidth : 0) : (showRight ? l.rightWidth : 0);
        const startVisible = l.flipped ? showRight : showLeft;
        const endVisible = l.flipped ? showLeft : showRight;
        return React.createElement("div", { className: "app-root", "data-theme": theme },
            React.createElement(Topbar_1.Topbar, { theme: theme, diagram: diagramData_1.diagrams.find(d => d.family === family).label, flipped: l.flipped, minimap: l.minimap, onTheme: t => this.setState({ theme: t }), onPreset: this.applyPreset, onFlip: () => this.patchLayout({ flipped: !l.flipped }), onMinimap: () => this.patchLayout({ minimap: !l.minimap }), onReset: () => this.patchLayout({ ...defaultLayout }) }),
            React.createElement("div", { className: `main-frame ${l.flipped ? 'is-reversed' : ''}`, style: { gridTemplateColumns: `48px ${startVisible ? `${startWidth}px 5px` : '0px 0px'} minmax(0, 1fr) ${endVisible ? `5px ${endWidth}px` : '0px 0px'}` } },
                React.createElement(ActivityRail_1.ActivityRail, { view: view, onView: v => this.setState({ view: v }) }),
                showLeft && React.createElement(React.Fragment, null,
                    React.createElement(LeftSidebar_1.LeftSidebar, { family: family, onFamily: this.activateFamily, search: search, onSearch: s => this.setState({ search: s }), chosen: chosen, onChoose: s => this.setState({ chosen: s }), collapsed: collapsed, onToggleGroup: id => this.setState({ collapsed: { ...collapsed, [id]: !collapsed[id] } }), projectsOpen: l.projectsOpen, toolboxOpen: l.toolboxOpen, onProjectsToggle: () => this.patchLayout({ projectsOpen: !l.projectsOpen }), onToolboxToggle: () => this.patchLayout({ toolboxOpen: !l.toolboxOpen }), projectsHeight: l.projectsHeight, onProjectsResize: d => this.resize('projectsHeight', d, 85, Math.max(145, window.innerHeight - 330)), onProjectsReset: () => this.patchLayout({ projectsHeight: 202 }) }),
                    React.createElement(ResizeHandle_1.ResizeHandle, { className: "layout-split-left", axis: "horizontal", label: "Largura da toolbox", value: l.leftWidth, min: 210, max: leftLimit(), onDelta: d => this.resize('leftWidth', l.flipped ? -d : d, 210, leftLimit()), onReset: () => this.patchLayout({ leftWidth: 246 }) })),
                React.createElement("div", { className: "center-column" }, editor ? React.createElement(CanvasShell_1.CanvasShell, { family: family, onFamily: this.activateFamily, leftVisible: showLeft, rightVisible: showRight, bottomVisible: showBottom, focusMode: l.focus, onLeftToggle: this.leftToggle, onRightToggle: this.rightToggle, onBottomToggle: () => this.patchLayout({ bottomOpen: !l.bottomOpen, focus: false }), onFocusToggle: () => this.patchLayout({ focus: !l.focus }), onLayoutReset: () => this.patchLayout({ ...defaultLayout }), bottomHeight: l.bottomHeight, onBottomResize: d => this.resize('bottomHeight', -d, 98, Math.max(180, window.innerHeight - 290)), onBottomReset: () => this.patchLayout({ bottomHeight: 206 }), bottomTab: bottomTab, onBottomTab: tab => this.setState({ bottomTab: tab }), minimapVisible: l.minimap, onMinimapToggle: () => this.patchLayout({ minimap: !l.minimap }) })
                    : view === 'library' ? React.createElement(ComponentGallery_1.ComponentGallery, { family: family, onFamily: this.activateFamily, chosen: chosen, onChoose: s => this.setState({ chosen: s }) })
                        : React.createElement(DesignSystemView_1.DesignSystemView, { theme: theme })),
                showRight && React.createElement(React.Fragment, null,
                    React.createElement(ResizeHandle_1.ResizeHandle, { className: "layout-split-right", axis: "horizontal", label: "Largura do inspetor", value: l.rightWidth, min: 242, max: rightLimit(), onDelta: d => this.resize('rightWidth', l.flipped ? d : -d, 242, rightLimit()), onReset: () => this.patchLayout({ rightWidth: 292 }) }),
                    React.createElement(RightInspector_1.RightInspector, { family: family, tab: inspector, onTab: t => this.setState({ inspector: t }), explorerOpen: l.explorerOpen, inspectorOpen: l.inspectorOpen, explorerHeight: l.explorerHeight, onExplorerToggle: () => this.patchLayout({ explorerOpen: !l.explorerOpen }), onInspectorToggle: () => this.patchLayout({ inspectorOpen: !l.inspectorOpen }), onExplorerResize: d => this.resize('explorerHeight', d, 80, Math.max(180, window.innerHeight - 300)), onExplorerReset: () => this.patchLayout({ explorerHeight: 263 }) }))),
            view === 'library' && selected && React.createElement("div", { className: "selected-toast" },
                selected.label,
                React.createElement("span", null, " \u00B7 exemplo")),
            React.createElement(Statusbar_1.Statusbar, { family: family }));
    }
}
exports.App = App;

};
__modules["src/app/diagramData.ts"] = function(require,module,exports){
"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.diagramTypes = exports.elements = exports.diagrams = exports.families = void 0;
exports.families = [
    { id: 'uml', name: 'UML 2.5', shorthand: 'UML', description: 'Software design & behavior' },
    { id: 'c4', name: 'C4 Model', shorthand: 'C4', description: 'Architecture at four levels' },
    { id: 'mindmap', name: 'Mind Map', shorthand: 'MIND', description: 'Topics and relationships' },
    { id: 'flowchart', name: 'Flowchart', shorthand: 'FLOW', description: 'Processes and decisions' }
];
exports.diagrams = [
    { family: 'uml', label: 'Domain model', type: 'Class diagram', name: 'Domain model' },
    { family: 'c4', label: 'System architecture', type: 'Container diagram', name: 'System architecture' },
    { family: 'mindmap', label: 'Product discovery', type: 'Mind map', name: 'Product discovery' },
    { family: 'flowchart', label: 'Order processing', type: 'Flowchart', name: 'Order processing' }
];
exports.elements = {
    uml: { name: 'OrderService', kind: 'Class', namespace: 'commerce.domain', notes: 'Coordinates order creation and validation.' },
    c4: { name: 'Order API', kind: 'Container', namespace: 'Commerce platform', notes: 'HTTP API for order management.' },
    mindmap: { name: 'Product strategy', kind: 'Central topic', namespace: 'Product discovery', notes: 'Planning branches and research topics.' },
    flowchart: { name: 'Validate input', kind: 'Decision', namespace: 'Order processing', notes: 'Checks whether the request is valid.' }
};
// Scope index for the library view, independent of editor behavior.
exports.diagramTypes = {
    uml: ['Class', 'Package', 'Object', 'Composite structure', 'Component', 'Deployment', 'Use case',
        'Sequence', 'Communication', 'Timing', 'Interaction overview', 'State machine', 'Activity',
        'Information flow', 'Profile'],
    c4: ['System landscape', 'System context (L1)', 'Container (L2)', 'Component (L3)',
        'Code (L4)', 'Dynamic', 'Deployment'],
    mindmap: ['Radial mind map', 'Tree mind map', 'Concept map'],
    flowchart: ['Basic flowchart', 'Cross-functional flowchart', 'Swimlane flowchart']
};

};
__modules["src/app/types.ts"] = function(require,module,exports){
"use strict";
Object.defineProperty(exports, "__esModule", { value: true });

};
__modules["src/components/canvas/CanvasShell.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.CanvasShell = void 0;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
const Controls_1 = require("../primitives/Controls");
const ResizeHandle_1 = require("../dock/ResizeHandle");
const diagramData_1 = require("../../app/diagramData");
const ClassDiagram_1 = require("../diagrams/ClassDiagram");
const C4Diagram_1 = require("../diagrams/C4Diagram");
const MindMapDiagram_1 = require("../diagrams/MindMapDiagram");
const FlowDiagram_1 = require("../diagrams/FlowDiagram");
const BottomPanel_1 = require("../layout/BottomPanel");
class CanvasShell extends React.Component {
    constructor() {
        super(...arguments);
        this.state = { tool: 'select' };
    }
    render() {
        const p = this.props, diagram = diagramData_1.diagrams.find(d => d.family === p.family);
        return React.createElement("main", { className: "workspace-main" },
            React.createElement("nav", { className: "document-tabs", "aria-label": "Open diagrams" },
                React.createElement("div", { className: "doc-tab-area" }, diagramData_1.diagrams.map(d => React.createElement("button", { key: d.family, className: `doc-tab ${p.family === d.family ? 'active' : ''}`, "aria-current": p.family === d.family ? 'page' : undefined, onClick: () => p.onFamily(d.family) },
                    React.createElement(Icon_1.Icon, { name: d.family === 'uml' ? 'box' : d.family === 'c4' ? 'layers' : d.family === 'mindmap' ? 'tree' : 'corner', size: 14 }),
                    React.createElement("span", null, d.label)))),
                React.createElement("div", { className: "document-tab-overflow", title: "Additional open tabs" },
                    React.createElement(Icon_1.Icon, { name: "more", size: 15 }))),
            React.createElement("div", { className: "canvas-toolbar" },
                React.createElement("div", { className: "canvas-path" },
                    React.createElement("span", null, "Model"),
                    React.createElement(Icon_1.Icon, { name: "chevron-right", size: 11 }),
                    React.createElement("strong", null, diagram.label),
                    React.createElement("span", { className: "canvas-type-label" }, diagram.type)),
                React.createElement("div", { className: "canvas-toolbar-controls", "aria-label": "Tools and panels" },
                    React.createElement("div", { className: "canvas-tool-pair", role: "group", "aria-label": "Visual tool" },
                        React.createElement("button", { className: this.state.tool === 'select' ? 'selected' : '', title: "Select", "aria-label": "Select", "aria-pressed": this.state.tool === 'select', onClick: () => this.setState({ tool: 'select' }) },
                            React.createElement(Icon_1.Icon, { name: "mouse", size: 14 })),
                        React.createElement("button", { className: this.state.tool === 'pan' ? 'selected' : '', title: "Pan canvas (visual only)", "aria-label": "Pan canvas", "aria-pressed": this.state.tool === 'pan', onClick: () => this.setState({ tool: 'pan' }) },
                            React.createElement(Icon_1.Icon, { name: "hand", size: 14 }))),
                    React.createElement("span", { className: "tool-divider" }),
                    React.createElement(Controls_1.IconButton, { icon: "panel-left", small: true, label: p.leftVisible ? 'Collapse left panels' : 'Expand left panels', active: p.leftVisible && !p.focusMode, onClick: p.onLeftToggle }),
                    React.createElement(Controls_1.IconButton, { icon: "panel-bottom", small: true, label: p.bottomVisible ? 'Collapse bottom panel' : 'Open bottom panel', active: p.bottomVisible && !p.focusMode, onClick: p.onBottomToggle }),
                    React.createElement(Controls_1.IconButton, { icon: "panel-right", small: true, label: p.rightVisible ? 'Collapse right panels' : 'Expand right panels', active: p.rightVisible && !p.focusMode, onClick: p.onRightToggle }),
                    React.createElement("span", { className: "tool-divider" }),
                    React.createElement(Controls_1.IconButton, { icon: "focus", small: true, label: p.focusMode ? 'Exit focus mode' : 'Focus mode', active: p.focusMode, onClick: p.onFocusToggle }))),
            React.createElement("div", { className: "canvas-surface", "data-notation": p.family, "data-tool": this.state.tool },
                React.createElement("div", { className: "canvas-grid-overlay" }),
                React.createElement("div", { className: "canvas-content", key: p.family }, p.family === 'uml' ? React.createElement(ClassDiagram_1.ClassDiagram, null) : p.family === 'c4' ? React.createElement(C4Diagram_1.C4Diagram, null) : p.family === 'mindmap' ? React.createElement(MindMapDiagram_1.MindMapDiagram, null) : React.createElement(FlowDiagram_1.FlowDiagram, null)),
                p.minimapVisible && React.createElement("div", { className: "minimap", "aria-label": "Diagram minimap" },
                    React.createElement("div", { className: "minimap-top" },
                        React.createElement("span", null, "OVERVIEW"),
                        React.createElement("button", { className: "clean-button", onClick: p.onMinimapToggle, "aria-label": "Close minimap" },
                            React.createElement(Icon_1.Icon, { name: "close", size: 12 }))),
                    React.createElement("div", { className: "minimap-preview" },
                        React.createElement("div", { className: "mini-box a" }),
                        React.createElement("div", { className: "mini-box b" }),
                        React.createElement("div", { className: "mini-box c" }),
                        React.createElement("div", { className: "mini-box d" }),
                        React.createElement("div", { className: "minimap-viewport" })))),
            p.bottomVisible && !p.focusMode && React.createElement(React.Fragment, null,
                React.createElement(ResizeHandle_1.ResizeHandle, { axis: "vertical", label: "Altura do painel inferior", onDelta: p.onBottomResize, onReset: p.onBottomReset, value: p.bottomHeight, min: 98 }),
                React.createElement(BottomPanel_1.BottomPanel, { family: p.family, onToggle: p.onBottomToggle, tab: p.bottomTab, onTab: p.onBottomTab, height: p.bottomHeight })));
    }
}
exports.CanvasShell = CanvasShell;

};
__modules["src/components/diagrams/C4Diagram.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.C4Diagram = C4Diagram;
const React = __importStar(require("react"));
const SvgNodes_1 = require("./SvgNodes");
function Box({ x, y, w = 210, h = 107, title, subtitle, role, selected = false, external = false, db = false }) {
    return React.createElement("g", { transform: `translate(${x} ${y})`, className: `c4-node${external ? ' c4-external' : ''}` },
        React.createElement("rect", { className: "c4-box", width: w, height: h, rx: "5" }),
        React.createElement("rect", { className: "c4-box-strip", width: "4", height: h, rx: "1" }),
        db && React.createElement(React.Fragment, null,
            React.createElement("ellipse", { className: "c4-database", cx: "29", cy: "29", rx: "10", ry: "5" }),
            React.createElement("path", { className: "c4-database", d: "M19 29v9c0 7 20 7 20 0v-9" })),
        React.createElement("text", { className: "c4-role", x: db ? 50 : 17, y: "27" }, role.toUpperCase()),
        React.createElement("text", { className: "c4-title", x: "17", y: "55" }, title),
        React.createElement("text", { className: "c4-subtitle", x: "17", y: "77" }, subtitle),
        selected && React.createElement(SvgNodes_1.SelectionOutline, { x: -6, y: -6, w: w + 12, h: h + 12 }));
}
function C4Diagram() {
    return React.createElement("svg", { className: "specimen-diagram c4-specimen", viewBox: "0 0 1060 650", role: "img", "aria-label": "C4 container architecture diagram, illustrative commerce platform" },
        React.createElement(SvgNodes_1.CanvasBadge, { x: 42, y: 31, text: "C4 \u00B7 CONTAINER LEVEL" }),
        React.createElement("defs", null,
            React.createElement("marker", { id: "c4arrow", markerWidth: "8", markerHeight: "8", refX: "7", refY: "4", orient: "auto" },
                React.createElement("path", { d: "M1 1 7 4 1 7", fill: "none", stroke: "var(--diagram-edge)", strokeWidth: "1.1" }))),
        React.createElement("rect", { x: "286", y: "91", width: "580", height: "484", rx: "11", className: "c4-boundary" }),
        React.createElement("text", { x: "307", y: "119", className: "boundary-title" }, "COMMERCE PLATFORM"),
        React.createElement("text", { x: "307", y: "139", className: "boundary-subtitle" }, "System boundary"),
        React.createElement("g", { className: "c4-person" },
            React.createElement("circle", { cx: "139", cy: "219", r: "24" }),
            React.createElement("path", { d: "M99 281c0-49 80-49 80 0" }),
            React.createElement("text", { x: "139", y: "314", textAnchor: "middle", className: "c4-title" }, "Customer"),
            React.createElement("text", { x: "139", y: "334", textAnchor: "middle", className: "c4-subtitle" }, "[Person]")),
        React.createElement("g", { className: "c4-links" },
            React.createElement("path", { d: "M181 254h151", markerEnd: "url(#c4arrow)" }),
            React.createElement("path", { d: "M544 255h67", markerEnd: "url(#c4arrow)" }),
            React.createElement("path", { d: "M715 308v76", markerEnd: "url(#c4arrow)" }),
            React.createElement("path", { d: "M638 308v42H502v34", markerEnd: "url(#c4arrow)" }),
            React.createElement("path", { d: "M818 385h70", markerEnd: "url(#c4arrow)" })),
        React.createElement("g", { className: "c4-edge-label" },
            React.createElement("text", { x: "220", y: "241" }, "Uses"),
            React.createElement("text", { x: "566", y: "241" }, "HTTPS / JSON"),
            React.createElement("text", { x: "724", y: "352" }, "Reads / writes"),
            React.createElement("text", { x: "531", y: "338" }, "Publishes")),
        React.createElement(Box, { x: 332, y: 197, w: 212, title: "Web application", subtitle: "React \u00B7 client interface", role: "Container" }),
        React.createElement(Box, { x: 611, y: 197, w: 212, title: "Order API", subtitle: "Rust \u00B7 REST service", role: "Container", selected: true }),
        React.createElement(Box, { x: 398, y: 384, w: 208, title: "Event broker", subtitle: "Message queue", role: "Container" }),
        React.createElement(Box, { x: 611, y: 384, w: 207, title: "Orders database", subtitle: "PostgreSQL", role: "Data store", db: true }),
        React.createElement(Box, { x: 887, y: 385, w: 104, h: 105, title: "Payment", subtitle: "External", role: "System", external: true }),
        React.createElement("g", { transform: "translate(692 160)", className: "diagram-mini-action" },
            React.createElement("rect", { width: "134", height: "26", rx: "4" }),
            React.createElement("text", { x: "10", y: "17" }, "+ Add relation")));
}

};
__modules["src/components/diagrams/ClassDiagram.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.ClassDiagram = ClassDiagram;
const React = __importStar(require("react"));
const SvgNodes_1 = require("./SvgNodes");
function ClassDiagram() {
    return React.createElement("svg", { className: "specimen-diagram", viewBox: "0 0 1060 650", role: "img", "aria-label": "UML class diagram, illustrative example of the ordering domain" },
        React.createElement("defs", null,
            React.createElement("marker", { id: "class-open", viewBox: "0 0 10 10", refX: "9", refY: "5", markerWidth: "8", markerHeight: "8", orient: "auto-start-reverse" },
                React.createElement("path", { d: "M1 1 9 5 1 9", fill: "none", stroke: "var(--diagram-edge)", strokeWidth: "1.1" })),
            React.createElement("marker", { id: "class-triangle", viewBox: "0 0 12 12", refX: "11", refY: "6", markerWidth: "10", markerHeight: "10", orient: "auto" },
                React.createElement("path", { d: "M1 1 11 6 1 11z", fill: "var(--paper-white)", stroke: "var(--diagram-edge)", strokeWidth: "1" }))),
        React.createElement("g", { className: "relationship-layer" },
            React.createElement(SvgNodes_1.Relation, { d: "M331 191 H438", label: "creates", labelX: 365, labelY: 182 }),
            React.createElement(SvgNodes_1.Relation, { d: "M562 287 V381", label: "1..*", labelX: 574, labelY: 340 }),
            React.createElement(SvgNodes_1.Relation, { d: "M772 214 H642", label: "places", labelX: 694, labelY: 204 }),
            React.createElement(SvgNodes_1.Relation, { d: "M655 457 H768", label: "refers to", labelX: 682, labelY: 446 }),
            React.createElement("path", { d: "M220 268V372", stroke: "var(--diagram-edge)", strokeWidth: "1.6", strokeDasharray: "7 5", markerEnd: "url(#class-triangle)", fill: "none" }),
            React.createElement("path", { d: "M562 294l-9-13 9-13 9 13z", fill: "var(--paper-white)", stroke: "var(--diagram-edge)", strokeWidth: "1.6", transform: "translate(0 6)" }),
            React.createElement("text", { className: "relation-label", x: "244", y: "337" }, "implements"),
            React.createElement("text", { className: "relation-small", x: "433", y: "184" }, "1"),
            React.createElement("text", { className: "relation-small", x: "644", y: "184" }, "*")),
        React.createElement(SvgNodes_1.ClassNode, { x: 110, y: 135, w: 221, title: "OrderService", attributes: ['− repo: OrderRepository', '− validator: Validator'], operations: ['+ createOrder(): Order'], selected: true }),
        React.createElement(SvgNodes_1.ClassNode, { x: 438, y: 135, w: 204, title: "Order", attributes: ['+ id: UUID', '+ status: OrderStatus', '+ total: Decimal'], operations: ['+ addItem(item): void'] }),
        React.createElement(SvgNodes_1.ClassNode, { x: 772, y: 135, w: 184, title: "Customer", attributes: ['+ id: UUID', '+ email: String'], operations: ['+ placeOrder(): void'] }),
        React.createElement(SvgNodes_1.ClassNode, { x: 110, y: 372, w: 221, title: "OrderRepository", stereotype: "interface", attributes: ['+ findById(id): Order'], operations: ['+ save(order): void'] }),
        React.createElement(SvgNodes_1.ClassNode, { x: 460, y: 381, w: 195, title: "OrderItem", attributes: ['+ quantity: Integer', '+ price: Decimal'], operations: ['+ subtotal(): Decimal'] }),
        React.createElement(SvgNodes_1.ClassNode, { x: 768, y: 381, w: 184, title: "Product", attributes: ['+ sku: String', '+ price: Decimal'], operations: ['+ isActive(): bool'] }),
        React.createElement("g", { className: "canvas-selection-toolbar", transform: "translate(110 101)" },
            React.createElement("rect", { width: "154", height: "27", rx: "4", className: "mini-toolbar-bg" }),
            React.createElement("path", { d: "M31 6v15M62 6v15M93 6v15M124 6v15", className: "mini-toolbar-divider" }),
            React.createElement("text", { x: "14", y: "18" }, "+"),
            React.createElement("text", { x: "45", y: "18" }, "\u25C7"),
            React.createElement("text", { x: "76", y: "18" }, "\u25B9"),
            React.createElement("text", { x: "107", y: "18" }, "\u22EF"),
            React.createElement("text", { x: "137", y: "18" }, "\u2197")));
}

};
__modules["src/components/diagrams/FlowDiagram.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.FlowDiagram = FlowDiagram;
const React = __importStar(require("react"));
const SvgNodes_1 = require("./SvgNodes");
function Process({ x, y, w = 200, h = 62, title, kind = 'process', selected = false }) {
    return React.createElement("g", { transform: `translate(${x} ${y})`, className: "flow-node" },
        kind === 'decision' ? React.createElement("path", { className: "flow-node-fill", d: `M${w / 2} 0 ${w} ${h / 2} ${w / 2} ${h} 0 ${h / 2}z` }) : kind === 'terminal' ? React.createElement("rect", { className: "flow-node-fill flow-terminal", width: w, height: h, rx: h / 2 }) : kind === 'document' ? React.createElement("path", { className: "flow-node-fill", d: `M0 0h${w}v${h - 9}q-${w / 4} -15 -${w / 2} 0T0 ${h - 9}z` }) : React.createElement("rect", { className: "flow-node-fill", width: w, height: h, rx: "6" }),
        React.createElement("text", { x: w / 2, y: h / 2 + 5, textAnchor: "middle", className: "flow-node-title" }, title),
        selected && React.createElement(SvgNodes_1.SelectionOutline, { x: -7, y: -7, w: w + 14, h: h + 14 }),
        " ");
}
function FlowDiagram() {
    return React.createElement("svg", { className: "specimen-diagram flow-specimen", viewBox: "0 0 1060 650", role: "img", "aria-label": "Example flowchart showing request validation and order processing" },
        React.createElement(SvgNodes_1.CanvasBadge, { x: 42, y: 31, text: "FLOWCHART" }),
        React.createElement("defs", null,
            React.createElement("marker", { id: "flow-arrow", viewBox: "0 0 10 10", refX: "9", refY: "5", markerWidth: "7", markerHeight: "7", orient: "auto" },
                React.createElement("path", { d: "M1 1 9 5 1 9", fill: "none", stroke: "var(--diagram-edge)", strokeWidth: "1.4" }))),
        React.createElement("g", { className: "flow-arrows" },
            React.createElement("path", { d: "M540 130v54" }),
            React.createElement("path", { d: "M540 303v74" }),
            React.createElement("path", { d: "M410 244H280v96" }),
            React.createElement("path", { d: "M540 439v59" }),
            React.createElement("path", { d: "M280 402v100" }),
            React.createElement("path", { d: "M540 560v49H280v-45" })),
        React.createElement("g", { className: "flow-edge-label" },
            React.createElement("text", { x: "558", y: "346" }, "Yes"),
            React.createElement("text", { x: "323", y: "235" }, "No")),
        React.createElement(Process, { x: 442, y: 72, w: 196, h: 58, title: "Receive request", kind: "terminal" }),
        React.createElement(Process, { x: 410, y: 184, w: 260, h: 119, title: "Valid request?", kind: "decision", selected: true }),
        React.createElement(Process, { x: 170, y: 340, w: 220, h: 62, title: "Return 400", kind: "process" }),
        React.createElement(Process, { x: 438, y: 377, w: 204, h: 62, title: "Persist order" }),
        React.createElement(Process, { x: 438, y: 498, w: 204, h: 62, title: "Publish event" }),
        React.createElement(Process, { x: 170, y: 502, w: 220, h: 62, title: "End", kind: "terminal" }),
        React.createElement("g", { transform: "translate(693 214)", className: "flow-note" },
            React.createElement("rect", { width: "249", height: "94", rx: "6" }),
            React.createElement("text", { x: "17", y: "30" }, "Validation gate"),
            React.createElement("text", { x: "17", y: "54" }, "All branches are static"),
            React.createElement("text", { x: "17", y: "72" }, "visual examples."),
            React.createElement("path", { d: "M0 46H-29" })));
}

};
__modules["src/components/diagrams/MindMapDiagram.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.MindMapDiagram = MindMapDiagram;
const React = __importStar(require("react"));
const SvgNodes_1 = require("./SvgNodes");
function Topic({ x, y, w, h, title, subtitle, variant = 'neutral', selected = false }) { return React.createElement("g", { transform: `translate(${x} ${y})`, className: `mind-topic mind-${variant}` },
    React.createElement("rect", { width: w, height: h, rx: "9" }),
    React.createElement("rect", { x: "0", width: "4", height: h, rx: "2", className: "mind-accent" }),
    React.createElement("text", { className: "mind-topic-title", x: "20", y: subtitle ? 32 : h / 2 + 5 }, title),
    subtitle && React.createElement("text", { className: "mind-topic-sub", x: "20", y: "54" }, subtitle),
    selected && React.createElement(SvgNodes_1.SelectionOutline, { x: -6, y: -6, w: w + 12, h: h + 12 })); }
function MindMapDiagram() {
    return React.createElement("svg", { className: "specimen-diagram mind-specimen", viewBox: "0 0 1060 650", role: "img", "aria-label": "Mind map example, product strategy with topic branches" },
        React.createElement(SvgNodes_1.CanvasBadge, { x: 42, y: 31, text: "MIND MAP" }),
        React.createElement("g", { className: "mind-edges" },
            React.createElement("path", { d: "M390 322C334 322 349 179 276 179", className: "mind-blue" }),
            React.createElement("path", { d: "M390 322C312 322 349 455 276 455", className: "mind-amber" }),
            React.createElement("path", { d: "M632 322C706 322 677 145 739 145", className: "mind-teal" }),
            React.createElement("path", { d: "M632 322C703 322 682 324 739 324", className: "mind-violet" }),
            React.createElement("path", { d: "M632 322C707 322 678 466 739 466", className: "mind-rose" }),
            React.createElement("path", { d: "M240 179V106H185", className: "mind-blue child-edge" }),
            React.createElement("path", { d: "M240 179v70h-55", className: "mind-blue child-edge" }),
            React.createElement("path", { d: "M919 145h64v-56", className: "mind-teal child-edge" }),
            React.createElement("path", { d: "M919 324h64v58", className: "mind-violet child-edge" })),
        React.createElement(Topic, { x: 390, y: 274, w: 242, h: 96, title: "Product strategy", subtitle: "Q4 planning \u00B7 Workshop", variant: "root", selected: true }),
        React.createElement(Topic, { x: 62, y: 146, w: 214, h: 66, title: "User research", subtitle: "Interviews \u00B7 Feedback", variant: "blue" }),
        React.createElement(Topic, { x: 62, y: 422, w: 214, h: 66, title: "Delivery", subtitle: "Milestones \u00B7 Releases", variant: "amber" }),
        React.createElement(Topic, { x: 739, y: 112, w: 208, h: 66, title: "Opportunities", subtitle: "Problems to solve", variant: "teal" }),
        React.createElement(Topic, { x: 739, y: 291, w: 208, h: 66, title: "Experience", subtitle: "Interaction patterns", variant: "violet" }),
        React.createElement(Topic, { x: 739, y: 433, w: 208, h: 66, title: "Success metrics", subtitle: "Retention \u00B7 Adoption", variant: "rose" }),
        React.createElement("g", { className: "mind-caption" },
            React.createElement("text", { x: "76", y: "104" }, "Customer interviews"),
            React.createElement("text", { x: "77", y: "256" }, "Usability sessions"),
            React.createElement("text", { x: "906", y: "81" }, "Market signals"),
            React.createElement("text", { x: "894", y: "396" }, "Accessibility")));
}

};
__modules["src/components/diagrams/SvgNodes.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.SelectionOutline = SelectionOutline;
exports.ClassNode = ClassNode;
exports.Relation = Relation;
exports.CanvasBadge = CanvasBadge;
const React = __importStar(require("react"));
function SelectionOutline({ x, y, w, h }) {
    const points = [[x, y], [x + w / 2, y], [x + w, y], [x, y + h / 2], [x + w, y + h / 2], [x, y + h], [x + w / 2, y + h], [x + w, y + h]];
    return React.createElement("g", { className: "selection-outline" },
        React.createElement("rect", { x: x, y: y, width: w, height: h, fill: "none", strokeWidth: "1.7", stroke: "var(--selection)" }),
        points.map(([a, b], i) => React.createElement("rect", { key: i, x: a - 4, y: b - 4, width: "8", height: "8", fill: "var(--paper-white)", stroke: "var(--selection)", strokeWidth: "1.7", rx: "1" })));
}
function ClassNode({ x, y, w = 220, title, stereotype, attributes = [], operations = [], selected = false, muted = false }) {
    const head = stereotype ? 53 : 37, row = 23;
    const aH = Math.max(32, attributes.length * row + 11), oH = Math.max(30, operations.length * row + 10);
    const height = head + aH + oH;
    return React.createElement("g", { transform: `translate(${x} ${y})`, className: muted ? 'diagram-node muted-node' : 'diagram-node' },
        React.createElement("rect", { width: w, height: height, rx: "3", className: "node-fill" }),
        React.createElement("rect", { width: w, height: head, rx: "3", className: "class-header-fill" }),
        React.createElement("path", { d: `M0 ${head}h${w}M0 ${head + aH}h${w}`, className: "node-divider" }),
        stereotype && React.createElement("text", { x: w / 2, y: "20", className: "node-stereotype", textAnchor: "middle" },
            "\u00AB",
            stereotype,
            "\u00BB"),
        React.createElement("text", { x: w / 2, y: stereotype ? 39 : 24, className: "node-title", textAnchor: "middle" }, title),
        attributes.map((a, i) => React.createElement("text", { key: a, x: "14", y: head + 22 + row * i, className: "node-detail" }, a)),
        operations.map((o, i) => React.createElement("text", { key: o, x: "14", y: head + aH + 22 + row * i, className: "node-detail" }, o)),
        selected && React.createElement(SelectionOutline, { x: -6, y: -6, w: w + 12, h: height + 12 }));
}
function Relation({ d, label, labelX, labelY, dashed = false, arrow = true }) {
    return React.createElement("g", { className: "diagram-relation" },
        React.createElement("path", { d: d, className: "relation-path", strokeDasharray: dashed ? '6 5' : undefined }),
        arrow && null,
        label && React.createElement("g", null,
            React.createElement("rect", { x: (labelX || 0) - 8, y: (labelY || 0) - 18, width: label.length * 6.3 + 16, height: "22", className: "relation-label-bg", rx: "3" }),
            React.createElement("text", { x: labelX, y: labelY, className: "relation-label" }, label)));
}
function CanvasBadge({ x, y, text }) {
    return React.createElement("g", { transform: `translate(${x} ${y})` },
        React.createElement("rect", { width: text.length * 8 + 20, height: "26", rx: "5", className: "canvas-badge-bg" }),
        React.createElement("text", { x: "10", y: "17", className: "canvas-badge-label" }, text));
}

};
__modules["src/components/dock/DockPanel.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.DockPanelHeader = DockPanelHeader;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
function DockPanelHeader({ title, open, onToggle, action, id }) {
    return React.createElement("header", { className: "dock-panel-header" },
        React.createElement("button", { type: "button", "aria-expanded": open, "aria-controls": id, title: `${open ? 'Collapse' : 'Expand'} ${title}`, className: "dock-panel-toggle", onClick: onToggle },
            React.createElement(Icon_1.Icon, { name: open ? 'chevron-down' : 'chevron-right', size: 13 }),
            React.createElement("span", null, title)),
        open && action && React.createElement("div", { className: "dock-panel-actions" }, action));
}

};
__modules["src/components/dock/ResizeHandle.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.ResizeHandle = void 0;
const React = __importStar(require("react"));
/** Visual-layout splitter only: no diagram editing behavior. */
class ResizeHandle extends React.Component {
    constructor() {
        super(...arguments);
        this.handle = null;
        this.previous = 0;
        this.dragging = false;
        this.pointerDown = (event) => {
            if (event.button !== 0)
                return;
            event.preventDefault();
            this.dragging = true;
            this.previous = this.coordinate(event);
            document.body.classList.add('is-resizing');
            window.addEventListener('pointermove', this.pointerMove);
            window.addEventListener('pointerup', this.pointerUp);
            window.addEventListener('pointercancel', this.pointerUp);
        };
        this.pointerMove = (event) => {
            if (!this.dragging)
                return;
            const coordinate = this.coordinate(event);
            const delta = coordinate - this.previous;
            this.previous = coordinate;
            if (delta !== 0)
                this.props.onDelta(delta);
        };
        this.pointerUp = () => this.endDrag();
        this.keyDown = (event) => {
            const horizontal = this.props.axis === 'horizontal';
            if (event.key === 'Home') {
                event.preventDefault();
                this.props.onReset();
                return;
            }
            const key = horizontal ? ['ArrowLeft', 'ArrowRight'] : ['ArrowUp', 'ArrowDown'];
            if (event.key !== key[0] && event.key !== key[1])
                return;
            event.preventDefault();
            this.props.onDelta(event.key === key[1] ? 16 : -16);
        };
    }
    componentDidMount() {
        var _a;
        (_a = this.handle) === null || _a === void 0 ? void 0 : _a.addEventListener('pointerdown', this.pointerDown);
    }
    componentWillUnmount() {
        var _a;
        (_a = this.handle) === null || _a === void 0 ? void 0 : _a.removeEventListener('pointerdown', this.pointerDown);
        this.endDrag();
    }
    coordinate(event) {
        return this.props.axis === 'horizontal' ? event.clientX : event.clientY;
    }
    endDrag() {
        this.dragging = false;
        if (typeof document !== 'undefined')
            document.body.classList.remove('is-resizing');
        window.removeEventListener('pointermove', this.pointerMove);
        window.removeEventListener('pointerup', this.pointerUp);
        window.removeEventListener('pointercancel', this.pointerUp);
    }
    render() {
        return React.createElement("div", { ref: node => { this.handle = node; }, className: `resize-handle resize-${this.props.axis} ${this.props.className || ''}`, role: "separator", "aria-orientation": this.props.axis === 'horizontal' ? 'vertical' : 'horizontal', "aria-label": `${this.props.label}. Drag to resize; Home resets.`, "aria-valuenow": this.props.value, "aria-valuemin": this.props.min, "aria-valuemax": this.props.max, tabIndex: 0, title: `${this.props.label} · drag to resize · double-click to reset`, onKeyDown: this.keyDown, onDoubleClick: this.props.onReset });
    }
}
exports.ResizeHandle = ResizeHandle;

};
__modules["src/components/gallery/ComponentGallery.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.ComponentGallery = ComponentGallery;
const React = __importStar(require("react"));
const diagramData_1 = require("../../app/diagramData");
const catalog_1 = require("../palette/catalog");
const ShapeGlyph_1 = require("../palette/ShapeGlyph");
const Icon_1 = require("../primitives/Icon");
function ComponentGallery({ family, onFamily, chosen, onChoose }) {
    var _a, _b;
    return React.createElement("main", { className: "gallery-view" },
        React.createElement("div", { className: "gallery-header" },
            React.createElement("div", { className: "gallery-eyebrow" }, "DESIGN LIBRARY / SHAPES"),
            React.createElement("h1", null, "Symbol library"),
            React.createElement("p", null, "A reference for diagram primitives. Names and geometry are organized for a direct GPUI component mapping.")),
        React.createElement("div", { className: "gallery-family-tabs" }, diagramData_1.families.map(f => React.createElement("button", { key: f.id, className: family === f.id ? 'active' : '', onClick: () => onFamily(f.id) },
            React.createElement("span", null, f.name),
            React.createElement("small", null, catalog_1.familyTotals[f.id])))),
        React.createElement("div", { className: "gallery-scroll" },
            React.createElement("div", { className: "library-context" },
                React.createElement("div", null,
                    React.createElement("span", { className: "library-title" }, (_a = diagramData_1.families.find(f => f.id === family)) === null || _a === void 0 ? void 0 : _a.name),
                    React.createElement("span", { className: "library-description" }, (_b = diagramData_1.families.find(f => f.id === family)) === null || _b === void 0 ? void 0 : _b.description)),
                React.createElement("div", { className: "gallery-meta" },
                    React.createElement(Icon_1.Icon, { name: "box", size: 14 }),
                    catalog_1.catalog[family].length,
                    " groups \u00B7 ",
                    catalog_1.familyTotals[family],
                    " symbols")),
            React.createElement("details", { className: "diagram-types-details" },
                React.createElement("summary", null,
                    React.createElement(Icon_1.Icon, { name: "layers-two", size: 15 }),
                    React.createElement("span", null, "Diagram types in scope"),
                    React.createElement("span", { className: "type-count" },
                        diagramData_1.diagramTypes[family].length,
                        " types"),
                    React.createElement(Icon_1.Icon, { name: "chevron-down", size: 13 })),
                React.createElement("div", { className: "diagram-type-grid" }, diagramData_1.diagramTypes[family].map(x => React.createElement("span", { key: x }, x)))),
            catalog_1.catalog[family].map(g => React.createElement("section", { className: "gallery-section", key: g.id },
                React.createElement("div", { className: "gallery-section-head" },
                    React.createElement("h2", null, g.title),
                    React.createElement("span", null,
                        g.items.length,
                        " items")),
                React.createElement("div", { className: "symbol-grid" }, g.items.map(i => React.createElement("button", { className: `symbol-card ${i.id === chosen ? 'selected' : ''}`, key: i.id, onClick: () => onChoose(i.id) },
                    React.createElement("span", { className: "symbol-preview" },
                        React.createElement(ShapeGlyph_1.ShapeGlyph, { kind: i.kind, size: 55 })),
                    React.createElement("span", { className: "symbol-name" }, i.label),
                    React.createElement("span", { className: "symbol-identifier" }, i.kind)))))),
            React.createElement("p", { className: "gallery-disclaimer" }, "Catalog entries are visual references. No metamodel, validation rules, serialization, or diagram operations are implemented.")));
}

};
__modules["src/components/gallery/DesignSystemView.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.DesignSystemView = DesignSystemView;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
const ShapeGlyph_1 = require("../palette/ShapeGlyph");
const surfaceTokens = ['--app-bg', '--surface-1', '--surface-2', '--surface-3', '--border', '--muted', '--text', '--accent', '--canvas', '--selection'];
const components = [
    { name: 'Dock / Split view', icon: 'layout', gpui: 'Dock · Resizable' },
    { name: 'Diagram tab', icon: 'layers-two', gpui: 'Tabs' },
    { name: 'Tool palette', icon: 'shapes', gpui: 'Sidebar · Scrollable' },
    { name: 'Model explorer', icon: 'tree', gpui: 'Tree' },
    { name: 'Property field', icon: 'sliders', gpui: 'Input · Select' },
    { name: 'Command menu', icon: 'command', gpui: 'Command · Menu' },
    { name: 'Canvas toolbar', icon: 'dots', gpui: 'Toolbar · Button' },
    { name: 'Status bar', icon: 'panel-bottom', gpui: 'StatusBar' }
];
function DesignSystemView({ theme }) {
    return React.createElement("main", { className: "design-system-view" },
        React.createElement("div", { className: "gallery-header" },
            React.createElement("div", { className: "gallery-eyebrow" }, "DESIGN LIBRARY / FOUNDATIONS"),
            React.createElement("h1", null, "Interface foundations"),
            React.createElement("p", null, "Semantic tokens, density, component states and naming conventions for the GPUI Kit handoff.")),
        React.createElement("div", { className: "system-scroll" },
            React.createElement("div", { className: "system-intro" },
                React.createElement("div", { className: "eyebrow" }, "CURRENT SCHEME"),
                React.createElement("div", { className: "current-theme-line" },
                    React.createElement("span", { className: "scheme-visual" }),
                    React.createElement("strong", null, theme[0].toUpperCase() + theme.slice(1)),
                    React.createElement("span", null, "12 px UI typography \u00B7 4 px radius \u00B7 8 px base spacing"))),
            React.createElement("section", { className: "system-section" },
                React.createElement("div", { className: "system-section-heading" },
                    React.createElement("h2", null, "Color roles"),
                    React.createElement("span", null, "Semantic CSS tokens")),
                React.createElement("div", { className: "token-grid" }, surfaceTokens.map(t => React.createElement("div", { className: "token-card", key: t },
                    React.createElement("span", { className: "token-swatch", style: { background: `var(${t})` } }),
                    React.createElement("code", null, t.replace('--', '')))))),
            React.createElement("section", { className: "system-section" },
                React.createElement("div", { className: "system-section-heading" },
                    React.createElement("h2", null, "Primitive states"),
                    React.createElement("span", null, "Visual treatment")),
                React.createElement("div", { className: "primitives-example" },
                    React.createElement("button", { className: "example-primary" }, "Primary action"),
                    React.createElement("button", { className: "example-secondary" }, "Secondary"),
                    React.createElement("button", { className: "example-quiet" },
                        React.createElement(Icon_1.Icon, { name: "plus", size: 15 }),
                        " Quiet action"),
                    React.createElement("button", { className: "example-secondary", disabled: true }, "Disabled"),
                    React.createElement("span", { className: "demo-chip" }, "Selected"),
                    React.createElement("span", { className: "demo-outline" }, "Default")),
                React.createElement("div", { className: "field-examples" },
                    React.createElement("div", null,
                        React.createElement("small", null, "INPUT"),
                        React.createElement("span", null, "OrderService")),
                    React.createElement("div", null,
                        React.createElement("small", null, "SELECT"),
                        React.createElement("span", null,
                            "Class ",
                            React.createElement(Icon_1.Icon, { name: "chevron-down", size: 13 }))),
                    React.createElement("div", null,
                        React.createElement("small", null, "TOGGLE"),
                        React.createElement("span", null,
                            "Show labels ",
                            React.createElement("span", { className: "mini-toggle" }))))),
            React.createElement("section", { className: "system-section" },
                React.createElement("div", { className: "system-section-heading" },
                    React.createElement("h2", null, "Diagram primitives"),
                    React.createElement("span", null, "SVG reference specimens")),
                React.createElement("div", { className: "system-shapes" }, ['class', 'interface', 'association', 'person', 'system', 'database', 'topic', 'decision'].map(k => React.createElement("div", { key: k },
                    React.createElement(ShapeGlyph_1.ShapeGlyph, { kind: k, size: 49 }),
                    React.createElement("span", null, k))))),
            React.createElement("section", { className: "system-section" },
                React.createElement("div", { className: "system-section-heading" },
                    React.createElement("h2", null, "Component anatomy"),
                    React.createElement("span", null, "Concept \u2192 GPUI Kit")),
                React.createElement("div", { className: "mapping-grid" }, components.map(c => React.createElement("div", { className: "mapping-row", key: c.name },
                    React.createElement(Icon_1.Icon, { name: c.icon, size: 17 }),
                    React.createElement("div", null,
                        React.createElement("strong", null, c.name),
                        React.createElement("span", null, c.gpui)),
                    React.createElement(Icon_1.Icon, { name: "chevron-right", size: 13 }))))),
            React.createElement("div", { className: "system-footer-note" },
                React.createElement(Icon_1.Icon, { name: "info", size: 14 }),
                " HTML/CSS is the design source here; production GPUI implementation is intentionally excluded.")));
}

};
__modules["src/components/layout/ActivityRail.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.ActivityRail = ActivityRail;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
const views = [
    { mode: 'editor', icon: 'layers', title: 'Workspace' },
    { mode: 'library', icon: 'shapes', title: 'Biblioteca de símbolos' },
    { mode: 'system', icon: 'palette', title: 'Design system' }
];
function ActivityRail({ view, onView }) {
    return React.createElement("nav", { className: "activity-rail", "aria-label": "Navega\u00E7\u00E3o principal" },
        React.createElement("div", { className: "rail-main" }, views.map(({ mode, icon, title }) => React.createElement("button", { key: mode, className: `rail-action ${view === mode ? 'is-active' : ''}`, title: title, "aria-label": title, "aria-current": view === mode ? 'page' : undefined, onClick: () => onView(mode) },
            React.createElement(Icon_1.Icon, { name: icon, size: 19 })))),
        React.createElement("div", { className: "rail-bottom" },
            React.createElement("span", { className: "rail-sample-mark", title: "Estudo de interface" }, "UI")));
}

};
__modules["src/components/layout/BottomPanel.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.BottomPanel = BottomPanel;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
const Controls_1 = require("../primitives/Controls");
const relations = {
    uml: [['OrderService', 'creates', 'Order'], ['Customer', 'places', 'Order'], ['Order', 'contains', 'OrderItem'], ['OrderItem', 'references', 'Product']],
    c4: [['Web application', 'calls', 'Backend API'], ['Backend API', 'reads', 'Database'], ['Backend API', 'requests', 'Payment gateway']],
    mindmap: [['Product strategy', 'contains', 'Research'], ['Product strategy', 'contains', 'Roadmap']],
    flowchart: [['Receive request', 'next', 'Validate order'], ['Validate order', 'next', 'Persist order']]
};
const descriptions = {
    uml: 'Entities, interfaces and relationships in the ordering domain.',
    c4: 'Containers, external systems and their dependencies.',
    mindmap: 'Product discovery and planning structure.',
    flowchart: 'Order processing steps and decisions.'
};
function BottomPanel({ family, onToggle, tab, onTab, height }) {
    const tabs = [
        { id: 'relationships', title: 'Relationships' }, { id: 'documentation', title: 'Markdown documentation' },
        { id: 'thumbnails', title: 'Diagram thumbnails' }
    ];
    return React.createElement("section", { className: "bottom-panel", style: { height, flex: `0 0 ${height}px` } },
        React.createElement("div", { className: "bottom-tabs", role: "tablist", "aria-label": "Pain\u00E9is auxiliares" },
            tabs.map(t => React.createElement("button", { role: "tab", "aria-selected": tab === t.id, key: t.id, className: tab === t.id ? 'active' : '', onClick: () => onTab(t.id) }, t.title)),
            React.createElement("div", { className: "bottom-flex" }),
            React.createElement(Controls_1.IconButton, { icon: "chevron-down", label: "Collapse bottom panel", small: true, onClick: onToggle })),
        React.createElement("div", { className: "bottom-panel-body" }, tab === 'relationships' ? React.createElement("div", { className: "relationship-table" },
            React.createElement("div", { className: "relationship-th" },
                React.createElement("span", null, "Source"),
                React.createElement("span", null, "Relation"),
                React.createElement("span", null, "Target")),
            relations[family].map(([from, action, to]) => React.createElement("div", { className: "relationship-tr", key: from + to },
                React.createElement("span", null,
                    React.createElement(Icon_1.Icon, { name: "box", size: 12 }),
                    from),
                React.createElement("span", null, action),
                React.createElement("span", null, to)))) : tab === 'documentation' ? React.createElement("div", { className: "markdown-panel-preview" },
            React.createElement("div", { className: "markdown-preview-actions" },
                React.createElement("span", null, "README.md"),
                React.createElement("span", null, "Preview")),
            React.createElement("h3", null, family === 'uml' ? 'Domain model' : family === 'c4' ? 'System architecture' : family === 'mindmap' ? 'Product discovery' : 'Order processing'),
            React.createElement("p", null, descriptions[family])) : React.createElement("div", { className: "thumbnail-row" },
            React.createElement("button", { className: "diagram-thumbnail active", "aria-label": "Miniatura do diagrama atual" },
                React.createElement("span", { className: "thumbnail-graphic" },
                    React.createElement("i", null),
                    React.createElement("i", null),
                    React.createElement("i", null)),
                React.createElement("span", null, family === 'uml' ? 'Domain model' : family === 'c4' ? 'System architecture' : family === 'mindmap' ? 'Product discovery' : 'Order processing')),
            React.createElement("span", { className: "thumbnail-hint" }, "Diagram structure preview"))));
}

};
__modules["src/components/layout/LeftSidebar.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.LeftSidebar = LeftSidebar;
const React = __importStar(require("react"));
const ResizeHandle_1 = require("../dock/ResizeHandle");
const WorkingDiagramsPane_1 = require("../panels/WorkingDiagramsPane");
const ToolboxPane_1 = require("../panels/ToolboxPane");
/** A stack is only composition; its panes can be placed in any future native Dock. */
function LeftSidebar(p) {
    return React.createElement("aside", { className: "left-sidebar", "aria-label": "Diagrams and tools" },
        React.createElement(WorkingDiagramsPane_1.WorkingDiagramsPane, { family: p.family, onFamily: p.onFamily, open: p.projectsOpen, onToggle: p.onProjectsToggle, height: p.projectsHeight }),
        p.projectsOpen && p.toolboxOpen && React.createElement(ResizeHandle_1.ResizeHandle, { axis: "vertical", label: "Working diagrams height", onDelta: p.onProjectsResize, onReset: p.onProjectsReset, value: p.projectsHeight, min: 90 }),
        React.createElement(ToolboxPane_1.ToolboxPane, { family: p.family, search: p.search, onSearch: p.onSearch, chosen: p.chosen, onChoose: p.onChoose, collapsed: p.collapsed, onToggleGroup: p.onToggleGroup, open: p.toolboxOpen, onToggle: p.onToolboxToggle }));
}

};
__modules["src/components/layout/RightInspector.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.RightInspector = RightInspector;
const React = __importStar(require("react"));
const ResizeHandle_1 = require("../dock/ResizeHandle");
const ModelExplorerPane_1 = require("../panels/ModelExplorerPane");
const PropertyInspectorPane_1 = require("../panels/PropertyInspectorPane");
/** Stack positioning can be replaced by GPUI Dock without rewriting pane content. */
function RightInspector(p) {
    return React.createElement("aside", { className: "right-inspector", "aria-label": "Model and properties" },
        React.createElement(ModelExplorerPane_1.ModelExplorerPane, { family: p.family, open: p.explorerOpen, onToggle: p.onExplorerToggle, height: p.explorerHeight }),
        p.explorerOpen && p.inspectorOpen && React.createElement(ResizeHandle_1.ResizeHandle, { axis: "vertical", label: "Model explorer height", onDelta: p.onExplorerResize, onReset: p.onExplorerReset, value: p.explorerHeight, min: 80 }),
        React.createElement(PropertyInspectorPane_1.PropertyInspectorPane, { family: p.family, tab: p.tab, onTab: p.onTab, open: p.inspectorOpen, onToggle: p.onInspectorToggle }));
}

};
__modules["src/components/layout/Statusbar.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.Statusbar = Statusbar;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
function Statusbar({ family }) {
    const label = { uml: 'UML · Class diagram', c4: 'C4 · Container diagram', mindmap: 'Mind map', flowchart: 'Flowchart' };
    return React.createElement("footer", { className: "statusbar" },
        React.createElement("div", { className: "statusbar-left" },
            React.createElement(Icon_1.Icon, { name: "layers", size: 12 }),
            React.createElement("span", null, "Model"),
            React.createElement("span", { className: "status-sep" }),
            React.createElement("span", null, label[family])),
        React.createElement("div", { className: "statusbar-right" },
            React.createElement("span", null, "UI study"),
            React.createElement("span", { className: "status-sep" }),
            React.createElement(Icon_1.Icon, { name: "grid", size: 12 }),
            React.createElement("span", null, "20 px"),
            React.createElement("span", { className: "status-sep" }),
            React.createElement("span", null, "100%")));
}

};
__modules["src/components/layout/Topbar.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.Topbar = void 0;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
class Topbar extends React.Component {
    constructor() {
        super(...arguments);
        this.state = { open: false };
        this.menuRef = null;
        this.closeOutside = (e) => {
            if (this.menuRef && !this.menuRef.contains(e.target))
                this.setState({ open: false });
        };
        this.run = (f) => { f(); this.setState({ open: false }); };
    }
    componentDidMount() { document.addEventListener('pointerdown', this.closeOutside); }
    componentWillUnmount() { document.removeEventListener('pointerdown', this.closeOutside); }
    render() {
        const p = this.props;
        return React.createElement("header", { className: "app-titlebar" },
            React.createElement("div", { className: "brand", "aria-label": "StarUML" },
                React.createElement("span", { className: "brand-mark" },
                    React.createElement(Icon_1.Icon, { name: "layers", size: 18 })),
                React.createElement("span", null, "StarUML")),
            React.createElement("span", { className: "title-divider" }),
            React.createElement("nav", { className: "menu-entries", "aria-label": "Application menu" }, ['File', 'Edit', 'Format', 'Model', 'Tools', 'View'].map(item => React.createElement("span", { className: "menu-entry", key: item }, item))),
            React.createElement("div", { className: "titlebar-fill" }),
            React.createElement("div", { className: "project-crumb" },
                React.createElement(Icon_1.Icon, { name: "folder", size: 14 }),
                React.createElement("span", null, "Commerce platform"),
                React.createElement("span", { className: "crumb-slash" }, "/"),
                React.createElement("strong", null, p.diagram)),
            React.createElement("div", { className: "titlebar-fill" }),
            React.createElement("div", { className: "toolbar-menu-anchor", ref: node => { this.menuRef = node; } },
                React.createElement("button", { className: `titlebar-control ${this.state.open ? 'is-open' : ''}`, "aria-haspopup": "menu", "aria-expanded": this.state.open, onClick: () => this.setState(s => ({ open: !s.open })), title: "Organize workspace" },
                    React.createElement(Icon_1.Icon, { name: "layout", size: 15 }),
                    React.createElement("span", null, "Layout"),
                    React.createElement(Icon_1.Icon, { name: "chevron-down", size: 11 })),
                this.state.open && React.createElement("div", { className: "workspace-menu", role: "menu", "aria-label": "Organiza\u00E7\u00E3o do workspace" },
                    React.createElement("div", { className: "menu-caption" }, "WORKSPACE PRESETS"),
                    React.createElement("button", { role: "menuitem", onClick: () => this.run(() => p.onPreset('modeling')) },
                        React.createElement(Icon_1.Icon, { name: "panel-left", size: 15 }),
                        React.createElement("span", null, "Modeling"),
                        React.createElement("small", null, "Side panels")),
                    React.createElement("button", { role: "menuitem", onClick: () => this.run(() => p.onPreset('review')) },
                        React.createElement(Icon_1.Icon, { name: "panel-bottom", size: 15 }),
                        React.createElement("span", null, "Review"),
                        React.createElement("small", null, "Bottom panel")),
                    React.createElement("button", { role: "menuitem", onClick: () => this.run(() => p.onPreset('focus')) },
                        React.createElement(Icon_1.Icon, { name: "focus", size: 15 }),
                        React.createElement("span", null, "Canvas only")),
                    React.createElement("div", { className: "menu-separator" }),
                    React.createElement("button", { role: "menuitemcheckbox", "aria-checked": p.flipped, onClick: () => this.run(p.onFlip) },
                        React.createElement(Icon_1.Icon, { name: "arrow-right", size: 15 }),
                        React.createElement("span", null, "Swap side panels"),
                        p.flipped && React.createElement(Icon_1.Icon, { name: "check", size: 13 })),
                    React.createElement("button", { role: "menuitemcheckbox", "aria-checked": p.minimap, onClick: () => this.run(p.onMinimap) },
                        React.createElement(Icon_1.Icon, { name: "eye", size: 15 }),
                        React.createElement("span", null, "Minimapa"),
                        p.minimap && React.createElement(Icon_1.Icon, { name: "check", size: 13 })),
                    React.createElement("button", { role: "menuitem", onClick: () => this.run(p.onReset) },
                        React.createElement(Icon_1.Icon, { name: "undo", size: 15 }),
                        React.createElement("span", null, "Reset layout")))),
            React.createElement("button", { className: "titlebar-control icon-only", title: `Switch to ${p.theme === 'dark' ? 'light' : 'dark'}`, "aria-label": `Theme ${p.theme === 'dark' ? 'dark' : 'light'}. Toggle`, onClick: () => p.onTheme(p.theme === 'dark' ? 'light' : 'dark') },
                React.createElement(Icon_1.Icon, { name: p.theme === 'dark' ? 'sun' : 'moon', size: 16 })));
    }
}
exports.Topbar = Topbar;

};
__modules["src/components/palette/ShapeGlyph.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.ShapeGlyph = ShapeGlyph;
const React = __importStar(require("react"));
function ShapeGlyph({ kind, size = 30 }) {
    const base = { fill: 'none', stroke: 'currentColor', strokeWidth: 1.5, strokeLinejoin: 'round', strokeLinecap: 'round' };
    const line = React.createElement(React.Fragment, null,
        React.createElement("path", { d: "M4 15h32" }),
        React.createElement("path", { d: "m30 11 6 4-6 4" }));
    let shape;
    switch (kind) {
        case 'class':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "6", y: "4", width: "28", height: "22", rx: "1" }),
                React.createElement("path", { d: "M6 11h28M6 19h28" }),
                React.createElement("path", { d: "M10 7h12M10 15h8M10 22h12" }));
            break;
        case 'interface':
            shape = React.createElement(React.Fragment, null,
                React.createElement("circle", { cx: "20", cy: "9", r: "6" }),
                React.createElement("path", { d: "M20 15v12M11 27h18" }));
            break;
        case 'enum':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "6", y: "4", width: "28", height: "22" }),
                React.createElement("path", { d: "M6 15h28M13 9h14M11 20h15" }));
            break;
        case 'association':
        case 'relation':
        case 'message':
            shape = line;
            break;
        case 'aggregate':
            shape = React.createElement(React.Fragment, null,
                React.createElement("path", { d: "m4 15 7-5 7 5-7 5zM18 15h18" }));
            break;
        case 'composition':
            shape = React.createElement(React.Fragment, null,
                React.createElement("path", { d: "m4 15 7-5 7 5-7 5z", fill: "currentColor" }),
                React.createElement("path", { d: "M18 15h18" }));
            break;
        case 'inheritance':
            shape = React.createElement(React.Fragment, null,
                React.createElement("path", { d: "m5 15 10-6v12zM15 15h21" }));
            break;
        case 'dependency':
            shape = React.createElement(React.Fragment, null,
                React.createElement("path", { d: "M4 15h32", strokeDasharray: "4 3" }),
                React.createElement("path", { d: "m30 11 6 4-6 4" }));
            break;
        case 'actor':
        case 'person':
            shape = React.createElement(React.Fragment, null,
                React.createElement("circle", { cx: "20", cy: "6", r: "4" }),
                React.createElement("path", { d: "M20 10v11m-8-6h16m-8 6-6 7m6-7 6 7" }));
            break;
        case 'usecase':
        case 'terminator':
            shape = React.createElement("ellipse", { cx: "20", cy: "15", rx: "16", ry: "8" });
            break;
        case 'lifeline':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "9", y: "3", width: "22", height: "9" }),
                React.createElement("path", { d: "M20 12v17", strokeDasharray: "4 3" }));
            break;
        case 'package':
            shape = React.createElement("path", { d: "M5 8h13l3 4h14v15H5zM5 8V4h13v4" });
            break;
        case 'note':
        case 'callout':
            shape = React.createElement("path", { d: "M6 4h21l7 7v17H6zM27 4v7h7M10 16h16M10 21h11" });
            break;
        case 'state':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "5", y: "8", width: "30", height: "16", rx: "7" }),
                React.createElement("path", { d: "M11 16h18" }));
            break;
        case 'fork':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "5", y: "13", width: "30", height: "5", fill: "currentColor" }),
                React.createElement("path", { d: "M20 3v10M12 18v10M28 18v10" }));
            break;
        case 'activity':
        case 'process':
            shape = React.createElement("rect", { x: "5", y: "6", width: "30", height: "19", rx: "3" });
            break;
        case 'decision':
            shape = React.createElement("path", { d: "M20 2 37 15 20 28 3 15z" });
            break;
        case 'component':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "8", y: "6", width: "27", height: "20" }),
                React.createElement("rect", { x: "5", y: "10", width: "8", height: "5", fill: "var(--surface-2)" }),
                React.createElement("rect", { x: "5", y: "18", width: "8", height: "5", fill: "var(--surface-2)" }));
            break;
        case 'deployment':
            shape = React.createElement(React.Fragment, null,
                React.createElement("path", { d: "M8 9 20 2l13 7v14l-13 7-12-7zM8 9l12 8 13-8M20 17v13" }));
            break;
        case 'boundary':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "4", y: "4", width: "32", height: "23", rx: "2", strokeDasharray: "4 3" }),
                React.createElement("path", { d: "M9 10h12" }));
            break;
        case 'system':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "4", y: "5", width: "32", height: "22", rx: "3", fill: "currentColor", opacity: ".18" }),
                React.createElement("rect", { x: "4", y: "5", width: "32", height: "22", rx: "3" }),
                React.createElement("path", { d: "M10 13h20M10 18h13" }));
            break;
        case 'container':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "5", y: "4", width: "30", height: "23", rx: "2" }),
                React.createElement("path", { d: "M5 10h30M10 17h20M10 21h13" }));
            break;
        case 'database':
            shape = React.createElement(React.Fragment, null,
                React.createElement("ellipse", { cx: "20", cy: "8", rx: "13", ry: "5" }),
                React.createElement("path", { d: "M7 8v14c0 7 26 7 26 0V8M7 15c0 7 26 7 26 0" }));
            break;
        case 'external':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "4", y: "5", width: "32", height: "22", rx: "3", strokeDasharray: "3 3" }),
                React.createElement("path", { d: "m15 12 5 5 6-7" }));
            break;
        case 'topic':
            shape = React.createElement("rect", { x: "3", y: "8", width: "34", height: "14", rx: "7", fill: "currentColor", fillOpacity: ".2" });
            break;
        case 'branch':
            shape = React.createElement(React.Fragment, null,
                React.createElement("path", { d: "M2 15h12l7-7h17" }),
                React.createElement("circle", { cx: "21", cy: "8", r: "2" }),
                React.createElement("path", { d: "M25 15h12" }));
            break;
        case 'document':
            shape = React.createElement("path", { d: "M6 4h28v22c-10-8-18 8-28 0z" });
            break;
        case 'input':
            shape = React.createElement("path", { d: "M11 5h26l-8 21H3z" });
            break;
        case 'connector':
            shape = React.createElement(React.Fragment, null,
                React.createElement("circle", { cx: "20", cy: "15", r: "10" }),
                React.createElement("path", { d: "m16 15 3 3 6-6" }));
            break;
        case 'predefined':
            shape = React.createElement(React.Fragment, null,
                React.createElement("rect", { x: "4", y: "6", width: "32", height: "19" }),
                React.createElement("path", { d: "M9 6v19M31 6v19" }));
            break;
        default: shape = line;
    }
    return React.createElement("svg", { viewBox: "0 0 40 30", width: size, height: size * .75, ...base, "aria-hidden": "true" }, shape);
}

};
__modules["src/components/palette/catalog.ts"] = function(require,module,exports){
"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.familyTotals = exports.catalog = void 0;
// Presentation catalog. A label is vocabulary, not a schema or serialized UML metamodel.
const group = (id, title, rows) => ({
    id, title, items: rows.map(([label, kind], index) => ({ id: `${id}-${index}`, label, kind }))
});
const u = {
    classes: group('uml-classes', 'Classes & structure', [
        ['Class', 'class'], ['Interface', 'interface'], ['Enumeration', 'enum'], ['Primitive type', 'class'],
        ['Data type', 'class'], ['Signal', 'class'], ['Association class', 'class'], ['Template', 'class'],
        ['Package', 'package'], ['Model', 'package'], ['Subsystem', 'package'], ['Comment', 'note']
    ]),
    relationships: group('uml-relationships', 'Relationships', [
        ['Association', 'association'], ['Directed association', 'association'], ['Aggregation', 'aggregate'],
        ['Composition', 'composition'], ['Dependency', 'dependency'], ['Generalization', 'inheritance'],
        ['Interface realization', 'inheritance'], ['Abstraction', 'dependency'], ['Usage', 'dependency'],
        ['Containment', 'association'], ['Information flow', 'association']
    ]),
    usecase: group('uml-usecase', 'Use cases', [
        ['Actor', 'actor'], ['Use case', 'usecase'], ['Subject boundary', 'boundary'], ['Include', 'dependency'],
        ['Extend', 'dependency'], ['Extension point', 'note'], ['Generalization', 'inheritance']
    ]),
    sequence: group('uml-sequence', 'Sequence & interaction', [
        ['Lifeline', 'lifeline'], ['Actor lifeline', 'actor'], ['Message', 'message'], ['Async message', 'message'],
        ['Reply message', 'message'], ['Create message', 'message'], ['Delete message', 'message'],
        ['Found message', 'message'], ['Lost message', 'message'], ['Self message', 'message'],
        ['Execution', 'lifeline'], ['Combined fragment', 'boundary'], ['Interaction use', 'boundary'],
        ['State invariant', 'state'], ['Gate', 'connector'], ['Duration constraint', 'note']
    ]),
    activity: group('uml-activity', 'Activities', [
        ['Initial node', 'state'], ['Action', 'activity'], ['Activity', 'activity'], ['Decision', 'decision'],
        ['Merge', 'decision'], ['Fork', 'fork'], ['Join', 'fork'], ['Activity final', 'state'],
        ['Flow final', 'state'], ['Object node', 'activity'], ['Input pin', 'connector'],
        ['Output pin', 'connector'], ['Control flow', 'relation'], ['Object flow', 'relation'],
        ['Partition', 'boundary'], ['Expansion region', 'boundary'], ['Accept event', 'activity']
    ]),
    state: group('uml-state', 'State machines', [
        ['State', 'state'], ['Initial pseudostate', 'state'], ['Final state', 'state'],
        ['Choice', 'decision'], ['Junction', 'decision'], ['Fork / join', 'fork'], ['History', 'state'],
        ['Entry point', 'connector'], ['Exit point', 'connector'], ['Transition', 'relation'],
        ['Region', 'boundary'], ['Terminate', 'state']
    ]),
    components: group('uml-components', 'Components & deployment', [
        ['Component', 'component'], ['Artifact', 'document'], ['Port', 'connector'], ['Part', 'class'],
        ['Provided interface', 'interface'], ['Required interface', 'interface'], ['Connector', 'relation'],
        ['Node', 'deployment'], ['Device', 'deployment'], ['Execution environment', 'deployment'],
        ['Deployment specification', 'document'], ['Communication path', 'association'],
        ['Deployment', 'dependency'], ['Manifestation', 'dependency']
    ]),
    others: group('uml-others', 'Other UML diagrams', [
        ['Instance specification', 'class'], ['Slot', 'class'], ['Object', 'class'],
        ['Collaboration', 'component'], ['Interaction overview', 'boundary'], ['Timing frame', 'boundary'],
        ['Profile', 'package'], ['Stereotype', 'class'], ['Metaclass', 'class'], ['Extension', 'relation'],
        ['Constraint', 'note'], ['Time observation', 'note'], ['Duration observation', 'note']
    ])
};
const c4 = {
    context: group('c4-context', 'System context', [
        ['Person', 'person'], ['External person', 'person'], ['Software system', 'system'],
        ['External system', 'external'], ['Enterprise boundary', 'boundary'], ['System boundary', 'boundary'],
        ['Relationship', 'relation'], ['Bidirectional relationship', 'relation']
    ]),
    container: group('c4-containers', 'Container view', [
        ['Container / application', 'container'], ['Web application', 'container'], ['Mobile application', 'container'],
        ['Microservice', 'container'], ['API', 'component'], ['Message broker', 'container'],
        ['Database', 'database'], ['Data store', 'database'], ['Container boundary', 'boundary']
    ]),
    component: group('c4-components', 'Component view', [
        ['Component', 'component'], ['External component', 'external'], ['Component boundary', 'boundary'],
        ['Dependency', 'relation'], ['Asynchronous relationship', 'relation']
    ]),
    supporting: group('c4-support', 'Supporting views', [
        ['Deployment node', 'deployment'], ['Infrastructure node', 'deployment'],
        ['Deployment environment', 'boundary'], ['Container instance', 'container'],
        ['Dynamic step', 'message'], ['System landscape', 'system'],
        ['Code element', 'class'], ['Technology label', 'note'], ['Description', 'note']
    ])
};
const mindmap = {
    topics: group('mindmap-topics', 'Topics', [
        ['Central topic', 'topic'], ['Main topic', 'branch'], ['Subtopic', 'branch'],
        ['Sibling topic', 'branch'], ['Floating topic', 'topic'], ['Callout', 'callout']
    ]),
    structure: group('mindmap-structure', 'Structure', [
        ['Branch connector', 'relation'], ['Relationship', 'relation'], ['Boundary', 'boundary'],
        ['Summary', 'note'], ['Grouping', 'boundary'], ['Cross-link', 'relation']
    ]),
    markers: group('mindmap-markers', 'Annotations & markers', [
        ['Priority marker', 'state'], ['Progress marker', 'state'], ['Flag', 'note'],
        ['Note', 'note'], ['Link', 'relation'], ['Tag', 'callout'], ['Image', 'document']
    ])
};
const flowchart = {
    basic: group('flowchart-basic', 'Common symbols', [
        ['Start / end', 'terminator'], ['Process', 'process'], ['Decision', 'decision'],
        ['Input / output', 'input'], ['Document', 'document'], ['Predefined process', 'predefined'],
        ['Database / storage', 'database'], ['Flow line', 'relation'], ['Connector', 'connector']
    ]),
    advanced: group('flowchart-advanced', 'Extended symbols', [
        ['Manual input', 'input'], ['Manual operation', 'process'], ['Preparation', 'process'],
        ['Delay', 'terminator'], ['Display', 'document'], ['Multiple documents', 'document'],
        ['Stored data', 'database'], ['Internal storage', 'database'], ['Direct access storage', 'database'],
        ['Sequential access storage', 'database'], ['Off-page connector', 'connector'],
        ['Annotation', 'note'], ['Subprocess', 'predefined']
    ]),
    lanes: group('flowchart-lanes', 'Layout & connectors', [
        ['Swimlane', 'boundary'], ['Pool', 'boundary'], ['Group', 'boundary'],
        ['Yes / no branch', 'relation'], ['Arrow connector', 'relation'], ['Dotted connector', 'relation']
    ])
};
exports.catalog = {
    uml: Object.values(u), c4: Object.values(c4), mindmap: Object.values(mindmap), flowchart: Object.values(flowchart)
};
exports.familyTotals = Object.fromEntries(Object.entries(exports.catalog).map(([id, groups]) => [
    id, groups.reduce((total, g) => total + g.items.length, 0)
]));

};
__modules["src/components/panels/ModelExplorerPane.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.ModelExplorerPane = ModelExplorerPane;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
const DockPanel_1 = require("../dock/DockPanel");
const diagramData_1 = require("../../app/diagramData");
/** Read-only tree specimen; selected identity is defined by scene data. */
function ModelExplorerPane({ family, open, onToggle, height }) {
    const node = diagramData_1.elements[family], diagram = diagramData_1.diagrams.find(d => d.family === family);
    const extras = family === 'uml' ? ['Order', 'OrderRepository', 'OrderItem', 'Product', 'Customer']
        : family === 'c4' ? ['Web application', 'Database', 'Payment gateway']
            : family === 'mindmap' ? ['Research', 'Audience', 'Roadmap', 'Delivery']
                : ['Receive request', 'Persist order', 'Send event'];
    const style = { flex: `0 0 ${open ? height : 36}px` };
    return React.createElement("section", { className: "explorer-panel", style: style },
        React.createElement(DockPanel_1.DockPanelHeader, { title: "Model explorer", open: open, onToggle: onToggle, id: "explorer-content" }),
        open && React.createElement("div", { id: "explorer-content", className: "explorer-inner" },
            React.createElement("div", { className: "explorer-root" },
                React.createElement(Icon_1.Icon, { name: "chevron-down", size: 12 }),
                React.createElement(Icon_1.Icon, { name: "folder", size: 14 }),
                React.createElement("strong", null, "Commerce platform")),
            React.createElement("div", { className: "tree-line level-1" },
                React.createElement(Icon_1.Icon, { name: "chevron-down", size: 12 }),
                React.createElement(Icon_1.Icon, { name: "folder", size: 14 }),
                React.createElement("span", null, "Architecture")),
            React.createElement("div", { className: "tree-line level-2" },
                React.createElement(Icon_1.Icon, { name: "chevron-down", size: 12 }),
                React.createElement(Icon_1.Icon, { name: family === 'uml' ? 'box' : family === 'c4' ? 'layers' : family === 'mindmap' ? 'tree' : 'corner', size: 14 }),
                React.createElement("span", null, diagram.label)),
            React.createElement("div", { className: "tree-line level-3 is-selected" },
                React.createElement("span", { className: "tree-indent-spacer" }),
                React.createElement(Icon_1.Icon, { name: "box", size: 13 }),
                React.createElement("span", null, node.name)),
            extras.map(s => React.createElement("div", { className: "tree-line level-3", key: s },
                React.createElement("span", { className: "tree-indent-spacer" }),
                React.createElement(Icon_1.Icon, { name: "box", size: 12 }),
                React.createElement("span", null, s)))));
}

};
__modules["src/components/panels/PropertyInspectorPane.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.PropertyInspectorPane = PropertyInspectorPane;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
const Controls_1 = require("../primitives/Controls");
const diagramData_1 = require("../../app/diagramData");
const ShapeGlyph_1 = require("../palette/ShapeGlyph");
/** Property editor preview; no domain writes or canvas mutation. */
function PropertyInspectorPane(p) {
    const node = diagramData_1.elements[p.family];
    return (React.createElement("section", { className: `inspector-panel ${p.open ? '' : 'dock-closed'}` },
        React.createElement("div", { className: "inspector-tabbar" },
            React.createElement(Controls_1.IconButton, { icon: p.open ? 'chevron-down' : 'chevron-right', small: true, label: p.open ? 'Collapse properties' : 'Expand properties', onClick: p.onToggle }),
            React.createElement("button", { "aria-label": "Properties", className: p.tab === 'properties' ? 'active' : '', onClick: () => { p.onTab('properties'); if (!p.open)
                    p.onToggle(); } }, "Properties"),
            p.open && React.createElement("button", { className: p.tab === 'styles' ? 'active' : '', onClick: () => p.onTab('styles') }, "Style")),
        p.open && React.createElement("div", { className: "inspector-scroll", id: "inspector-content" },
            React.createElement("div", { className: "element-identity" },
                React.createElement("span", { className: "identity-icon" },
                    React.createElement(ShapeGlyph_1.ShapeGlyph, { kind: p.family === 'uml' ? 'class' : p.family === 'c4' ? 'container' : p.family === 'mindmap' ? 'topic' : 'decision', size: 28 })),
                React.createElement("div", null,
                    React.createElement("h3", null, node.name),
                    React.createElement("p", null,
                        node.kind,
                        " \u00B7 Selected"))),
            p.tab === 'properties' ? React.createElement(React.Fragment, null,
                React.createElement(Controls_1.SectionHeading, { title: "GENERAL" }),
                React.createElement("div", { className: "property-list" },
                    React.createElement(Prop, { label: "Name" },
                        React.createElement("span", { className: "faux-input" }, node.name)),
                    React.createElement(Prop, { label: "Type" },
                        React.createElement("span", { className: "faux-select" },
                            node.kind,
                            React.createElement(Icon_1.Icon, { name: "chevron-down", size: 12 }))),
                    React.createElement(Prop, { label: "Namespace" },
                        React.createElement("span", { className: "faux-input muted-field" }, node.namespace)),
                    React.createElement(Prop, { label: "Visibility" },
                        React.createElement("span", { className: "faux-select" },
                            "public",
                            React.createElement(Icon_1.Icon, { name: "chevron-down", size: 12 })))),
                React.createElement(Controls_1.SectionHeading, { title: "ATTRIBUTES" }),
                React.createElement("div", { className: "attribute-row" },
                    React.createElement("span", null, "+"),
                    "id: UUID"),
                React.createElement("div", { className: "attribute-row" },
                    React.createElement("span", null, "+"),
                    "status: OrderStatus"),
                React.createElement("details", { className: "inspector-details" },
                    React.createElement("summary", null, "Documentation"),
                    React.createElement("p", null, node.notes)),
                React.createElement("details", { className: "inspector-details" },
                    React.createElement("summary", null, "Advanced"),
                    React.createElement("div", { className: "metadata-row" },
                        React.createElement("span", null, "Abstract"),
                        React.createElement(Controls_1.Toggle, { label: "Abstract" })),
                    React.createElement("div", { className: "metadata-row" },
                        React.createElement("span", null, "Active"),
                        React.createElement(Controls_1.Toggle, { checked: true, label: "Active" })))) : React.createElement(React.Fragment, null,
                React.createElement(Controls_1.SectionHeading, { title: "FILL & STROKE" }),
                React.createElement("div", { className: "appearance-field" },
                    React.createElement("span", null, "Fill"),
                    React.createElement("div", { className: "swatch dark-swatch" }),
                    React.createElement("div", { className: "faux-input" }, "#FFFFFF")),
                React.createElement("div", { className: "appearance-field" },
                    React.createElement("span", null, "Stroke"),
                    React.createElement("div", { className: "swatch stroke-swatch" }),
                    React.createElement("div", { className: "faux-input" }, "#475569")),
                React.createElement("div", { className: "appearance-field" },
                    React.createElement("span", null, "Width"),
                    React.createElement("div", { className: "faux-select" },
                        "1.5 px",
                        React.createElement(Icon_1.Icon, { name: "chevron-down", size: 12 }))),
                React.createElement(Controls_1.SectionHeading, { title: "TYPOGRAPHY" }),
                React.createElement("div", { className: "appearance-field" },
                    React.createElement("span", null, "Font"),
                    React.createElement("div", { className: "faux-select" },
                        "Inter",
                        React.createElement(Icon_1.Icon, { name: "chevron-down", size: 12 }))),
                React.createElement("div", { className: "appearance-field" },
                    React.createElement("span", null, "Size"),
                    React.createElement("div", { className: "faux-input" }, "13 px")),
                React.createElement("div", { className: "align-buttons" },
                    React.createElement("button", { "aria-label": "Align left" },
                        React.createElement(Icon_1.Icon, { name: "align-left", size: 15 })),
                    React.createElement("button", { className: "on", "aria-label": "Align center" },
                        React.createElement(Icon_1.Icon, { name: "align-center", size: 15 })),
                    React.createElement("button", { "aria-label": "Align right" },
                        React.createElement(Icon_1.Icon, { name: "align-right", size: 15 }))),
                React.createElement(Controls_1.SectionHeading, { title: "GEOMETRY" }),
                React.createElement("div", { className: "position-grid" },
                    React.createElement("span", null, "X"),
                    React.createElement("div", { className: "faux-input" }, "130"),
                    React.createElement("span", null, "Y"),
                    React.createElement("div", { className: "faux-input" }, "95"),
                    React.createElement("span", null, "W"),
                    React.createElement("div", { className: "faux-input" }, "218"),
                    React.createElement("span", null, "H"),
                    React.createElement("div", { className: "faux-input" }, "132"))))));
}
function Prop({ label, children }) {
    return React.createElement("div", { className: "property-row" },
        React.createElement("span", null, label),
        React.createElement("div", null, children));
}

};
__modules["src/components/panels/ToolboxPane.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.ToolboxPane = void 0;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
const DockPanel_1 = require("../dock/DockPanel");
const catalog_1 = require("../palette/catalog");
const ShapeGlyph_1 = require("../palette/ShapeGlyph");
const favorites = {
    uml: ['Class', 'Interface', 'Association', 'Composition', 'Package', 'Comment'],
    c4: ['Person', 'Software system', 'Container / application', 'Database', 'Relationship', 'System boundary'],
    mindmap: ['Central topic', 'Main topic', 'Subtopic', 'Branch connector', 'Callout'],
    flowchart: ['Start / end', 'Process', 'Decision', 'Flow line', 'Document', 'Database / storage']
};
/** Symbol vocabulary + one search control; no dependence on canvas or panel host. */
class ToolboxPane extends React.Component {
    constructor() {
        super(...arguments);
        this.state = { searchOpen: false };
        this.entry = (i, quick = false) => React.createElement("button", { key: i.id, title: i.label, onClick: () => this.props.onChoose(i.id), className: `palette-entry ${quick ? 'quick-entry' : ''} ${this.props.chosen === i.id ? 'active' : ''}` },
            React.createElement("span", { className: "palette-glyph" },
                React.createElement(ShapeGlyph_1.ShapeGlyph, { kind: i.kind, size: 23 })),
            React.createElement("span", { className: "palette-label" }, i.label));
    }
    render() {
        const p = this.props, query = p.search.trim().toLowerCase(), searching = query.length > 0;
        const groups = catalog_1.catalog[p.family].map(g => ({ ...g, items: g.items.filter(i => i.label.toLowerCase().includes(query)) })).filter(g => g.items.length);
        const all = catalog_1.catalog[p.family].flatMap(g => g.items);
        const quick = favorites[p.family].map(name => all.find(i => i.label === name)).filter((x) => !!x);
        return React.createElement("section", { className: `toolbox-panel ${p.open ? '' : 'dock-closed'}` },
            React.createElement(DockPanel_1.DockPanelHeader, { title: "Toolbox", open: p.open, onToggle: p.onToggle, id: "toolbox-contents", action: React.createElement("button", { className: "header-icon", "aria-label": this.state.searchOpen ? 'Close search' : 'Search tools', title: "Search symbols", onClick: () => this.setState(s => ({ searchOpen: !s.searchOpen })) },
                    React.createElement(Icon_1.Icon, { name: "search", size: 14 })) }),
            p.open && React.createElement("div", { className: "toolbox-main", id: "toolbox-contents" },
                this.state.searchOpen && React.createElement("label", { className: "side-search" },
                    React.createElement(Icon_1.Icon, { name: "search", size: 13 }),
                    React.createElement("input", { autoFocus: true, value: p.search, placeholder: "Search symbols\u2026", "aria-label": "Search symbols", onChange: e => p.onSearch(e.currentTarget.value) }),
                    p.search && React.createElement("button", { className: "clean-button", onClick: () => p.onSearch(''), "aria-label": "Clear search" },
                        React.createElement(Icon_1.Icon, { name: "close", size: 13 }))),
                React.createElement("div", { className: "palette-scroll" },
                    !searching && React.createElement("section", { className: "palette-quick" },
                        React.createElement("div", { className: "palette-quick-header" }, "QUICK TOOLS"),
                        React.createElement("div", { className: "palette-quick-list" }, quick.map(i => this.entry(i, true)))),
                    React.createElement("div", { className: "palette-browser-label" }, searching ? 'RESULTS' : 'SYMBOL LIBRARY'),
                    groups.map(g => React.createElement("section", { key: g.id, className: "palette-section" },
                        React.createElement("button", { className: "palette-group-header", onClick: () => p.onToggleGroup(g.id), "aria-expanded": searching || !!p.collapsed[g.id] },
                            React.createElement(Icon_1.Icon, { name: searching || p.collapsed[g.id] ? 'chevron-down' : 'chevron-right', size: 12 }),
                            React.createElement("span", null, g.title),
                            React.createElement("span", { className: "group-count" }, g.items.length)),
                        (searching || !!p.collapsed[g.id]) && React.createElement("div", { className: "palette-entries" }, g.items.map(i => this.entry(i))))),
                    !groups.length && React.createElement("div", { className: "empty-compact" }, "No symbols found."))));
    }
}
exports.ToolboxPane = ToolboxPane;

};
__modules["src/components/panels/WorkingDiagramsPane.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.WorkingDiagramsPane = WorkingDiagramsPane;
const React = __importStar(require("react"));
const Icon_1 = require("../primitives/Icon");
const diagramData_1 = require("../../app/diagramData");
const DockPanel_1 = require("../dock/DockPanel");
/** Reusable working-diagram dock content. Contains no resize or placement logic. */
function WorkingDiagramsPane({ family, onFamily, open, onToggle, height }) {
    return React.createElement("section", { className: "left-project-panel", style: { flex: `0 0 ${open ? height : 36}px` } },
        React.createElement(DockPanel_1.DockPanelHeader, { title: "Working diagrams", open: open, onToggle: onToggle, id: "working-diagrams" }),
        open && React.createElement("div", { id: "working-diagrams", className: "left-project-content" },
            React.createElement("div", { className: "project-group" },
                React.createElement(Icon_1.Icon, { name: "chevron-down", size: 12 }),
                React.createElement(Icon_1.Icon, { name: "folder", size: 14 }),
                React.createElement("span", null, "Commerce platform")),
            React.createElement("div", { className: "diagram-list" }, diagramData_1.diagrams.map(d => React.createElement("button", { key: d.family, onClick: () => onFamily(d.family), className: `diagram-item ${family === d.family ? 'active' : ''}`, "aria-current": family === d.family ? 'page' : undefined },
                React.createElement(Icon_1.Icon, { name: d.family === 'uml' ? 'box' : d.family === 'c4' ? 'layers' : d.family === 'mindmap' ? 'tree' : 'corner', size: 14 }),
                React.createElement("span", null, d.label))))));
}

};
__modules["src/components/primitives/Controls.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.IconButton = IconButton;
exports.SectionHeading = SectionHeading;
exports.PanelHeader = PanelHeader;
exports.Kbd = Kbd;
exports.Toggle = Toggle;
const React = __importStar(require("react"));
const Icon_1 = require("./Icon");
function IconButton({ icon, label, active, onClick, small, disabled }) {
    return React.createElement("button", { type: "button", className: `icon-button${active ? ' is-active' : ''}${small ? ' is-small' : ''}`, "aria-label": label, title: label, onClick: onClick, disabled: disabled },
        React.createElement(Icon_1.Icon, { name: icon, size: small ? 14 : 16 }));
}
function SectionHeading({ title, count, action }) {
    return React.createElement("div", { className: "section-heading" },
        React.createElement("span", null, title),
        count !== undefined && React.createElement("span", { className: "subtle-count" }, count),
        action && React.createElement("div", { className: "section-actions" }, action));
}
function PanelHeader({ title, right }) {
    return React.createElement("div", { className: "panel-header" },
        React.createElement("span", null, title),
        React.createElement("div", { className: "panel-header-actions" }, right));
}
function Kbd({ children }) { return React.createElement("kbd", { className: "kbd" }, children); }
function Toggle({ checked, label }) { return React.createElement("span", { className: `toggle-switch ${checked ? 'on' : ''}`, "aria-label": label },
    React.createElement("span", null)); }

};
__modules["src/components/primitives/Icon.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.Icon = Icon;
const React = __importStar(require("react"));
const common = { fill: 'none', stroke: 'currentColor', strokeLinecap: 'round', strokeLinejoin: 'round' };
function Icon({ name, size = 16, strokeWidth = 1.75, className }) {
    const paths = {
        app: React.createElement(React.Fragment, null,
            React.createElement("rect", { x: "3", y: "3", width: "7", height: "7", rx: "1" }),
            React.createElement("rect", { x: "14", y: "3", width: "7", height: "7", rx: "1" }),
            React.createElement("rect", { x: "3", y: "14", width: "7", height: "7", rx: "1" }),
            React.createElement("path", { d: "M14 14h7v7h-7z" })),
        layers: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "m12 3 9 5-9 5-9-5 9-5Z" }),
            React.createElement("path", { d: "m3 12 9 5 9-5M3 16l9 5 9-5" })),
        shapes: React.createElement(React.Fragment, null,
            React.createElement("rect", { x: "3", y: "3", width: "9", height: "9", rx: "1" }),
            React.createElement("circle", { cx: "17.5", cy: "17.5", r: "4.5" }),
            React.createElement("path", { d: "M5 20h7" })),
        tree: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M12 3v5M12 8H5v6M12 8h7v6M5 14v3M19 14v3M12 8v9" }),
            React.createElement("rect", { x: "2", y: "17", width: "6", height: "4", rx: "1" }),
            React.createElement("rect", { x: "9", y: "17", width: "6", height: "4", rx: "1" }),
            React.createElement("rect", { x: "16", y: "17", width: "6", height: "4", rx: "1" })),
        palette: React.createElement(React.Fragment, null,
            React.createElement("circle", { cx: "12", cy: "12", r: "9" }),
            React.createElement("circle", { cx: "7", cy: "11", r: "1" }),
            React.createElement("circle", { cx: "11", cy: "7", r: "1" }),
            React.createElement("circle", { cx: "17", cy: "9", r: "1" }),
            React.createElement("path", { d: "M12 21c-2-1-2-3 0-4s4-1 5-3" })),
        settings: React.createElement(React.Fragment, null,
            React.createElement("circle", { cx: "12", cy: "12", r: "3" }),
            React.createElement("path", { d: "M10 2h4l.6 2.3 2.1.9 2-.9 2.8 2.8-.9 2 1 2.2L24 12l-2.4.7-1 2.1.9 2-2.8 2.8-2-.9-2.1.9L14 22h-4l-.6-2.4-2.1-.9-2 .9-2.8-2.8.9-2-1-2.1L0 12l2.4-.7 1-2.2-.9-2 2.8-2.8 2 .9 2.1-.9z", transform: "translate(1.2 1.2) scale(.9)" })),
        'chevron-down': React.createElement("path", { d: "m6 9 6 6 6-6" }), 'chevron-right': React.createElement("path", { d: "m9 6 6 6-6 6" }),
        search: React.createElement(React.Fragment, null,
            React.createElement("circle", { cx: "10.5", cy: "10.5", r: "6.5" }),
            React.createElement("path", { d: "m16 16 5 5" })),
        plus: React.createElement("path", { d: "M12 5v14M5 12h14" }), more: React.createElement(React.Fragment, null,
            React.createElement("circle", { cx: "5", cy: "12", r: "1.2" }),
            React.createElement("circle", { cx: "12", cy: "12", r: "1.2" }),
            React.createElement("circle", { cx: "19", cy: "12", r: "1.2" })),
        undo: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M9 14 4 9l5-5" }),
            React.createElement("path", { d: "M4 9h9a7 7 0 0 1 0 14" })),
        redo: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "m15 14 5-5-5-5" }),
            React.createElement("path", { d: "M20 9h-9a7 7 0 0 0 0 14" })),
        mouse: React.createElement("path", { d: "M5 3 19 12l-6.3 1.1L10 20z" }),
        hand: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M8 13V6a2 2 0 0 1 4 0v5-6a2 2 0 0 1 4 0v6-4a2 2 0 0 1 4 0v9a6 6 0 0 1-6 6h-3a7 7 0 0 1-5.8-3L2 14a2 2 0 0 1 3-2z" })),
        box: React.createElement("rect", { x: "4", y: "4", width: "16", height: "16", rx: "1" }), circle: React.createElement("circle", { cx: "12", cy: "12", r: "8" }), diamond: React.createElement("path", { d: "M12 2 22 12 12 22 2 12z" }),
        link: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M10 13a5 5 0 0 0 7 .5l3-3a5 5 0 0 0-7-7l-2 2" }),
            React.createElement("path", { d: "M14 11a5 5 0 0 0-7-.5l-3 3a5 5 0 0 0 7 7l2-2" })),
        'zoom-in': React.createElement(React.Fragment, null,
            React.createElement("circle", { cx: "10.5", cy: "10.5", r: "7" }),
            React.createElement("path", { d: "M10.5 7v7M7 10.5h7M16 16l5 5" })),
        'zoom-out': React.createElement(React.Fragment, null,
            React.createElement("circle", { cx: "10.5", cy: "10.5", r: "7" }),
            React.createElement("path", { d: "M7 10.5h7M16 16l5 5" })),
        grid: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M3 3h18v18H3zM9 3v18M15 3v18M3 9h18M3 15h18" })),
        focus: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M8 3H3v5M16 3h5v5M3 16v5h5M21 16v5h-5" }),
            React.createElement("circle", { cx: "12", cy: "12", r: "3" })),
        'panel-bottom': React.createElement(React.Fragment, null,
            React.createElement("rect", { x: "3", y: "3", width: "18", height: "18", rx: "2" }),
            React.createElement("path", { d: "M3 15h18" })),
        eye: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z" }),
            React.createElement("circle", { cx: "12", cy: "12", r: "3" })),
        lock: React.createElement(React.Fragment, null,
            React.createElement("rect", { x: "5", y: "10", width: "14", height: "11", rx: "2" }),
            React.createElement("path", { d: "M8 10V7a4 4 0 0 1 8 0v3" })),
        check: React.createElement("path", { d: "m5 12 5 5L20 7" }), info: React.createElement(React.Fragment, null,
            React.createElement("circle", { cx: "12", cy: "12", r: "9" }),
            React.createElement("path", { d: "M12 11v6M12 7h.01" })),
        code: React.createElement("path", { d: "m8 8-5 4 5 4m8-8 5 4-5 4m-3-12-3 20" }),
        warning: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M12 3 2 21h20L12 3z" }),
            React.createElement("path", { d: "M12 9v5M12 18h.01" })),
        close: React.createElement("path", { d: "M5 5l14 14M19 5 5 19" }), sliders: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M4 7h16M4 17h16" }),
            React.createElement("circle", { cx: "9", cy: "7", r: "2.5", fill: "var(--surface-2)" }),
            React.createElement("circle", { cx: "15", cy: "17", r: "2.5", fill: "var(--surface-2)" })),
        download: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M12 3v13m-5-5 5 5 5-5M4 17v4h16v-4" })),
        sun: React.createElement(React.Fragment, null,
            React.createElement("circle", { cx: "12", cy: "12", r: "4" }),
            React.createElement("path", { d: "M12 2v2m0 16v2M2 12h2m16 0h2M4.9 4.9l1.5 1.5m11.2 11.2 1.5 1.5M19.1 4.9l-1.5 1.5M6.4 17.6l-1.5 1.5" })),
        moon: React.createElement("path", { d: "M20 15.5A8.5 8.5 0 0 1 8.5 4 8.5 8.5 0 1 0 20 15.5Z" }),
        command: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M9 8V5a2 2 0 1 0-2 2h10a2 2 0 1 0-2-2v14a2 2 0 1 0 2-2H7a2 2 0 1 0 2 2z" })),
        'align-left': React.createElement("path", { d: "M4 4v16m4-13h12M8 12h8M8 17h12" }),
        'align-center': React.createElement("path", { d: "M12 3v18M4 7h16M7 12h10M4 17h16" }),
        'align-right': React.createElement("path", { d: "M20 4v16M4 7h12M8 12h8M4 17h12" }),
        ruler: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M3 8 16 21l5-5L8 3z" }),
            React.createElement("path", { d: "m7 12 3-3m1 7 3-3" })),
        'arrow-right': React.createElement("path", { d: "M4 12h16m-6-6 6 6-6 6" }),
        file: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "M6 2h8l5 5v15H6z" }),
            React.createElement("path", { d: "M14 2v6h5" })),
        folder: React.createElement("path", { d: "M3 6h7l2 2h9v12H3z" }),
        type: React.createElement("path", { d: "M4 7V4h16v3m-8-3v16m-4 0h8" }),
        list: React.createElement("path", { d: "M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" }),
        help: React.createElement(React.Fragment, null,
            React.createElement("circle", { cx: "12", cy: "12", r: "9" }),
            React.createElement("path", { d: "M9.5 9a3 3 0 0 1 5.8 1c0 2-3.3 2.5-3.3 5M12 18h.01" })),
        pin: React.createElement(React.Fragment, null,
            React.createElement("path", { d: "m14 3 7 7-3 1-4 4-1 6-3-7-7-3 6-1 4-4z" })),
        'chevrons-left': React.createElement("path", { d: "m11 17-5-5 5-5m7 10-5-5 5-5" }),
        'chevrons-right': React.createElement("path", { d: "m6 17 5-5-5-5m7 10 5-5-5-5" }),
        fullscreen: React.createElement("path", { d: "M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5" }),
        copy: React.createElement(React.Fragment, null,
            React.createElement("rect", { x: "8", y: "8", width: "12", height: "12", rx: "1" }),
            React.createElement("path", { d: "M16 8V4H4v12h4" })),
        image: React.createElement(React.Fragment, null,
            React.createElement("rect", { x: "3", y: "3", width: "18", height: "18", rx: "2" }),
            React.createElement("circle", { cx: "8", cy: "8", r: "2" }),
            React.createElement("path", { d: "m3 18 6-6 4 4 3-3 5 5" })),
        dots: React.createElement(React.Fragment, null,
            React.createElement("circle", { cx: "6", cy: "6", r: "1" }),
            React.createElement("circle", { cx: "12", cy: "6", r: "1" }),
            React.createElement("circle", { cx: "18", cy: "6", r: "1" }),
            React.createElement("circle", { cx: "6", cy: "12", r: "1" }),
            React.createElement("circle", { cx: "12", cy: "12", r: "1" }),
            React.createElement("circle", { cx: "18", cy: "12", r: "1" }),
            React.createElement("circle", { cx: "6", cy: "18", r: "1" }),
            React.createElement("circle", { cx: "12", cy: "18", r: "1" }),
            React.createElement("circle", { cx: "18", cy: "18", r: "1" })),
        corner: React.createElement("path", { d: "M4 4v16h16M4 12h9v8" }),
        spark: React.createElement("path", { d: "m12 2 2.6 7.4L22 12l-7.4 2.6L12 22l-2.6-7.4L2 12l7.4-2.6z" }),
        'panel-left': React.createElement(React.Fragment, null,
            React.createElement("rect", { x: "3", y: "3", width: "18", height: "18", rx: "2" }),
            React.createElement("path", { d: "M9 3v18" })),
        'panel-right': React.createElement(React.Fragment, null,
            React.createElement("rect", { x: "3", y: "3", width: "18", height: "18", rx: "2" }),
            React.createElement("path", { d: "M15 3v18" })),
        monitor: React.createElement(React.Fragment, null,
            React.createElement("rect", { x: "2", y: "4", width: "20", height: "15", rx: "2" }),
            React.createElement("path", { d: "M8 22h8m-4-3v3" })),
        'layers-two': React.createElement(React.Fragment, null,
            React.createElement("path", { d: "m12 3 9 5-9 5-9-5zM3 13l9 5 9-5" })),
        layout: React.createElement(React.Fragment, null,
            React.createElement("rect", { x: "3", y: "3", width: "18", height: "18", rx: "2" }),
            React.createElement("path", { d: "M3 9h18M9 9v12" }))
    };
    return React.createElement("svg", { className: className, width: size, height: size, viewBox: "0 0 24 24", strokeWidth: strokeWidth, ...common, "aria-hidden": "true" }, paths[name]);
}

};
__modules["src/main.tsx"] = function(require,module,exports){
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
const React = __importStar(require("react"));
const client_1 = require("react-dom/client");
const App_1 = require("./app/App");
require("./styles/tokens.css");
require("./styles/base.css");
require("./styles/app.css");
require("./styles/diagrams.css");
require("./styles/gallery.css");
const root = document.getElementById('root');
if (root)
    (0, client_1.createRoot)(root).render(React.createElement(React.StrictMode, null,
        React.createElement(App_1.App, null)));

};
function normalize(base,req){if(req==='react'||req==='react-dom/client')return req;if(req.endsWith('.css'))return 'css';let p=req[0]==='.'?base.split('/').slice(0,-1).concat(req.split('/')):req.split('/');let stack=[];for(let s of p){if(s==='.'||!s)continue;if(s==='..')stack.pop();else stack.push(s);}let x=stack.join('/');if(!/\.tsx?$/.test(x))x+='.tsx';if(!__modules[x])x=x.replace(/\.tsx$/,'.ts');return x;}
function load(id){if(id==='react')return window.React;if(id==='react-dom/client')return {createRoot:function(el){return {render:function(node){window.ReactDOM.render(node,el)}}}};if(id==='css')return {};if(__cache[id])return __cache[id].exports;if(!__modules[id])throw new Error('Missing module: '+id);let module={exports:{}};__cache[id]=module;__modules[id](req=>load(normalize(id,req)),module,module.exports);return module.exports;}
window.React.Fragment=window.React.Fragment||function(p){return p.children};window.React.StrictMode=window.React.StrictMode||window.React.Fragment;window.React.useState=window.React.useState||function(v){return [typeof v==='function'?v():v,function(){}]};load('src/main.tsx');})();