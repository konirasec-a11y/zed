/*
 * Copyright (c) 2013-2023 Minkyu Lee. All rights reserved.
 *
 * NOTICE:  All information contained herein is, and remains the
 * property of Minkyu Lee. The intellectual and technical concepts
 * contained herein are proprietary to Minkyu Lee and may be covered
 * by Republic of Korea and Foreign Patents, patents in process,
 * and are protected by trade secret or copyright law.
 * Dissemination of this information or reproduction of this material
 * is strictly forbidden unless prior written permission is obtained
 * from Minkyu Lee (niklaus.lee@gmail.com).
 *
 */

const path = require("path");

const {
  ExtensibleModel,
  DirectedRelationship,
  Diagram,
  NodeView,
  EdgeView,
  LabelView,
  NodeLabelView,
  Hyperlink,
  EdgeParasiticView,
  Canvas,
} = app.type;

const LEFT_PADDING = 10;
const RIGHT_PADDING = 10;
const TOP_PADDING = 10;
const BOTTOM_PADDING = 10;

const ICON_MINWIDTH = 48;
const ICON_MINHEIGHT = 48;

const CALLOUT_MINWIDTH = 24;
const CALLOUT_MINHEIGHT = 24;

const GROUP_ROUND = 10;

/**
 * Load and draw image of view
 */
function loadAndDrawImage(
  canvas,
  icon,
  basePath,
  file,
  left,
  top,
  width,
  height,
) {
  if (icon.state === 2) {
    // loaded
    canvas.drawImage(icon.img, left, top, width, height);
  } else if (icon.state === 0) {
    icon.state = 1;
    if (file.length > 0) {
      icon.img.src = `file://${path.join(basePath, file)}`;
    }
    icon.img.onload = () => {
      icon.state = 2;
      canvas.drawImage(icon.img, left, top, width, height);
    };
    icon.img.onerror = () => {
      icon.state = 4; // error
    };
  }
  if (file.length === 0 || icon.state === 4) {
    canvas.line(left, top, left + width, top + height);
    canvas.line(left + width, top, left, top + height);
    canvas.rect(left, top, left + width, top + height);
  }
}

/**
 * AzureElement
 */
class AzureElement extends ExtensibleModel {
  constructor() {
    super();

    /** @type {string} */
    this.icon = "";

    // indicate whether to popup icon selector when double clicked
    this.__hasIcon = false;
  }

  getDisplayClassName() {
    return this.getClassName();
  }
}

/**
 * AzureDiagram
 */
class AzureDiagram extends Diagram {
  canAcceptModel(model) {
    return (
      model instanceof Hyperlink ||
      model instanceof Diagram ||
      model instanceof type.AzureElement ||
      model instanceof type.AzureConnector
    );
  }
}

/**
 * AzureModel
 */
class AzureModel extends AzureElement {}

/**
 * AzureGroup
 */
class AzureGroup extends AzureElement {
  constructor() {
    super();

    /** @type {boolean} */
    this.dashed = false;

    /** @type {boolean} */
    this.rounded = false;
  }

  canContainKind(kind) {
    return app.metamodels.isKindOf(kind, "AzureElement");
  }
}

/**
 * AzureService
 */
class AzureService extends AzureElement {
  constructor() {
    super();
    this.icon = "App-Services.svg"; // default service
    this.__hasIcon = true;
    this.__assetBaseDir = "azure";
  }
}

/**
 * AzureCallout
 */
class AzureCallout extends AzureElement {
  constructor() {
    super();
    this.icon = "";
    this.__hasIcon = false;
    this.name = "1";
  }
}

/**
 * AzureConnector
 */
class AzureConnector extends DirectedRelationship {
  constructor() {
    super();

    /** @type {boolean} */
    this.dashed = false;
  }
}

/* ---------------------------- View Elements ------------------------------- */

/**
 * AzureGeneralNodeView
 */
