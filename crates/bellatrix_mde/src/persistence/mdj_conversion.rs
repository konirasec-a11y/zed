use super::mdj::{self, Diagnostic, MdjError};
use crate::{
    Authored, Diagram, Element, Geometry, LayoutBounds, MdjObjectResidual, MdjProvenance,
    PresentationSource, Project, Style, View, ViewPresentation,
};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const PROJECTION_VERSION: u32 = 1;

#[derive(Debug)]
pub struct Import {
    pub project: Project,
    pub diagnostics: Vec<Diagnostic>,
}

/// A conservative interchange projection, not a StarUML renderer or UML adapter.
/// Semantic objects retain their MDJ namespace until a canonical UML adapter exists.
pub fn import(bytes: &[u8]) -> Result<Import, MdjError> {
    let document = mdj::decode(bytes)?;
    let (mut imported, residuals, paths) = project_document(&document)?;
    let projection = project_projection(&imported.project, &residuals, &paths)?;
    imported.project.mdj_provenance = Some(MdjProvenance::capture(
        bytes.to_vec(),
        PROJECTION_VERSION,
        &projection,
        paths,
        residuals,
    )?);
    Ok(imported)
}

/// Unsupported edits return an error rather than silently exporting stale source.
pub fn export(project: &Project) -> Result<Vec<u8>, MdjError> {
    let provenance = project.mdj_provenance.as_ref().ok_or_else(|| {
        failure(
            "",
            "MDJ export currently requires an imported document's provenance",
        )
    })?;
    let original = mdj::decode(provenance.source_bytes_for_verification())?;
    let (original_import, residuals, paths) = project_document(&original)?;
    if equivalent(project, &original_import.project, &residuals, &paths) {
        return Ok(provenance.source_bytes_for_verification().to_vec());
    }
    let document = export_document(project)?;
    // Reimport the actual result, not a digest supplied by the persisted source.
    // This also rejects edits to contract fields with no inverse MDJ mapping.
    let (reimported, residuals, paths) = project_document(&document)?;
    let expected = metadata_after_export(project);
    if !equivalent(&expected, &reimported.project, &residuals, &paths) {
        return Err(failure(
            "",
            "MDJ export cannot represent this edit losslessly; unsupported contract or containment changes must not be discarded",
        ));
    }
    Ok(serde_json::to_vec_pretty(&document)?)
}

type Residuals = BTreeMap<String, MdjObjectResidual>;
type Paths = BTreeMap<String, String>;

