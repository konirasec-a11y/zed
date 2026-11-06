/*
 * Copyright (c) 2014 MKLab. All rights reserved.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a
 * copy of this software and associated documentation files (the "Software"),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 * DEALINGS IN THE SOFTWARE.
 *
 */

/* global CodeMirror */

const Mustache = require("mustache");
const fs = require("fs");
const path = require("path");
const { getDiagramType } = require("./utils");
const { createClassDiagram } = require("./factory/class-factory");
const { createMindmap } = require("./factory/mindmap-factory");
const { createFlowchart } = require("./factory/flowchart-factory");
const { createErDiagram } = require("./factory/er-factory");
const { createStateDiagram } = require("./factory/state-factory");
const { createRequirementDiagram } = require("./factory/requirement-factory");
const { createSequenceDiagram } = require("./factory/sequence-factory");
const {
  classDiagramExample,
  sequenceDiagramExample,
  stateDiagramExample,
  flowchartExample,
  erDiagramExample,
  requirementDiagramExample,
  mindmapExample,
} = require("./examples");

const mermaidDialogTemplate = fs.readFileSync(
  path.join(__dirname, "mermaid-dialog.html"),
  "utf8",
);

const input = `
classDiagram
    Animal <|-- Duck
    Animal <|-- Fish
    Animal <|-- Zebra
    Animal : +int age
    Animal : +String gender
    Animal: +isMammal()
    Animal: +mate()
    class Duck{
        +String beakColor
        +swim()
        +quack()
    }
    class Fish{
        -int sizeInFeet
        -canEat()
    }
    class Zebra{
        +bool is_wild
        +run()
    }
`;

let mermaidEditor;

/**
 * Show Mermaid Dialog
 * @return {$.Promise}
 */
function showMermaidDialog() {
  var dialog = app.dialogs.showModalDialogUsingTemplate(
    Mustache.render(mermaidDialogTemplate),
    true,
    function ($dlg) {
      var val = {
        code: mermaidEditor.getValue(),
        keyword: $dlg.find(".keyword").val(),
        caseSensitive: $dlg.find(".case-sensitive").is(":checked"),
        findInDocumentation: $dlg.find(".find-in-documentation").is(":checked"),
      };
      $dlg.data("returnValue", val);
    },
  );
  var $dlg = dialog.getElement();

  // Setup CodeMirror
  mermaidEditor = CodeMirror.fromTextArea(
    document.getElementById("mermaid-code-editor"),
    {
      lineNumbers: false,
      styleActiveLine: true,
      matchBrackets: true,
      theme: "monokai",
      mode: "text",
    },
  );
  mermaidEditor.setValue(input.trim());

  // example dropdown
  var exampleDropdown = $dlg.find("button.example-dropdown");
  exampleDropdown.on("contextmenu", (e) => {
    e.preventDefault();
    e.stopImmediatePropagation();
  });
  exampleDropdown.click((e) => {
    e.preventDefault();
    app.contextMenu.popup("mermaid-example-dropdown");
    return false;
  });

  // clear button
  var clearButton = $dlg.find("button.clear-button");
  clearButton.click((e) => {
    e.preventDefault();
    mermaidEditor.setValue("");
  });

  return dialog;
}

/**
 * Show mermaid dialog
 */
function handleShowDialog() {
  showMermaidDialog().then(({ buttonId, returnValue }) => {
    if (buttonId === "ok") {
      try {
        const code = returnValue.code.trim();
        handleGenerateDiagram(code);
      } catch (error) {
        console.error(error);
        app.toast.error(error.message);
      }
    }
  });
}

/**
 * Generate diagram by mermaid syntax.
 * @param {string} code - Mermaid syntax code.
 * @param {type.ExtensibleModel} base - The base element where the diagram will be generated.
 */
function handleGenerateDiagram(code, base) {
  const diagramType = getDiagramType(code.trim());
  switch (diagramType) {
    case "classDiagram":
      createClassDiagram(code, base);
      break;
    case "sequenceDiagram":
      createSequenceDiagram(code, base);
      break;
    case "flowchart":
      createFlowchart(code, base);
      break;
    case "erDiagram":
      createErDiagram(code, base);
      break;
    case "mindmap":
      createMindmap(code, base);
      break;
    case "requirementDiagram":
      createRequirementDiagram(code, base);
      break;
    case "stateDiagram":
      createStateDiagram(code, base);
      break;
    default:
      throw new Error(`Unsupported diagram type`);
  }
  return true;
}

function handleSetClassDiagramExample() {
  mermaidEditor.setValue(classDiagramExample.trim());
}

function handleSetSequenceDiagramExample() {
  mermaidEditor.setValue(sequenceDiagramExample.trim());
}

function handleSetStateDiagramExample() {
  mermaidEditor.setValue(stateDiagramExample.trim());
}

function handleSetFlowchartExample() {
  mermaidEditor.setValue(flowchartExample.trim());
}

function handleSetErDiagramExample() {
  mermaidEditor.setValue(erDiagramExample.trim());
}

function handleSetRequirementDiagramExample() {
  mermaidEditor.setValue(requirementDiagramExample.trim());
}

function handleSetMindmapExample() {
  mermaidEditor.setValue(mindmapExample.trim());
}

function init() {
  // register commands
  app.commands.register(
    "mermaid:show-mermaid-dialog",
    handleShowDialog,
    "[Mermaid] Show Mermaid Dialog",
  );
  // register commands
  app.commands.register(
    "mermaid:generate-diagram",
    handleGenerateDiagram,
    "[Mermaid] Generate Diagram",
  );
  // register context menu
  app.commands.register(
    "mermaid:example-class-diagram",
    handleSetClassDiagramExample,
    "[Mermaid] Set Class Diagram Example",
  );
  app.commands.register(
    "mermaid:example-sequence-diagram",
    handleSetSequenceDiagramExample,
    "[Mermaid] Set Sequence Diagram Example",
  );
  app.commands.register(
    "mermaid:example-state-diagram",
    handleSetStateDiagramExample,
    "[Mermaid] Set State Diagram Example",
  );
  app.commands.register(
    "mermaid:example-flowchart",
    handleSetFlowchartExample,
    "[Mermaid] Set Flowchart Example",
  );
  app.commands.register(
    "mermaid:example-er-diagram",
    handleSetErDiagramExample,
    "[Mermaid] Set ER Diagram Example",
  );
  app.commands.register(
    "mermaid:example-requirement-diagram",
    handleSetRequirementDiagramExample,
    "[Mermaid] Set Requirement Diagram Example",
  );
  app.commands.register(
    "mermaid:example-mindmap",
    handleSetMindmapExample,
    "[Mermaid] Set Mindmap Example",
  );
}

exports.init = init;