class AzureGeneralNodeView extends NodeView {
  constructor() {
    super();
    this.containerChangeable = true;

    /** @type {boolean} */
    this.wordWrap = true;

    /** @type {LabelView} */
    this.nameLabel = new LabelView();
    this.nameLabel.parentStyle = true;
    this.nameLabel.horizontalAlignment = Canvas.AL_CENTER;
    this.nameLabel.verticalAlignment = Canvas.AL_MIDDLE;
    this.addSubView(this.nameLabel);
  }

  update(canvas) {
    super.update(canvas);
    if (this.model) {
      this.nameLabel.text = this.model.name;
    }
    this.nameLabel.wordWrap = this.wordWrap;
  }

  sizeObject(canvas) {
    super.sizeObject(canvas);
    this.minWidth = this.nameLabel.minWidth + LEFT_PADDING + RIGHT_PADDING;
    this.minHeight = this.nameLabel.minHeight + TOP_PADDING + BOTTOM_PADDING;
  }

  arrangeObject(canvas) {
    super.arrangeObject(canvas);
    this.nameLabel.left = this.left + LEFT_PADDING;
    this.nameLabel.top = this.top + TOP_PADDING;
    this.nameLabel.width = this.width - (LEFT_PADDING + RIGHT_PADDING);
    this.nameLabel.height = this.nameLabel.minHeight;
  }

  drawObject(canvas) {
    super.drawObject(canvas);
  }

  canDelete() {
    return false;
  }
}

/**
 * AzureIconNodeView
 */
class AzureIconNodeView extends NodeView {
  constructor() {
    super();
    this.containerExtending = false;
    this.containerChangeable = true;
    this.sizable = NodeView.SZ_RATIO;

    /** @member {NodeLabelView} */
    this.nameLabel = new NodeLabelView();
    this.nameLabel.distance = 42;
    this.nameLabel.alpha = -2 * (Math.PI / 4);
    this.addSubView(this.nameLabel);

    this.__icon = {
      img: new Image(),
      state: 0, // 0 = not loaded, 1 = loading, 2 = loaded, 4 = error
    };
  }

  sizeObject(canvas) {
    super.sizeObject(canvas);
    this.minWidth = ICON_MINWIDTH;
    this.minHeight = ICON_MINHEIGHT;
  }

  update(canvas) {
    super.update(canvas);
    if (this.model) {
      this.nameLabel.text = this.model.name;
      // nameLabel이 model을 정상적으로 reference 할 수 있도록 Bypass Command에 의해서 설정한다.
      if (this.nameLabel.model !== this.model) {
        app.repository.bypassFieldAssign(this.nameLabel, "model", this.model);
      }
    }
  }

  arrange(canvas) {
    this.nameLabel.visible = this.nameLabel.text.length > 0;
    super.arrange(canvas);
  }

  drawObject(canvas) {
    super.drawObject(canvas);
    const basePath = path.join(
      __dirname,
      `../../../resources/assets`,
      this.model.__assetBaseDir,
    );
    loadAndDrawImage(
      canvas,
      this.__icon,
      basePath,
      this.model.icon,
      this.left,
      this.top,
      this.width,
      this.height,
    );
  }
}

/**
 * AzureGeneralEdgeView
 */
class AzureGeneralEdgeView extends EdgeView {
  constructor() {
    super();
    this.tailEndStyle = EdgeView.ES_FLAT;
    this.headEndStyle = EdgeView.ES_SOLID_ARROW;
    this.lineMode = EdgeView.LM_SOLID;

    /** @member {EdgeLabelView} */
    this.nameLabel = new type.EdgeLabelView();
    this.nameLabel.hostEdge = this;
    this.nameLabel.edgePosition = EdgeParasiticView.EP_MIDDLE;
    this.nameLabel.distance = 15;
    this.nameLabel.alpha = Math.PI / 2;
    this.addSubView(this.nameLabel);
  }

  update(canvas) {
    if (this.model) {
      // nameLabel
      this.nameLabel.visible = this.model.name.length > 0;
      if (this.model.name) {
        this.nameLabel.text = this.model.name;
      }
      // Enforce nameLabel.mode refers to this.model by using Bypass Command.
      if (this.nameLabel.model !== this.model) {
        app.repository.bypassFieldAssign(this.nameLabel, "model", this.model);
      }
    }
    super.update(canvas);
  }