fn project_document(document: &Value) -> Result<(Import, Residuals, Paths), MdjError> {
    let index = mdj::Index::new(document)?;
    let root = document
        .as_object()
        .ok_or_else(|| failure("", "expected object"))?;
    let mut project = Project {
        schema_version: 1,
        id: string(root, "_id")?.into(),
        name: root
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        root_model: None,
        mdj_provenance: None,
        hierarchy: BTreeMap::new(),
        elements: Vec::new(),
        relations: Vec::new(),
        diagrams: Vec::new(),
    };
    let mut diagnostics = index.diagnostics().to_vec();
    diagnostics.extend(index.unresolved_references().map(|reference| Diagnostic {
        path: reference.path.clone(),
        message: format!("unresolved reference preserved: {}", reference.target_id),
    }));
    let paths: Paths = index
        .objects()
        .map(|object| (object.id.into(), object.path.clone()))
        .collect();
    let mut objects: Vec<_> = index.objects().collect();
    objects.sort_by(|left, right| path_order(&left.path).cmp(&path_order(&right.path)));
    let mut residuals = Residuals::new();
    let mut view_ids = BTreeSet::new();
    for object in &objects {
        if is_diagram(object.type_name) && !in_view_composition(&object.path) {
            let mut residue = extract_residue(object.properties);
            let mut mapped_fields = BTreeSet::new();
            let name = take_name(&mut residue, &mut mapped_fields);
            let views = read_views(object.properties.get("ownedViews"), &mut view_ids)?;
            project.diagrams.push(Diagram {
                id: object.id.into(),
                name,
                width: 1280.0,
                height: 720.0,
                views,
                uml_di_type: diagram_type(object.type_name).into(),
                interaction_kind: None,
                model_elements: Vec::new(),
            });
            residuals.insert(
                object.id.into(),
                MdjObjectResidual {
                    type_name: object.type_name.into(),
                    residual: residue,
                    mapped_fields,
                },
            );
            diagnostics.push(Diagnostic { path: object.path.clone(), message: "diagram imported conservatively; canvas equivalence and UML context mapping are not implemented".into() });
        }
    }
    for object in objects {
        if view_ids.contains(object.id)
            || (is_diagram(object.type_name) && !in_view_composition(&object.path))
        {
            continue;
        }
        let mut residue = extract_residue(object.properties);
        let mut mapped_fields = BTreeSet::new();
        let name = take_name(&mut residue, &mut mapped_fields);
        if object.id != project.id {
            // MDJ names are not assumed to be canonical UML metaclasses.
            project.elements.push(Element {
                id: object.id.into(),
                kind: format!("mdj.{}", object.type_name),
                name,
                label: None,
                description: None,
                properties: residue,
                annotations: Vec::new(),
                implementations: Vec::new(),
                content: None,
            });
            residue = BTreeMap::new();
            diagnostics.push(Diagnostic { path: object.path.clone(), message: format!("{} preserved as an MDJ object; canonical UML mapping (including associations and ends) is not implemented", object.type_name) });
        }
        residuals.insert(
            object.id.into(),
            MdjObjectResidual {
                type_name: object.type_name.into(),
                residual: residue,
                mapped_fields,
            },
        );
    }
    for diagram in &project.diagrams {
        for view in diagram.all_views() {
            if let Some(source) = &view.presentation.source {
                for field in ["left", "top", "width", "height"] {
                    if source.residual.contains_key(field) {
                        diagnostics.push(Diagnostic {
                            path: format!("{}/{field}", paths.get(&view.id).map(String::as_str).unwrap_or_default()),
                            message: "layout value preserved as residue: not an exactly round-trippable finite number; native coordinate editing is unavailable for this field".into(),
                        });
                    }
                }
            }
            diagnostics.push(Diagnostic {
                path: paths.get(&view.id).cloned().unwrap_or_default(),
                message: "view/subview retained with subject, text and exact representable layout; style, routing, resources and other fields remain source residue, not native rendering support".into(),
            });
        }
    }
    Ok((
        Import {
            project,
            diagnostics,
        },
        residuals,
        paths,
    ))
}

fn read_views(value: Option<&Value>, ids: &mut BTreeSet<String>) -> Result<Vec<View>, MdjError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| failure("", "expected view array"))?;
    values
        .iter()
        .enumerate()
        .map(|(position, value)| {
            let object = value
                .as_object()
                .ok_or_else(|| failure("", "expected view object"))?;
            let id = string(object, "_id")?.to_owned();
            ids.insert(id.clone());
            let type_name = string(object, "_type")?.to_owned();
            let mut residual = extract_residue(object);
            let subject = take_reference(&mut residual, "model");
            let label = match residual.get("text").and_then(Value::as_str) {
                Some(text) => {
                    let text = text.to_owned();
                    residual.remove("text");
                    Some(text)
                }
                None => None,
            };
            let geometry = Geometry::Layout {
                bounds: LayoutBounds {
                    x: take_number(&mut residual, "left"),
                    y: take_number(&mut residual, "top"),
                    width: take_number(&mut residual, "width"),
                    height: take_number(&mut residual, "height"),
                },
            };
            let children = read_views(object.get("subViews"), ids)?;
            Ok(View {
                id,
                subject,
                draft: None,
                endpoints: None,
                timing: None,
                label,
                geometry,
                children,
                presentation: ViewPresentation {
                    kind: Some(type_name.clone()),
                    source: Some(PresentationSource {
                        format: "mdj".into(),
                        type_name,
                        residual,
                    }),
                    ..ViewPresentation::default()
                },
                style: Style::default(),
                z_index: i32::try_from(position).map_err(|_| failure("", "too many views"))?,
            })
        })
        .collect()
}

