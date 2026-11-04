#[path = "mdj_associations.rs"]
mod associations;
#[path = "mdj_erd_projection.rs"]
mod erd_projection;
#[path = "mdj_interface_projection.rs"]
mod interface_projection;
#[path = "mdj_label_projection.rs"]
mod label_projection;
#[path = "mdj_members.rs"]
mod members;
#[path = "mdj_native.rs"]
mod native;
#[path = "mdj_style_projection.rs"]
mod style_projection;
#[path = "mdj_view_projection.rs"]
mod view_projection;

use super::mdj::{self, Diagnostic, MdjError};
use crate::{
    Authored, Diagram, Element, Geometry, LayoutBounds, MdjObjectResidual, MdjProvenance,
    PresentationSource, Project, Relation, View, ViewPresentation,
};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const PROJECTION_VERSION: u32 = 4;

#[derive(Debug)]
pub struct Import {
    pub project: Project,
    pub diagnostics: Vec<Diagnostic>,
}

/// A conservative interchange projection, not a StarUML renderer.
/// Models, packages, classes, interfaces and a restricted binary association encoding map to UML.
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
    ensure_composition_unchanged(project, &original_import.project)?;
    if equivalent(project, &original_import.project, &residuals, &paths) {
        return Ok(provenance.source_bytes_for_verification().to_vec());
    }
    let original_index = mdj::Index::new(&original)?;
    let document = export_document(project, &original_index)?;
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
        authoring_knowledge: Default::default(),
        mdj_provenance: None,
        hierarchy: BTreeMap::new(),
        composition: BTreeMap::new(),
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
            diagnostics.push(Diagnostic { path: object.path.clone(), message: format!("{} presentation imported with its owned views; model semantics and diagram context are not inferred from canvas contents", object.type_name) });
        }
    }
    let association_ids = associations::import_associations(
        &index,
        &mut project,
        &mut residuals,
        &paths,
        &mut diagnostics,
    );
    for object in objects {
        if association_ids.contains(object.id) {
            continue;
        }
        if view_ids.contains(object.id)
            || (is_diagram(object.type_name) && !in_view_composition(&object.path))
        {
            continue;
        }
        let mut residue = extract_residue(object.properties);
        let mut mapped_fields = BTreeSet::new();
        let name = take_name(&mut residue, &mut mapped_fields);
        if object.id != project.id {
            let mapped_kind = canonical_kind_for_object(object, &index);
            if mapped_kind.is_some() {
                mapped_fields.insert("_type".into());
                if let Some(parent) = residue
                    .get("_parent")
                    .and_then(Value::as_object)
                    .filter(|reference| reference.len() == 1)
                    .and_then(|reference| reference.get("$ref"))
                    .and_then(Value::as_str)
                    .filter(|parent| {
                        index.object(parent).is_some_and(|parent| {
                            canonical_kind_for_object(parent, &index).is_some()
                        })
                    })
                {
                    project.relations.push(Relation {
                        id: ownership_relation_id(object.id, &paths),
                        kind: "owns".into(),
                        from: parent.into(),
                        to: object.id.into(),
                        properties: BTreeMap::new(),
                    });
                    residue.remove("_parent");
                    mapped_fields.insert("_parent".into());
                }
            }
            let description = if mapped_kind.is_some() {
                match residue.get("documentation").and_then(Value::as_str) {
                    Some(documentation) => {
                        let documentation = documentation.to_owned();
                        residue.remove("documentation");
                        mapped_fields.insert("documentation".into());
                        Some(documentation)
                    }
                    None => None,
                }
            } else {
                None
            };
            let properties = if mapped_kind.is_some() {
                members::import_properties(object.type_name, &mut residue, &mut mapped_fields)
            } else {
                std::mem::take(&mut residue)
            };
            project.elements.push(Element {
                id: object.id.into(),
                kind: mapped_kind
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("mdj.{}", object.type_name)),
                name,
                label: None,
                description,
                // Opaque MDJ properties must not accidentally become UML properties.
                properties,
                annotations: Vec::new(),
                implementations: Vec::new(),
                content: None,
            });
            diagnostics.push(Diagnostic {
                path: object.path.clone(),
                message: if mapped_kind.is_some() {
                    format!("{} mapped by identity, name, documentation, explicit ownership and representable properties; classifier members and operation parameters preserve source order, textual types and default values remain unresolved declarations, and source-only encoding stays in residue", object.type_name)
                } else if canonical_kind(object.type_name).is_some() {
                    let fields = object.properties.iter()
                        .filter(|(field, value)| !supported_semantic_field(object.type_name, field, value))
                        .map(|(field, _)| field.clone()).collect::<Vec<_>>().join(", ");
                    format!("{} kept opaque, not promoted to UML: unsupported fields [{}], an opaque/misplaced owned member, mismatched member slot and _parent, a non-string name, presentation containment or an unmappable ownership chain; source semantics must not be replaced with UML defaults", object.type_name, fields)
                } else {
                    format!("{} preserved as an MDJ object; this object has no supported canonical UML mapping", object.type_name)
                },
            });
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
            if let Geometry::Layout { bounds } = &view.geometry {
                for (field, extent) in [("width", &bounds.width), ("height", &bounds.height)] {
                    if let Authored::Value(crate::LayoutExtent::Unresolved { original, .. }) =
                        extent
                    {
                        diagnostics.push(Diagnostic {
                            path: format!("{}/{field}", paths.get(&view.id).map(String::as_str).unwrap_or_default()),
                            message: format!("authored negative extent {original} retained as an unresolved layout dimension; it is not a rendered size and requires layout resolution"),
                        });
                    }
                }
            }
            if let Some(source) = &view.presentation.source {
                for field in ["left", "top", "width", "height"] {
                    if source.residual.contains_key(field) {
                        diagnostics.push(Diagnostic {
                            path: format!("{}/{field}", paths.get(&view.id).map(String::as_str).unwrap_or_default()),
                            message: "layout value preserved as residue: unavailable in this geometry projection or not an exactly round-trippable finite number; native coordinate editing is unavailable for this field".into(),
                        });
                    }
                }
            }
            diagnostics.push(Diagnostic {
                path: paths.get(&view.id).cloned().unwrap_or_default(),
                message: "view/subview retained with subject, text, exact representable layout/path, attachments, supported authored style and style inheritance; computed appearance, unsupported style, route encodings, resources and other fields remain partial".into(),
            });
            style_projection::diagnose(
                view,
                paths.get(&view.id).map(String::as_str).unwrap_or_default(),
                &mut diagnostics,
            );
        }
    }
    capture_composition(&index, &mut project, &residuals, &mut diagnostics)?;
    label_projection::bind(&index, &mut project);
    interface_projection::bind(&index, &mut project);
    erd_projection::bind(&index, &mut project);
    view_projection::bind_markers(&index, &mut project, &mut diagnostics);
    native::normalize(&mut project, &mut diagnostics);
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
            let path = view_projection::take_path(&mut residual);
            let geometry = path.unwrap_or_else(|| Geometry::Layout {
                bounds: LayoutBounds {
                    x: take_number(&mut residual, "left"),
                    y: take_number(&mut residual, "top"),
                    width: take_extent(&mut residual, "width"),
                    height: take_extent(&mut residual, "height"),
                },
            });
            let endpoints = view_projection::take_endpoints(&mut residual);
            let style = style_projection::take_style(&mut residual);
            let default_style = Some(style_projection::default_style(&type_name, &residual));
            let mut field_presence = BTreeMap::new();
            let style_inheritance =
                style_projection::take_inheritance(&type_name, &mut residual, &mut field_presence);
            let children = read_views(object.get("subViews"), ids)?;
            Ok(View {
                id,
                subject,
                draft: None,
                endpoints,
                timing: None,
                label,
                geometry,
                children,
                presentation: ViewPresentation {
                    default_style,
                    style_inheritance,
                    kind: Some(type_name.clone()),
                    source: Some(PresentationSource {
                        format: "mdj".into(),
                        type_name,
                        field_presence,
                        residual,
                    }),
                    ..ViewPresentation::default()
                },
                style,
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

fn take_extent(residue: &mut BTreeMap<String, Value>, key: &str) -> Authored<crate::LayoutExtent> {
    match take_number(residue, key) {
        Authored::Missing => Authored::Missing,
        Authored::Null => Authored::Null,
        Authored::Value(value) => Authored::Value(if value < 0.0 {
            crate::LayoutExtent::Unresolved {
                original: value,
                reason: crate::UnresolvedExtentReason::NegativeExtent,
            }
        } else {
            crate::LayoutExtent::Resolved(value)
        }),
    }
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
    for diagram in &mut expected.diagrams {
        style_projection::update_inheritance_metadata(&mut diagram.views);
    }
    if let Some(provenance) = &mut expected.mdj_provenance {
        // These metadata fields describe the encoding of authored values,
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
                // Association ends share the UML Property kind with attributes,
                // but retain their distinct source role and metadata updater.
                if !associations::is_end(metadata) {
                    if let Some(kind) = mdj_kind(&element.kind) {
                        metadata.type_name = kind.into();
                    }
                    if metadata.mapped_fields.contains("_type") {
                        members::update_metadata(element, metadata);
                    }
                }
            }
        }
        associations::update_metadata(project, provenance);
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

fn export_document(project: &Project, original: &mdj::Index<'_>) -> Result<Value, MdjError> {
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
        if associations::is_end(metadata) {
            continue;
        }
        let kind = mdj_kind(&element.kind)
            .ok_or_else(|| failure("", "this canonical UML kind has no MDJ export mapping"))?;
        let canonical = metadata.mapped_fields.contains("_type");
        if canonical == element.kind.starts_with("mdj.") {
            return Err(failure(
                "",
                "changing between opaque MDJ and canonical UML objects is not supported",
            ));
        }
        let mut residual = if canonical {
            metadata.residual.clone()
        } else {
            element.properties.clone()
        };
        if canonical {
            if let Some(documentation) = &element.description {
                if residual
                    .insert("documentation".into(), Value::String(documentation.clone()))
                    .is_some()
                {
                    return Err(failure(
                        "",
                        "mapped documentation conflicts with source residue",
                    ));
                }
            }
            for (field, value) in &element.properties {
                if !supported_semantic_property(kind, field, value) {
                    return Err(failure(
                        "",
                        "canonical UML property has no lossless MDJ mapping",
                    ));
                }
                if !metadata.mapped_fields.contains(field)
                    && members::preserves_omitted_default(kind, field, value)
                {
                    continue;
                }
                if residual.insert(field.clone(), value.clone()).is_some() {
                    return Err(failure(
                        "",
                        "mapped semantic property conflicts with source residue",
                    ));
                }
            }
        }
        if canonical && metadata.mapped_fields.contains("_parent") {
            let relation_id = ownership_relation_id(&element.id, &provenance.object_paths);
            let ownership = project
                .relations
                .iter()
                .find(|relation| relation.id == relation_id)
                .ok_or_else(|| failure("", "mapped MDJ ownership is missing"))?;
            if ownership.kind != "owns"
                || ownership.to != element.id
                || !project.elements.iter().any(|parent| {
                    parent.id == ownership.from
                        && matches!(
                            parent.kind.as_str(),
                            "uml.model"
                                | "uml.package"
                                | "uml.class"
                                | "uml.interface"
                                | "uml.operation"
                        )
                })
            {
                return Err(failure(
                    "",
                    "mapped MDJ ownership cannot reference an unsupported or missing parent",
                ));
            }
            if residual
                .insert(
                    "_parent".into(),
                    serde_json::json!({"$ref": ownership.from}),
                )
                .is_some()
            {
                return Err(failure(
                    "",
                    "mapped ownership conflicts with source residue",
                ));
            }
        }
        objects.insert(
            element.id.clone(),
            named_object(
                &element.id,
                kind,
                &element.name,
                &residual,
                &metadata.mapped_fields,
            )?,
        );
    }
    associations::export_associations(project, &mut objects)?;
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
            objects.insert(
                view.id.clone(),
                export_view(view, original.object(&view.id))?,
            );
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
    // Current authored composition, not historical paths, locates model objects.
    // Views retain their existing nested representation in this first slice.
    let mut export_paths = provenance.object_paths.clone();
    export_paths.extend(crate::composition_paths(project).map_err(|message| failure("", message))?);
    let mut entries: Vec<_> = export_paths.iter().collect();
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

fn export_view(view: &View, original: Option<&mdj::Object<'_>>) -> Result<Value, MdjError> {
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
    view_projection::export_endpoints(view, &mut object)?;
    style_projection::export_style(view, &mut object)?;
    interface_projection::export(view, &mut object)?;
    erd_projection::export(view, &mut object)?;
    if let Some(visible) = view.presentation.visible
        && !source.residual.contains_key("visible")
    {
        insert_mapped(&mut object, "visible", Value::Bool(visible))?;
    }
    let original = original.filter(|original| original.type_name == source.type_name);
    if view_projection::export_path(view, original, &mut object)? {
        return Ok(Value::Object(object));
    }
    if let Geometry::Rect {
        x,
        y,
        width,
        height,
    } = &view.geometry
    {
        for (key, value) in [
            ("left", x),
            ("top", y),
            ("width", width),
            ("height", height),
        ] {
            if !value.is_finite() || matches!(key, "width" | "height") && *value < 0.0 {
                return Err(failure(&view.id, "invalid rectangular MDJ geometry"));
            }
            insert_mapped(&mut object, key, serde_json::from_str(&value.to_string())?)?;
        }
        return Ok(Value::Object(object));
    }
    let Geometry::Layout { bounds } = &view.geometry else {
        return Err(failure(
            "",
            "MDJ geometry export requires rectangular or unresolved Layout bounds",
        ));
    };
    for (key, number) in [("left", &bounds.x), ("top", &bounds.y)] {
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
    for (key, extent) in [("width", &bounds.width), ("height", &bounds.height)] {
        match extent {
            Authored::Missing => {}
            Authored::Null => insert_mapped(&mut object, key, Value::Null)?,
            Authored::Value(extent) => {
                if !extent.is_valid() {
                    return Err(failure("", "invalid authored MDJ extent"));
                }
                insert_mapped(
                    &mut object,
                    key,
                    serde_json::from_str(&extent.original().to_string())?,
                )?;
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

fn canonical_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "UMLModel" => Some("uml.model"),
        "UMLPackage" => Some("uml.package"),
        "UMLClass" => Some("uml.class"),
        "UMLInterface" => Some("uml.interface"),
        "UMLAttribute" => Some("uml.attribute"),
        "UMLOperation" => Some("uml.operation"),
        "UMLParameter" => Some("uml.parameter"),
        _ => None,
    }
}

fn supported_semantic_field(kind: &str, field: &str, value: &Value) -> bool {
    matches!(field, "_id" | "_type" | "name" | "_parent")
        || (field == "ownedElements" && value.is_array())
        || members::supported_field(kind, field, value)
}

fn supported_semantic_property(kind: &str, field: &str, value: &Value) -> bool {
    members::supported_property(kind, field, value)
}

fn canonical_kind_for_object<'document>(
    object: &mdj::Object<'document>,
    index: &mdj::Index<'document>,
) -> Option<&'static str> {
    let kind = canonical_kind(object.type_name)?;
    let mut current = object;
    let mut visited = BTreeSet::new();
    loop {
        // A partially promoted chain would hide known ownership from the kernel.
        // Unsupported source semantics must not become canonical UML defaults.
        if !visited.insert(current.id)
            || in_view_composition(&current.path)
            || canonical_kind(current.type_name).is_none()
            || current
                .properties
                .get("name")
                .is_some_and(|name| !name.is_string())
            || current
                .properties
                .iter()
                .any(|(field, value)| !supported_semantic_field(current.type_name, field, value))
            || !members::supported_structure(current, index)
        {
            return None;
        }
        let Some(parent) = current.properties.get("_parent") else {
            return Some(kind);
        };
        let reference = parent
            .as_object()
            .filter(|reference| reference.len() == 1)?;
        let parent_id = reference.get("$ref")?.as_str()?;
        let parent = index.object(parent_id)?;
        // The MDJ Project is an interchange envelope, not a UML owner.
        if parent.path.is_empty() && parent.type_name == "Project" {
            return Some(kind);
        }
        current = parent;
    }
}

fn mdj_kind(kind: &str) -> Option<&str> {
    match kind {
        "uml.model" => Some("UMLModel"),
        "uml.package" => Some("UMLPackage"),
        "uml.class" => Some("UMLClass"),
        "uml.interface" => Some("UMLInterface"),
        "uml.attribute" => Some("UMLAttribute"),
        "uml.operation" => Some("UMLOperation"),
        "uml.parameter" => Some("UMLParameter"),
        _ => kind.strip_prefix("mdj."),
    }
}

fn ownership_relation_id(child: &str, paths: &Paths) -> String {
    // Derived contract edges are not MDJ objects and never receive composition slots.
    let mut id = format!("mdj.owns:{child}");
    while paths.contains_key(&id) {
        id.insert(0, ':');
    }
    id
}

fn in_view_composition(path: &str) -> bool {
    let mut parts = path.rsplit('/');
    let Some(index) = parts.next() else {
        return false;
    };
    index.parse::<usize>().is_ok() && matches!(parts.next(), Some("ownedViews" | "subViews"))
}

fn is_diagram(kind: &str) -> bool {
    // ERD is a separate StarUML language. Its diagram still owns presentation
    // Views; retaining it as a model element would silently leave the canvas empty.
    kind == "ERDDiagram" || (kind.starts_with("UML") && kind.ends_with("Diagram"))
}

fn diagram_type(kind: &str) -> &'static str {
    match kind {
        "ERDDiagram" => "ERDDiagram",
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
        "UMLProfileDiagram" => "UMLProfileDiagram",
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

/// Capture non-view document composition independently of semantic type mapping.
/// View composition remains authored by Diagram.views / View.children in this slice.
fn capture_composition(
    index: &mdj::Index<'_>,
    project: &mut Project,
    residuals: &Residuals,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), MdjError> {
    let objects = index.objects().collect::<Vec<_>>();
    let paths: Paths = objects
        .iter()
        .map(|object| (object.id.to_owned(), object.path.clone()))
        .collect();
    let view_ids = project
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .map(|view| view.id.as_str())
        .collect::<BTreeSet<_>>();
    for object in &objects {
        if object.id == project.id || view_ids.contains(object.id) {
            continue;
        }
        let Some(parent) = objects
            .iter()
            .copied()
            .filter(|parent| {
                parent.path.len() < object.path.len()
                    && object.path.starts_with(&format!("{}/", parent.path))
            })
            .max_by_key(|parent| parent.path.len())
        else {
            diagnostics.push(Diagnostic {
                path: object.path.clone(),
                message: "document composition has no identified container; no parent inferred"
                    .into(),
            });
            continue;
        };
        if view_ids.contains(parent.id)
            || objects.iter().any(|ancestor| {
                view_ids.contains(ancestor.id)
                    && object.path.starts_with(&format!("{}/", ancestor.path))
            })
        {
            diagnostics.push(Diagnostic { path: object.path.clone(), message: "non-view object embedded in view composition remains preserved; authoring-tree placement is not projected in this slice".into() });
            continue;
        }
        let relative = &object.path[parent.path.len()..];
        let (slot, order) = relative
            .rsplit_once('/')
            .and_then(|(slot, index)| {
                let parent_value = Value::Object(parent.properties.clone());
                parent_value
                    .pointer(slot)
                    .filter(|value| value.is_array())?;
                index
                    .parse::<usize>()
                    .ok()
                    .map(|index| (slot.to_owned(), Some(index)))
            })
            .unwrap_or_else(|| (relative.to_owned(), None));
        let explicit_parent = object
            .properties
            .get("_parent")
            .and_then(Value::as_object)
            .filter(|value| value.len() == 1)
            .and_then(|value| value.get("$ref"))
            .and_then(Value::as_str);
        if explicit_parent.is_some_and(|reference| reference != parent.id) {
            diagnostics.push(Diagnostic { path: object.path.clone(), message: "explicit _parent differs from document composition; both values are preserved, Explorer follows composition".into() });
        }
        // Only the adapter's mapped edge for this source _parent may supply the
        // parent. A coincidentally matching unrelated relationship is not evidence.
        let mapped_parent = residuals
            .get(object.id)
            .is_some_and(|metadata| metadata.mapped_fields.contains("_parent"));
        let ownership_id = ownership_relation_id(object.id, &paths);
        let ownership = project.relations.iter().find(|relation| {
            relation.id == ownership_id
                && relation.kind == "owns"
                && relation.to == object.id
                && relation.from == parent.id
        });
        let parent = if mapped_parent && explicit_parent == Some(parent.id) {
            ownership
                .map(|relation| crate::CompositionParent::SemanticOwner {
                    relation_id: relation.id.clone(),
                })
                .unwrap_or_else(|| crate::CompositionParent::Object {
                    id: parent.id.to_owned(),
                })
        } else {
            crate::CompositionParent::Object {
                id: parent.id.to_owned(),
            }
        };
        project.composition.insert(
            object.id.to_owned(),
            crate::CompositionPlacement {
                parent,
                slot,
                order,
            },
        );
    }
    Ok(())
}

fn ensure_composition_unchanged(current: &Project, original: &Project) -> Result<(), MdjError> {
    if current.composition != original.composition
        || current.composition.iter().any(|(id, placement)| {
            placement.parent_id(id, &current.relations)
                != original
                    .composition
                    .get(id)
                    .and_then(|placement| placement.parent_id(id, &original.relations))
        })
    {
        return Err(failure(
            "",
            "document composition changes are not yet supported by MDJ export; parent, slot and order must not be discarded",
        ));
    }
    Ok(())
}
