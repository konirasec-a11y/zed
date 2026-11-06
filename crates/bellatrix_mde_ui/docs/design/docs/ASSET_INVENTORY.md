# Supplied reference files

The user supplied a StarUML screenshot and two ZIP archives.

| Source | Contents inspected | How the prototype uses it |
| --- | --- | --- |
| `image.png` | 1369 × 1088 desktop screenshot of StarUML | Reference for 3-pane layout, working diagrams/toolbox, canvas and model inspector |
| `static.zip` | 947 entries including HTML partials for `main-view`, `toolbox-view`, `model-explorer-view`, `style-editor-view`, properties and theme CSS | Structural reference; original HTML/JS is not embedded |
| `resources.zip` | 1892 entries, large AWS/Azure/GCP vector icon catalog, Inter and other fonts, metamodel, default menu preferences | Geometry vocabulary and future asset sources; three cloud SVG examples copied into `public/asset-samples` |

The three included SVG asset samples are from the user-provided resources:

- `aws-database.svg`
- `azure-app-services.svg`
- `gcp-cloud-run.svg`

The output intentionally omits the other 1,800+ cloud icon files and all fonts. This keeps the design prototype small and avoids shipping a huge cloud library that has not been incorporated into the UI architecture. Re-introduce families through a dedicated icon registry in the actual GPUI app, with source and license reviewed for each icon collection.