fn extract_residue(object: &Map<String, Value>) -> BTreeMap<String, Value> {
    object
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "_id" | "_type"))
        .map(|(key, value)| (key.clone(), placeholders(value.clone())))
        .collect()
}

// Composition slots retain only shape/order; the identified child is authored once.
fn placeholders(value: Value) -> Value {
    match value {
        Value::Object(object)
            if object
                .get("_id")
                .and_then(Value::as_str)
                .is_some_and(|id| !id.is_empty())
                && object
                    .get("_type")
                    .and_then(Value::as_str)
                    .is_some_and(|kind| !kind.is_empty()) =>
        {
            Value::Null
        }
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, placeholders(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(placeholders).collect()),
        value => value,
    }
}

fn take_name(residue: &mut BTreeMap<String, Value>, fields: &mut BTreeSet<String>) -> String {
    match residue.get("name").and_then(Value::as_str) {
        Some(name) => {
            let name = name.to_owned();
            residue.remove("name");
            fields.insert("name".into());
            name
        }
        None => String::new(),
    }
}

fn take_reference(residue: &mut BTreeMap<String, Value>, key: &str) -> Option<String> {
    let value = residue.get(key)?.as_object()?;
    if value.len() != 1 {
        return None;
    }
    let target = value.get("$ref")?.as_str()?.to_owned();
    residue.remove(key);
    Some(target)
}

fn take_number(residue: &mut BTreeMap<String, Value>, key: &str) -> Authored<f64> {
    match residue.get(key) {
        Some(Value::Null) => {
            residue.remove(key);
            Authored::Null
        }
        Some(Value::Number(number)) => {
            // Do not round arbitrary-precision source values into editable f64s.
            let Some(value) = number.as_f64().filter(|value| value.is_finite()) else {
                return Authored::Missing;
            };
            if decimal_key(&number.to_string()) != decimal_key(&value.to_string()) {
                return Authored::Missing;
            }
            residue.remove(key);
            Authored::Value(value)
        }
        _ => Authored::Missing,
    }
}

// Compare decimal values, not spelling: 1.0 and 1e0 are editable without
// accepting a longer decimal that would round to the same f64.
fn decimal_key(number: &str) -> Option<(bool, String, i64)> {
    let (negative, number) = match number.strip_prefix('-') {
        Some(number) => (true, number),
        None => (false, number),
    };
    let (mantissa, exponent) = match number.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i64>().ok()?),
        None => (number, 0),
    };
    let fraction_length = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    let digits: String = mantissa
        .chars()
        .filter(|character| *character != '.')
        .collect();
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Some((false, "0".into(), 0));
    }
    let trimmed = digits.trim_end_matches('0');
    let exponent = exponent
        .checked_sub(i64::try_from(fraction_length).ok()?)?
        .checked_add(i64::try_from(digits.len() - trimmed.len()).ok()?)?;
    Some((negative, trimmed.into(), exponent))
}

fn project_projection(
    project: &Project,
    residuals: &Residuals,
    paths: &Paths,
) -> Result<Value, MdjError> {
    let mut project = project.clone();
    project.mdj_provenance = None;
    Ok(
        serde_json::json!({ "project": serde_json::to_value(project)?, "residuals": residuals, "paths": paths }),
    )
}

fn equivalent(current: &Project, imported: &Project, residuals: &Residuals, paths: &Paths) -> bool {
    let Some(provenance) = &current.mdj_provenance else {
        return false;
    };
    if &provenance.object_residuals != residuals || &provenance.object_paths != paths {
        return false;
    }
    let mut current = current.clone();
    current.mdj_provenance = None;
    // Typed equality prevents serializer coercions (for example NaN to null)
    // from making unsupported edits look unchanged.
    if current != *imported {
        return false;
    }
    // f64 equality intentionally equates signed zero; JSON's numeric spelling
    // additionally distinguishes that authored change without trusting coercions.
    match (
        serde_json::to_value(&current),
        serde_json::to_value(imported),
    ) {
        (Ok(current), Ok(imported)) => current == imported,
        _ => false,
    }
}