  canConnectTo(view) {
    return view.model instanceof AzureElement;
  }

  canDelete() {
    return false;
  }
}

/**
 * AzureGroupView
 */
class AzureGroupView extends AzureGeneralNodeView {
  canContainViewKind(kind) {
    return (
      app.metamodels.isKindOf(kind, "AzureGeneralNodeView") ||
      app.metamodels.isKindOf(kind, "AzureIconNodeView")
    );
  }

  drawObject(canvas) {
    super.drawObject(canvas);
    if (this.model.rounded) {
      canvas.fillRoundRect(
        this.left,
        this.top,
        this.getRight(),
        this.getBottom(),
        GROUP_ROUND,
      );
      canvas.roundRect(
        this.left,
        this.top,
        this.getRight(),
        this.getBottom(),
        GROUP_ROUND,
        this.model.dashed ? [3, 3] : [],
      );
    } else {
      canvas.fillRect(this.left, this.top, this.getRight(), this.getBottom());
      canvas.rect(
        this.left,
        this.top,
        this.getRight(),
        this.getBottom(),
        this.model.dashed ? [3, 3] : [],
      );
    }
  }
}

/**
 * AzureServiceView
 */
class AzureServiceView extends AzureIconNodeView {}

/**
 * AzureCalloutView
 */
class AzureCalloutView extends AzureGeneralNodeView {
  constructor() {
    super();
    this.containerExtending = false;
    this.containerChangeable = true;
    this.sizable = NodeView.SZ_RATIO;
    this.fillColor = "#000000";
    this.fontColor = "#ffffff";
  }

  sizeObject(canvas) {
    super.sizeObject(canvas);
    this.minWidth = CALLOUT_MINWIDTH;
    this.minHeight = CALLOUT_MINHEIGHT;
  }

  update(canvas) {
    super.update(canvas);
    if (this.model) {
      this.nameLabel.text = this.model.name;
      // nameLabel이 model을 정상적으로 reference 할 수 있도록 Bypass Command에 의해서 설정한다.
      if (this.nameLabel.model !== this.model) {
        app.repository.bypassFieldAssign(this.nameLabel, "model", this.model);
      }
    }
  }

  arrange(canvas) {
    super.arrange(canvas);
    this.nameLabel.left = this.left;
    this.nameLabel.width = this.width;
    this.nameLabel.height = this.nameLabel.minHeight;
    this.nameLabel.top =
      this.top + Math.round((this.height - this.nameLabel.height) / 2);
  }

  drawObject(canvas) {
    canvas.fillEllipse(this.left, this.top, this.getRight(), this.getBottom());
    super.drawObject(canvas);
  }
}

/**
 * AzureConnectorView
 */
class AzureConnectorView extends AzureGeneralEdgeView {
  constructor() {
    super();
    this.lineStyle =
      app.preferences.get("azure.arrow.lineStyle", EdgeView.LS_ROUNDRECT) ||
      app.preferences.get("view.lineStyle", EdgeView.LS_OBLIQUE);
    this.headEndStyle = EdgeView.ES_SOLID_ARROW;
  }

  update(canvas) {
    if (this.model) {
      if (this.model.dashed) {
        this.lineMode = EdgeView.LM_DOT;
      } else {
        this.lineMode = EdgeView.LM_SOLID;
      }
    }
  }
}

type.AzureElement = AzureElement;
type.AzureDiagram = AzureDiagram;
type.AzureModel = AzureModel;
type.AzureGroup = AzureGroup;
type.AzureService = AzureService;
type.AzureCallout = AzureCallout;
type.AzureConnector = AzureConnector;
type.AzureGeneralNodeView = AzureGeneralNodeView;
type.AzureIconNodeView = AzureIconNodeView;
type.AzureGeneralEdgeView = AzureGeneralEdgeView;
type.AzureGroupView = AzureGroupView;
type.AzureServiceView = AzureServiceView;
type.AzureCalloutView = AzureCalloutView;
type.AzureConnectorView = AzureConnectorView;
