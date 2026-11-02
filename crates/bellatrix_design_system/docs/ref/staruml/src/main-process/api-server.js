const express = require("express");
const { z } = require("zod");

const generateDiagramSchema = z.object({
  code: z.string(), // Mermaid code string
});

const getDiagramImageSchema = z.object({
  diagramId: z.string(), // ID of the diagram to retrieve
});

function getCurrentWindow() {
  const wins = global.application.windows || [];
  if (wins.length > 0) {
    return wins[wins.length - 1];
  } else {
    throw new Error("No windows found");
  }
}

function startApiServer(port) {
  const app = express();

  app.use(express.json());
  app.use(express.urlencoded({ extended: true }));

  app.get("/", (req, res) => {
    res.send("Hello from StarUML API Server!");
  });

  /**
   * POST /generate_diagram
   */
  app.post("/generate_diagram", async (req, res) => {
    const window = getCurrentWindow();
    if (window) {
      const parseResult = generateDiagramSchema.safeParse(req.body);
      if (!parseResult.success) {
        res.status(400).json({
          success: false,
          error: "Invalid request body",
          details: parseResult.error.flatten(),
        });
      }
      const params = parseResult.data;
      try {
        const code = params.code;
        await window.sendCommandAsync("mermaid:generate-diagram", code);
        res.json({
          success: true,
        });
      } catch (error) {
        res.status(500).json({
          success: false,
          error: error.toString(),
        });
      }
    } else {
      res.status(500).json({ success: false, error: "No windows found" });
    }
  });

  /**
   * POST /get_all_diagrams_info
   */
  app.post("/get_all_diagrams_info", async (req, res) => {
    const window = getCurrentWindow();
    if (window) {
      try {
        const allDiagramInfo = await window.sendCommandAsync(
          "api:get_all_diagrams_info",
        );
        res.json({
          success: true,
          data: allDiagramInfo,
        });
      } catch (error) {
        res.status(500).json({
          success: false,
          error: error.toString(),
        });
      }
    } else {
      res.status(500).json({ success: false, error: "No windows found" });
    }
  });

  /**
   * POST /get_current_diagram_info
   */
  app.post("/get_current_diagram_info", async (req, res) => {
    const window = getCurrentWindow();
    if (window) {
      try {
        const currentDiagram = await window.sendCommandAsync(
          "api:get_current_diagram_info",
        );
        res.json({
          success: true,
          data: currentDiagram,
        });
      } catch (error) {
        res.status(500).json({
          success: false,
          error: error.toString(),
        });
      }
    } else {
      res.status(500).json({ success: false, error: "No windows found" });
    }
  });

  /**
   * POST /get_diagram_image_by_id
   */
  app.post("/get_diagram_image_by_id", async (req, res) => {
    const window = getCurrentWindow();
    if (window) {
      const parseResult = getDiagramImageSchema.safeParse(req.body);
      if (!parseResult.success) {
        res.status(400).json({
          success: false,
          error: "Invalid request body",
          details: parseResult.error.flatten(),
        });
      }
      const params = parseResult.data;
      try {
        const diagramImage = await window.sendCommandAsync(
          "api:get_diagram_image_by_id",
          params.diagramId,
        );
        res.json({
          success: true,
          data: diagramImage,
        });
      } catch (error) {
        res.status(500).json({
          success: false,
          error: error.toString(),
        });
      }
    } else {
      res.status(500).json({ success: false, error: "No windows found" });
    }
  });

  app.listen(port, (err) => {
    if (err) {
      console.error(`Failed to start API server on port ${port}:`, err);
      return;
    }
    console.log(`API server running on port ${port}`);
  });
}

exports.startApiServer = startApiServer;