fn metadata_after_export(project: &Project) -> Project {
    let mut expected = project.clone();
    if let Some(provenance) = &mut expected.mdj_provenance {
        // These two metadata fields describe the encoding of authored name/kind,
        // not a competing value. Only adjust them after constructing actual output;
        // raw replay still requires exact metadata equality with reimported source.
        if !project.name.is_empty() {
            if let Some(metadata) = provenance.object_residuals.get_mut(&project.id) {
                metadata.mapped_fields.insert("name".into());
            }
        }
        for element in &project.elements {
            if let Some(metadata) = provenance.object_residuals.get_mut(&element.id) {
                if !element.name.is_empty() {
                    metadata.mapped_fields.insert("name".into());
                }
                if let Some(kind) = element.kind.strip_prefix("mdj.") {
                    metadata.type_name = kind.into();
                }
            }
        }
        for diagram in &project.diagrams {
            if !diagram.name.is_empty() {
                if let Some(metadata) = provenance.object_residuals.get_mut(&diagram.id) {
                    metadata.mapped_fields.insert("name".into());
                }
            }
        }
    }
    expected
}

fn export_document(project: &Project) -> Result<Value, MdjError> {
    let provenance = project
        .mdj_provenance
        .as_ref()
        .ok_or_else(|| failure("", "missing provenance"))?;
    let mut objects = BTreeMap::<String, Value>::new();
    let root = provenance
        .object_residuals
        .get(&project.id)
        .ok_or_else(|| failure("", "missing project metadata"))?;
    objects.insert(
        project.id.clone(),
        named_object(
            &project.id,
            "Project",
            &project.name,
            &root.residual,
            &root.mapped_fields,
        )?,
    );
    for element in &project.elements {
        let metadata = provenance
            .object_residuals
            .get(&element.id)
            .ok_or_else(|| {
                failure(
                    "",
                    "new semantic objects are not supported by the MDJ adapter",
                )
            })?;
        let kind = element
            .kind
            .strip_prefix("mdj.")
            .ok_or_else(|| failure("", "canonical UML export is not implemented"))?;
        objects.insert(
            element.id.clone(),
            named_object(
                &element.id,
                kind,
                &element.name,
                &element.properties,
                &metadata.mapped_fields,
            )?,
        );
    }
    for diagram in &project.diagrams {
        let metadata = provenance
            .object_residuals
            .get(&diagram.id)
            .ok_or_else(|| failure("", "new diagrams are not supported by the MDJ adapter"))?;
        objects.insert(
            diagram.id.clone(),
            named_object(
                &diagram.id,
                &metadata.type_name,
                &diagram.name,
                &metadata.residual,
                &metadata.mapped_fields,
            )?,
        );
        for view in diagram.all_views() {
            objects.insert(view.id.clone(), export_view(view)?);
        }
    }
    if objects.len() != provenance.object_paths.len()
        || objects
            .keys()
            .any(|id| !provenance.object_paths.contains_key(id))
    {
        return Err(failure(
            "",
            "adding, removing or duplicating MDJ objects is not yet supported",
        ));
    }
    let mut entries: Vec<_> = provenance.object_paths.iter().collect();
    entries.sort_by_key(|(_, path)| path.matches('/').count());
    let mut document = Value::Null;
    for (id, path) in entries {
        let object = objects
            .remove(id)
            .ok_or_else(|| failure(path, "missing authored object"))?;
        if path.is_empty() {
            if !document.is_null() {
                return Err(failure(path, "multiple roots"));
            }
            document = object;
        } else {
            let slot = document
                .pointer_mut(path)
                .ok_or_else(|| failure(path, "composition slot no longer exists"))?;
            if !slot.is_null() {
                return Err(failure(
                    path,
                    "composition slot conflicts with authored data",
                ));
            }
            *slot = object;
        }
    }
    mdj::Index::new(&document)?;
    Ok(document)
}

fn named_object(
    id: &str,
    kind: &str,
    name: &str,
    residual: &BTreeMap<String, Value>,
    fields: &BTreeSet<String>,
) -> Result<Value, MdjError> {
    let mut object = base_object(id, kind, residual)?;
    if fields.contains("name") || !name.is_empty() {
        insert_mapped(&mut object, "name", Value::String(name.into()))?;
    }
    Ok(Value::Object(object))
}

fn base_object(
    id: &str,
    kind: &str,
    residual: &BTreeMap<String, Value>,
) -> Result<Map<String, Value>, MdjError> {
    let mut object: Map<_, _> = residual.clone().into_iter().collect();
    insert_mapped(&mut object, "_id", Value::String(id.into()))?;
    insert_mapped(&mut object, "_type", Value::String(kind.into()))?;
    Ok(object)
}

fn export_view(view: &View) -> Result<Value, MdjError> {
    let source = view
        .presentation
        .source
        .as_ref()
        .filter(|source| source.format == "mdj")
        .ok_or_else(|| failure("", "view has no MDJ source mapping"))?;
    let mut object = base_object(&view.id, &source.type_name, &source.residual)?;
    if let Some(subject) = &view.subject {
        insert_mapped(&mut object, "model", serde_json::json!({"$ref": subject}))?;
    }
    if let Some(label) = &view.label {
        insert_mapped(&mut object, "text", Value::String(label.clone()))?;
    }
    let Geometry::Layout { bounds } = &view.geometry else {
        return Err(failure(
            "",
            "MDJ geometry export only supports authored Layout bounds",
        ));
    };
    for (key, number) in [
        ("left", &bounds.x),
        ("top", &bounds.y),
        ("width", &bounds.width),
        ("height", &bounds.height),
    ] {
        match number {
            Authored::Missing => {}
            Authored::Null => insert_mapped(&mut object, key, Value::Null)?,
            Authored::Value(number) => {
                if !number.is_finite() {
                    return Err(failure("", "non-finite MDJ coordinate"));
                }
                // Use the shortest exact decimal representation consistently with import.
                insert_mapped(&mut object, key, serde_json::from_str(&number.to_string())?)?;
            }
        }
    }
    Ok(Value::Object(object))
}

fn insert_mapped(object: &mut Map<String, Value>, key: &str, value: Value) -> Result<(), MdjError> {
    if object.contains_key(key) {
        return Err(failure(
            "",
            format!("mapped field {key} conflicts with source residue"),
        ));
    }
    object.insert(key.into(), value);
    Ok(())
}

fn string<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, MdjError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| failure(key, "expected string"))
}

fn in_view_composition(path: &str) -> bool {
    let mut parts = path.rsplit('/');
    let Some(index) = parts.next() else {
        return false;
    };
    index.parse::<usize>().is_ok() && matches!(parts.next(), Some("ownedViews" | "subViews"))
}

fn is_diagram(kind: &str) -> bool {
    kind.starts_with("UML") && kind.ends_with("Diagram")
}

fn diagram_type(kind: &str) -> &'static str {
    match kind {
        "UMLClassDiagram" => "UMLClassDiagram",
        "UMLUseCaseDiagram" => "UMLUseCaseDiagram",
        "UMLActivityDiagram" => "UMLActivityDiagram",
        "UMLStatechartDiagram" => "UMLStateMachineDiagram",
        "UMLSequenceDiagram" | "UMLCommunicationDiagram" | "UMLTimingDiagram" => {
            "UMLInteractionDiagram"
        }
        "UMLComponentDiagram" => "UMLComponentDiagram",
        "UMLDeploymentDiagram" => "UMLDeploymentDiagram",
        "UMLPackageDiagram" => "UMLPackageDiagram",
        _ => "MDJUnsupportedDiagram",
    }
}

fn path_order(path: &str) -> Vec<(u8, usize, String)> {
    path.split('/')
        .map(|part| match part.parse::<usize>() {
            Ok(index) => (0, index, String::new()),
            Err(_) => (1, 0, part.to_owned()),
        })
        .collect()
}

fn failure(path: &str, message: impl Into<String>) -> MdjError {
    MdjError::Structure {
        path: path.into(),
        message: message.into(),
    }
}
