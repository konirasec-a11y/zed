#!/usr/bin/env python3
"""Compile pinned OMG XMI structural contracts; handwritten Rust evaluates supported semantics."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ElementTree


ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path("docs/bellatrix/standards/uml/20161101/UML.xmi")
TARGET = Path("crates/bellatrix_mde/src/KERNEL/uml/metamodel.rs")
SOURCE_URL = "https://www.omg.org/spec/UML/20161101/UML.xmi"
XMI = "{http://www.omg.org/spec/XMI/20131001}"

RUST = '''
/// Whether a metaclass declares or inherits a named UML property.
/// This is schema availability, not an evaluation of UML constraints or OCL.
pub fn has_property(metaclass: &str, property: &str) -> bool {
    super::schema::uml_property(metaclass, property).is_ok_and(|property| property.is_some())
}

/// Tests declared metaclass identity, not supported editor behavior.
pub fn is_known_metaclass(name: &str) -> bool {
    DIRECT_GENERALIZATIONS
        .iter()
        .any(|(candidate, _)| *candidate == name)
}

/// Reflexive, transitive UML metaclass inheritance; unknown names never conform.
/// This does not establish property, multiplicity, OCL, or behavioral conformance.
pub fn conforms_to(specific: &str, general: &str) -> bool {
    let (Some(specific), Some(general)) = (
        super::schema::uml_classifier(specific),
        super::schema::uml_classifier(general),
    ) else { return false; };
    super::schema::conforms_to(specific.id, general.id) == Ok(true)
}

#[cfg(test)]
fn conforms_in(specific: &str, general: &str, catalog: &[(&str, &[&str])]) -> bool {
    if !catalog.iter().any(|(name, _)| *name == specific)
        || !catalog.iter().any(|(name, _)| *name == general)
    {
        return false;
    }

    let mut pending = vec![specific];
    let mut visited = std::collections::BTreeSet::new();
    while let Some(current) = pending.pop() {
        if !visited.insert(current) {
            continue;
        }
        if current == general {
            return true;
        }
        if let Some((_, parents)) = catalog.iter().find(|(name, _)| *name == current) {
            pending.extend(parents.iter().copied());
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bellatrix_class_inheritance_is_transitive_and_directional() {
        for parent in ["Class", "StructuredClassifier", "Classifier", "Element"] {
            assert!(conforms_to("Component", parent));
            assert!(!conforms_to(parent, "Component"));
        }
        assert!(conforms_to("Class", "StructuredClassifier"));
        assert!(conforms_to("StructuredClassifier", "Classifier"));
        assert!(conforms_to("Device", "Node"));
        assert!(!conforms_to("Node", "Device"));
        assert!(conforms_to("Stereotype", "Class"));
        assert!(!conforms_to("Interface", "Class"));
        assert!(conforms_to("Interface", "Classifier"));
    }

    #[test]
    fn bellatrix_multiple_inheritance_preserves_both_branches() {
        assert!(conforms_to("AssociationClass", "Class"));
        assert!(conforms_to("AssociationClass", "Association"));
        assert!(conforms_to("AssociationClass", "Relationship"));
        assert!(!conforms_to("Class", "AssociationClass"));
    }

    #[test]
    fn bellatrix_unknown_names_never_conform() {
        assert!(!is_known_metaclass("Unknown"));
        assert!(!is_known_metaclass("class"));
        assert!(!conforms_to("Unknown", "Unknown"));
        assert!(!conforms_to("Unknown", "Element"));
        assert!(!conforms_to("Class", "Unknown"));
        assert!(!conforms_to("", ""));
    }

    #[test]
    fn bellatrix_catalog_is_sorted_closed_and_reflexive() {
        for pair in DIRECT_GENERALIZATIONS.windows(2) {
            if let [left, right] = pair {
                assert!(left.0 < right.0);
            }
        }
        for (name, parents) in DIRECT_GENERALIZATIONS {
            assert!(is_known_metaclass(name));
            assert!(conforms_to(name, name));
            for parent in *parents {
                assert!(is_known_metaclass(parent));
                assert!(conforms_to(name, parent));
            }
        }
    }

    #[test]
    fn bellatrix_declared_properties_follow_multiple_and_transitive_inheritance() {
        assert!(has_property("Component", "ownedOperation"));
        assert!(has_property("Artifact", "ownedOperation"));
        assert!(has_property("AssociationClass", "ownedPort"));
        assert!(has_property("Collaboration", "part"));
        assert!(!has_property("Collaboration", "ownedPort"));
        assert!(!has_property("Signal", "ownedOperation"));
        assert!(!has_property("Model", "ownedAttribute"));
        assert!(!has_property("Unknown", "ownedAttribute"));
        assert!(!has_property("Class", "unknown"));
        for (owner, properties) in DECLARED_PROPERTIES {
            assert!(is_known_metaclass(owner));
            for property in *properties {
                assert!(has_property(owner, property));
            }
        }
    }

    #[test]
    fn bellatrix_traversal_terminates_on_cycles() {
        let cyclic: &[(&str, &[&str])] = &[("A", &["B"]), ("B", &["A"]), ("C", &[])];
        assert!(conforms_in("A", "B", cyclic));
        assert!(!conforms_in("A", "C", cyclic));
    }
}
'''


def generate_legacy(source_bytes: bytes) -> bytes:
    root = ElementTree.fromstring(source_bytes)
    classes = [element for element in root.iter() if element.get(XMI + "type") == "uml:Class"]
    names_by_id = {}
    elements_by_name = {}
    for element in classes:
        identifier = element.get(XMI + "id")
        name = element.get("name")
        if not identifier or not name:
            raise ValueError("Metaclass has no XMI identifier or name")
        if identifier in names_by_id or name in elements_by_name:
            raise ValueError(f"Duplicate metaclass identifier or name: {identifier}, {name}")
        names_by_id[identifier] = name
        elements_by_name[name] = element
    if not classes:
        raise ValueError("No UML metaclasses found")

    rows = []
    for name, element in sorted(elements_by_name.items()):
        parents = set()
        for generalization in element.findall("generalization"):
            identifier = generalization.get("general")
            if identifier not in names_by_id:
                raise ValueError(f"Unresolved parent for {name}: {identifier}")
            parents.add(names_by_id[identifier])
        parent_values = ", ".join(json.dumps(parent) for parent in sorted(parents))
        rows.append(f"    ({json.dumps(name)}, &[{parent_values}]),")

    property_rows = []
    for name, element in sorted(elements_by_name.items()):
        names = {prop.get("name") for prop in element.findall("ownedAttribute")}
        if any(not name for name in names):
            raise ValueError(f"Unnamed property in {name}")
        properties = sorted(names)
        values = ", ".join(json.dumps(prop) for prop in properties)
        property_rows.append(f"    ({json.dumps(name)}, &[{values}]),")
    properties = "\n#[rustfmt::skip]\npub const DECLARED_PROPERTIES: &[(&str, &[&str])] = &[\n" + "\n".join(property_rows) + "\n];\n"
    digest = hashlib.sha256(source_bytes).hexdigest()
    header = f'''// Generated by script/bellatrix_mde/bellatrix_uml_catalog.py; do not edit.
//! Source-qualified UML 2.5.1 structural contracts from four pinned OMG documents.
//! Runtime support is selected by uml::schema; metadata is not full UML conformance.
//! Constraint descriptors identify unevaluated specifications, not executable OCL.

pub const SOURCE_URL: &str = "{SOURCE_URL}";
pub const SOURCE_PATH: &str = "{SOURCE.as_posix()}";
pub const SOURCE_SHA256: &str = "{digest}";

#[rustfmt::skip]
pub const DIRECT_GENERALIZATIONS: &[(&str, &[&str])] = &[
'''
    return (header + "\n".join(rows) + "\n];\n" + properties).encode("utf-8")



# Imported versions retain their own document identities, including historical UML.
DOCUMENTS = {
    "Uml": (SOURCE, "http://www.omg.org/spec/UML/20161101/UML.xmi"),
    "PrimitiveTypes": (SOURCE.parent / "PrimitiveTypes.xmi", "http://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi"),
    "StandardProfile": (SOURCE.parent / "StandardProfile.xmi", "http://www.omg.org/spec/UML/20161101/StandardProfile.xmi"),
    "UmlDi": (SOURCE.parent / "UMLDI.xmi", "http://www.omg.org/spec/UML/20161101/UMLDI.xmi"),
    "PrimitiveTypes20131001": (SOURCE.parent.parent / "imports/20131001/PrimitiveTypes.xmi", "http://www.omg.org/spec/UML/20131001/PrimitiveTypes.xmi"),
    "Uml20131001": (SOURCE.parent.parent / "imports/20131001/UML.xmi", "http://www.omg.org/spec/UML/20131001/UML.xmi"),
    "Di": (SOURCE.parent.parent / "imports/20131001/DI.xmi", "http://www.omg.org/spec/DD/20131001/DI.xmi"),
    "Dc": (SOURCE.parent.parent / "imports/20131001/DC.xmi", "http://www.omg.org/spec/DD/20131001/DC.xmi"),
}
REFERENCE_ATTRIBUTES = {
    "type", "association", "general", "importedPackage", "bodyCondition", "instance",
    "element", "metamodelReference", "memberEnd", "annotatedElement", "constrainedElement",
    "redefinedProperty", "subsettedProperty", "redefinedOperation", "precondition", "postcondition",
}
CLASSIFIER_KINDS = {
    "uml:Class": "Class", "uml:Stereotype": "Stereotype", "uml:PrimitiveType": "Primitive",
    "uml:Enumeration": "Enumeration", "uml:Association": "Association", "uml:Extension": "Extension",
    "uml:DataType": "DataType",
}


def quoted(value):
    return json.dumps(value, ensure_ascii=False)


def identity(document, identifier):
    return f"XmiId::new(Document::{document}, {quoted(identifier)})"


def optional_string(value):
    return "None" if value is None else f"Some({quoted(value)})"


def text_values(element, role):
    values = [element.get(role)] if element.get(role) is not None else []
    return values + [child.text or "" for child in element.findall(role)]


def rust_row(typename, **fields):
    return "    " + typename + " { " + ", ".join(f"{key}: {value}" for key, value in fields.items()) + " },"


def rust_slice(values):
    return "&[" + ", ".join(values) + "]"


class Contracts:
    def __init__(self):
        self.bytes = {document: (ROOT / path).read_bytes()
                      for document, (path, _) in DOCUMENTS.items()}
        self.roots = {document: ElementTree.fromstring(value) for document, value in self.bytes.items()}
        self.ids = {}
        self.parents = {}
        self.duplicates = {}
        self.unresolved = []
        self.counts = {document: {"Exact": 0, "ExternalDocument": 0,
                                  "MissingIdentifier": 0, "AmbiguousIdentifier": 0} for document in DOCUMENTS}
        for document, root in self.roots.items():
            for parent in root.iter():
                for child in parent:
                    self.parents[child] = parent
                identifier = parent.get(XMI + "id")
                if identifier:
                    key = (document, identifier)
                    if key in self.ids:
                        self.duplicates.setdefault(key, [self.ids[key]]).append(parent)
                    else:
                        self.ids[key] = parent
        self.urls = {url: document for document, (_, url) in DOCUMENTS.items()}
        self.urls.update({url.replace("http:", "https:"): document
                          for url, document in list(self.urls.items())})

    def resolve(self, document, raw):
        if "#" in raw:
            url, identifier = raw.rsplit("#", 1)
            if not url:
                target_document, resolution = document, "Exact"
            elif url in self.urls:
                target_document, resolution = self.urls[url], "Exact"
            else:
                return raw, None, "ExternalDocument"
        else:
            identifier, target_document, resolution = raw, document, "Exact"
        if (target_document, identifier) in self.duplicates:
            return raw, None, "AmbiguousIdentifier"
        if (target_document, identifier) not in self.ids:
            return raw, None, "MissingIdentifier"
        return raw, (target_document, identifier), resolution

    def reference(self, document, raw):
        raw, target, resolution = self.resolve(document, raw)
        target = f"Some({identity(*target)})" if target else "None"
        return f"XmiReference {{ raw: {quoted(raw)}, target: {target}, resolution: ReferenceResolution::{resolution} }}"

    def references(self, element, role):
        values = element.get(role, "").split()
        for child in element.findall(role):
            value = child.get("href") or child.get(XMI + "idref")
            if not value:
                raise ValueError(f"Unsupported {role} reference on {element.get(XMI + 'id')}")
            values.append(value)
        return values

    def owner(self, element):
        while element in self.parents:
            element = self.parents[element]
            if element.get(XMI + "id"):
                return element.get(XMI + "id")
        raise ValueError("XMI node has no identified owner")

    def audit_references(self):
        for document, root in self.roots.items():
            for element in root.iter():
                references = []
                for name in REFERENCE_ATTRIBUTES:
                    references.extend((name, value) for value in element.get(name, "").split())
                for attribute in ("href", XMI + "idref"):
                    if element.get(attribute):
                        references.append((element.tag, element.get(attribute)))
                for role, raw in references:
                    _, target, resolution = self.resolve(document, raw)
                    self.counts[document][resolution] += 1
                    if target is None:
                        owner = element.get(XMI + "id") or self.owner(element)
                        self.unresolved.append((document, owner, role, raw))

    def bound(self, element, role):
        literal = element.find(role)
        if literal is None:
            return '"0"' if role == "lowerValue" and element.get(XMI + "type") == "uml:ExtensionEnd" else '"1"'
        kind = literal.get(XMI + "type")
        if kind not in ("uml:LiteralInteger", "uml:LiteralUnlimitedNatural"):
            raise ValueError(f"Nonliteral metamodel bound on {element.get(XMI + 'id')}")
        value = literal.get("value", "0")
        if value != "*" and (not value.isdecimal()):
            raise ValueError(f"Invalid metamodel bound: {value}")
        return quoted(value)

    def default(self, document, element):
        default = element.find("defaultValue")
        if default is None:
            return "None"
        kind = default.get(XMI + "type", "")
        if kind == "uml:LiteralBoolean":
            value = default.get("value", "false")
            if value not in ("true", "false"):
                raise ValueError(f"Invalid Boolean literal: {value}")
            value = f"DefaultValue::Boolean({value})"
        elif kind in ("uml:LiteralInteger", "uml:LiteralUnlimitedNatural"):
            value = f"DefaultValue::{kind.removeprefix('uml:Literal')}({quoted(default.get('value', '0'))})"
        elif kind in ("uml:LiteralReal", "uml:LiteralString") and default.get("value") is not None:
            value = f"DefaultValue::{kind.removeprefix('uml:Literal')}({quoted(default.get('value'))})"
        elif kind == "uml:InstanceValue":
            references = self.references(default, "instance")
            if len(references) != 1:
                raise ValueError(f"InstanceValue requires one instance: {default.attrib}")
            _, target, _ = self.resolve(document, references[0])
            property_types = self.references(element, "type")
            expected = self.resolve(document, property_types[0])[1] if len(property_types) == 1 else None
            if target and self.ids[target].get(XMI + "type") == "uml:EnumerationLiteral" and expected == (target[0], self.owner(self.ids[target])):
                value = f"DefaultValue::Enumeration {{ literal: {identity(*target)}, name: {quoted(self.ids[target].get('name'))} }}"
            else:
                value = f"DefaultValue::Unsupported({identity(document, default.get(XMI + 'id'))})"
        else:
            value = f"DefaultValue::Unsupported({identity(document, default.get(XMI + 'id'))})"
        return f"Some({value})"

    def generate(self):
        self.audit_references()
        lines = ["\nuse super::schema::{Aggregation, AmbiguousDefinition, ClassifierDescriptor, ClassifierKind, ConstraintDescriptor, DefaultValue, Document, DocumentationDescriptor, OperationDescriptor, ParameterDescriptor, ParameterDirection, PropertyDescriptor, SpecificationDescriptor, ReferenceResolution, SourceDescriptor, UnresolvedReference, XmiId, XmiReference};\n"]
        def table(name, typename, rows):
            lines.extend(["#[rustfmt::skip]", f"pub const {name}: &[{typename}] = &[", *rows, "];\n"])
        rows = []
        for document, (path, url) in DOCUMENTS.items():
            counts = self.counts[document]
            path = path.as_posix()
            rows.append(f"    SourceDescriptor {{ document: Document::{document}, path: {quoted(path)}, url: {quoted(url)}, sha256: {quoted(hashlib.sha256(self.bytes[document]).hexdigest())}, resolved_references: {counts['Exact']}, unresolved_references: {counts['ExternalDocument'] + counts['MissingIdentifier'] + counts['AmbiguousIdentifier']} }},")
        table("SOURCES", "SourceDescriptor", rows)
        table("AMBIGUOUS_DEFINITIONS", "AmbiguousDefinition", [
            f"    AmbiguousDefinition {{ id: {identity(document, identifier)}, owners: {rust_slice(identity(document, self.owner(element)) for element in elements)} }},"
            for (document, identifier), elements in sorted(self.duplicates.items())])
        table("UNRESOLVED_REFERENCES", "UnresolvedReference", [
            f"    UnresolvedReference {{ owner: {identity(document, owner)}, role: {quoted(role)}, reference: {self.reference(document, raw)} }},"
            for document, owner, role, raw in sorted(self.unresolved)])
        classifiers, properties, constraints = [], [], []
        operations, parameters, specifications, documentation = [], [], [], []
        for (document, identifier), element in sorted(self.ids.items()):
            if (document, identifier) in self.duplicates:
                continue
            kind = element.get(XMI + "type")
            if kind in CLASSIFIER_KINDS:
                parents = [value for generalization in element.findall("generalization")
                           for value in self.references(generalization, "general")]
                owned = [child.get(XMI + "id") for child in element
                         if child.get(XMI + "type") in ("uml:Property", "uml:ExtensionEnd")]
                literals = element.findall("ownedLiteral")
                classifiers.append(f"    ClassifierDescriptor {{ id: {identity(document, identifier)}, name: {quoted(element.get('name'))}, kind: ClassifierKind::{CLASSIFIER_KINDS[kind]}, is_abstract: {element.get('isAbstract', 'false')}, generals: {rust_slice(self.reference(document, parent) for parent in parents)}, owned_properties: {rust_slice(identity(document, prop) for prop in owned)}, owned_operations: {rust_slice(self.reference(document, operation.get(XMI + 'id')) for operation in element.findall('ownedOperation'))}, member_ends: {rust_slice(self.reference(document, member) for member in self.references(element, 'memberEnd'))}, literal_names: {rust_slice(quoted(literal.get('name')) for literal in literals)}, literal_ids: {rust_slice(identity(document, literal.get(XMI + 'id')) for literal in literals)} }},")
            if kind in ("uml:Property", "uml:ExtensionEnd"):
                types = self.references(element, "type")
                if len(types) != 1:
                    raise ValueError(f"Property must have exactly one Type: {document}#{identifier}")
                associations = self.references(element, "association")
                if len(associations) > 1:
                    raise ValueError(f"Property has multiple associations: {document}#{identifier}")
                association = f"Some({self.reference(document, associations[0])})" if associations else "None"
                opposites = []
                if associations:
                    _, target, _ = self.resolve(document, associations[0])
                    if target:
                        members = self.references(self.ids[target], "memberEnd")
                        member_targets = [self.resolve(target[0], member)[1] for member in members]
                        if len(members) == 2 and (document, identifier) in member_targets:
                            opposites = [self.reference(target[0], member) for member, member_target in zip(members, member_targets)
                                         if member_target != (document, identifier)]
                aggregation = element.get("aggregation", "none").capitalize()
                if aggregation not in ("None", "Shared", "Composite"):
                    raise ValueError(f"Unknown aggregation: {aggregation}")
                properties.append(f"    PropertyDescriptor {{ id: {identity(document, identifier)}, owner: {identity(document, self.owner(element))}, name: {quoted(element.get('name'))}, property_type: {self.reference(document, types[0])}, visibility: {optional_string(element.get('visibility'))}, lower: {self.bound(element, 'lowerValue')}, upper: {self.bound(element, 'upperValue')}, default: {self.default(document, element)}, is_derived: {element.get('isDerived', 'false')}, is_derived_union: {element.get('isDerivedUnion', 'false')}, is_read_only: {element.get('isReadOnly', 'false')}, is_ordered: {element.get('isOrdered', 'false')}, is_unique: {element.get('isUnique', 'true')}, aggregation: Aggregation::{aggregation}, association: {association}, opposites: {rust_slice(opposites)}, subsets: {rust_slice(self.reference(document, reference) for reference in self.references(element, 'subsettedProperty'))}, redefines: {rust_slice(self.reference(document, reference) for reference in self.references(element, 'redefinedProperty'))} }},")
            if kind == "uml:Operation":
                operations.append(rust_row("OperationDescriptor",
                    id=identity(document, identifier), owner=identity(document, self.owner(element)),
                    name=quoted(element.get("name")), is_query=element.get("isQuery", "false"),
                    is_abstract=element.get("isAbstract", "false"), visibility=optional_string(element.get("visibility")),
                    parameters=rust_slice(self.reference(document, parameter.get(XMI + "id")) for parameter in element.findall("ownedParameter")),
                    redefines=rust_slice(self.reference(document, reference) for reference in self.references(element, "redefinedOperation")),
                    preconditions=rust_slice(self.reference(document, reference) for reference in self.references(element, "precondition")),
                    postconditions=rust_slice(self.reference(document, reference) for reference in self.references(element, "postcondition")),
                    body_conditions=rust_slice(self.reference(document, reference) for reference in self.references(element, "bodyCondition"))))
            if kind == "uml:Parameter":
                types = self.references(element, "type")
                if len(types) > 1:
                    raise ValueError(f"Parameter has multiple Types: {document}#{identifier}")
                direction = {"in": "In", "out": "Out", "inout": "InOut", "return": "Return"}.get(element.get("direction", "in"))
                if direction is None:
                    raise ValueError(f"Unknown parameter direction: {document}#{identifier}")
                parameters.append(rust_row("ParameterDescriptor",
                    id=identity(document, identifier), owner=identity(document, self.owner(element)),
                    name=quoted(element.get("name")), parameter_type=f"Some({self.reference(document, types[0])})" if types else "None",
                    visibility=optional_string(element.get("visibility")), direction=f"ParameterDirection::{direction}", lower=self.bound(element, "lowerValue"), upper=self.bound(element, "upperValue"),
                    default=self.default(document, element), is_ordered=element.get("isOrdered", "false"),
                    is_unique=element.get("isUnique", "true"), is_exception=element.get("isException", "false"),
                    is_stream=element.get("isStream", "false"), effect=optional_string(element.get("effect"))))
            if kind == "uml:OpaqueExpression":
                specifications.append(rust_row("SpecificationDescriptor",
                    id=identity(document, identifier), owner=identity(document, self.owner(element)), name=optional_string(element.get("name")),
                    bodies=rust_slice(quoted(body) for body in text_values(element, "body")),
                    languages=rust_slice(quoted(language) for language in text_values(element, "language"))))
            if kind == "uml:Comment":
                bodies = text_values(element, "body")
                if len(bodies) > 1:
                    raise ValueError(f"Comment has multiple body values: {document}#{identifier}")
                documentation.append(rust_row("DocumentationDescriptor",
                    id=identity(document, identifier), owner=identity(document, self.owner(element)),
                    annotated_elements=rust_slice(self.reference(document, reference) for reference in self.references(element, "annotatedElement")),
                    body=optional_string(bodies[0] if bodies else None)))
            if kind == "uml:Constraint":
                specification = element.find("specification")
                languages = [] if specification is None else text_values(specification, "language")
                spec = "None" if specification is None else f"Some({identity(document, specification.get(XMI + 'id'))})"
                constraints.append(rust_row("ConstraintDescriptor",
                    id=identity(document, identifier), owner=identity(document, self.owner(element)), name=quoted(element.get("name")),
                    specification=spec, constrained_elements=rust_slice(self.reference(document, reference) for reference in self.references(element, "constrainedElement")),
                    languages=rust_slice(quoted(language) for language in languages)))
        table("CLASSIFIERS", "ClassifierDescriptor", classifiers)
        table("PROPERTIES", "PropertyDescriptor", properties)
        table("UNEVALUATED_CONSTRAINTS", "ConstraintDescriptor", constraints)
        table("OPERATIONS", "OperationDescriptor", operations)
        table("PARAMETERS", "ParameterDescriptor", parameters)
        table("SPECIFICATIONS", "SpecificationDescriptor", specifications)
        table("DOCUMENTATION", "DocumentationDescriptor", documentation)
        return "\n".join(lines).encode("utf-8")


    def inventory(self):
        representation = {
            **{kind: "ClassifierDescriptor" for kind in CLASSIFIER_KINDS},
            "uml:Property": "PropertyDescriptor", "uml:ExtensionEnd": "PropertyDescriptor",
            "uml:Operation": "OperationDescriptor", "uml:Parameter": "ParameterDescriptor",
            "uml:OpaqueExpression": "SpecificationDescriptor (bodies and languages, not execution)",
            "uml:Constraint": "ConstraintDescriptor (not execution)", "uml:Comment": "DocumentationDescriptor",
            "uml:EnumerationLiteral": "embedded classifier literal identity/name",
            "uml:Generalization": "embedded classifier general reference",
            **{f"uml:{kind}": "embedded typed default or multiplicity bound" for kind in
               ("LiteralBoolean", "LiteralInteger", "LiteralUnlimitedNatural", "LiteralReal", "LiteralString", "InstanceValue")},
        }
        sources = []
        for document, (path, url) in DOCUMENTS.items():
            occurrences = Counter(element.get(XMI + "type", "untyped") for element in self.roots[document].iter() if element.get(XMI + "id"))
            unique = Counter(element.get(XMI + "type", "untyped") for (owner, _), element in self.ids.items() if owner == document)
            unambiguous = Counter(element.get(XMI + "type", "untyped") for key, element in self.ids.items() if key[0] == document and key not in self.duplicates)
            sources.append({"document": document, "path": path.as_posix(), "url": url,
                "sha256": hashlib.sha256(self.bytes[document]).hexdigest(), "references": self.counts[document],
                "definitions": [{"xmi_type": kind, "identified_occurrences": count, "unique_ids": unique[kind],
                    "unambiguous_ids": unambiguous[kind], "representation": representation.get(kind, "reference-audited/provenance only; no typed descriptor")}
                    for kind, count in sorted(occurrences.items())]})
        return {
            "format_version": 1,
            "generator": "script/bellatrix_mde/bellatrix_uml_catalog.py",
            "generated_rust": TARGET.as_posix(),
            "sources": sources,
            "reference_rules": "Document-qualified identities; historical versions are distinct; duplicate IDs never resolve by first-match.",
            "ambiguous_definitions": [{"document": document, "id": identifier,
                "owners": [self.owner(element) for element in elements], "occurrences": len(elements)}
                for (document, identifier), elements in sorted(self.duplicates.items())],
            "unresolved_references": [{"document": document, "owner": owner, "role": role, "raw": raw,
                "status": self.resolve(document, raw)[2]} for document, owner, role, raw in sorted(self.unresolved)],
            "executable_consumers": [
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/schema.rs", "scope": "Source-qualified inheritance/redefinition queries, inherited defaults, shared authored/default scalar readers, explicitly supplied scalar type and null/cardinality checks; metadata reads for operations/specifications/documentation."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/root.rs", "scope": "Effective visibility defaults, scalar validation and qualified names via the shared allNamespaces query; separate handwritten Root semantic rules."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/element_graph_queries.rs", "scope": "Canonical Element ownership closure, NamedElement namespace/package chains and inverse Dependency clients; exposed template parameters, identity/cycle checks and explicit unsupported alternate representations."},
                {"file": "crates/bellatrix_mde/src/view-model/inspector.rs", "scope": "Canonical scalar/reference field editors, choices and defaults. Semantic edit validation and mutation are in crates/bellatrix_mde/src/KERNEL/inspector_edit.rs; unresolved type text and legacy defaultValue declarations remain preserved."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/references.rs", "scope": "Snapshot-indexed local TypedElement.type and descriptor-driven reference collections with bounds, uniqueness and target conformance; derived Association membership index; external references remain unsupported."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/instances.rs", "scope": "Canonical Slot ownership, classifier/definingFeature references and uniqueness; shared slottable-feature resolution includes inherited private attributes and direct redefinitions. Values and ambiguous ancestor redefinitions remain explicit unsupported capabilities."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/value_specifications.rs", "scope": "Pure evaluation of canonical Integer/UnlimitedNatural/Real/Boolean/String/Null literal identities and generated defaults, computability, null and typed value queries; Real uses finite JSON numbers. Owned lowerValue/upperValue role resolution is shared by Property, Parameter, Operation return and redefinition checks, plus independent upperBound evaluation for subsetting. External references, opaque/nonliteral expression evaluation and unassigned owned-value roles remain explicit unsupported capabilities."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/types.rs", "scope": "Arbitrary-precision Integer, UnlimitedNatural upper-bound comparison and evaluated multiplicity intervals; one retained literal-notation adapter, with reference-aware ValueSpecification evaluation supplied by the separate shared consumer."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/operations.rs", "scope": "Ordered canonical Parameters; generated scalar defaults; derived return Type, ordering, uniqueness and reference-aware multiplicity; selected Parameter constraints and only_body_for_query with owned Constraint references. Nonliteral ValueSpecifications and condition expressions remain explicit limits; signature compatibility consumes these projections in operation_signatures."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/associations.rs", "scope": "Canonical ordered Property member ends, derived ownership/opposite/end Types, binary compatibility migration/projections and selected Association/Property constraints; n-ary queries supported, binary notation and specialized/navigability/ValueSpecification limits explicit."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/classifiers.rs", "scope": "Generated ownership subsets, direct/inherited Namespace members and Classifier features/attributes, direct redefinition filtering and slottable-feature unions. Bounded Property and normative Operation signature consistency are consumed from shared feature_compatibility/operation_signatures after checking redefinition contexts; independent behavioral/template contracts are not additional Operation consistency requirements. Ambiguous ownership/ancestor redefinition cases remain explicit unsupported capabilities."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/property_subsetting.rs", "scope": "Canonical Property subset targets, different names, shared Type conformance and upper-bound restriction; attribute and n-ary Association-end context conformance. Consumed by semantic command, structural validation and Inspector; no lower-bound restriction or feature replacement, and unresolved contexts/Types/expressions remain explicit unsupported diagnostics."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/redefinition_contexts.rs", "scope": "Generated ownership-subset derivation of RedefinableElement contexts and the default allParents/includesAll predicate; consumed before Property/Operation consistency and by Inspector. Association-owned end redefinition uses ancestor allFeatures rather than attributes. Behavior/state-machine specialized contexts and alternate stored context lists remain explicit unsupported capabilities."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/operation_signatures.rs", "scope": "Official Operation signature consistency over ordered canonical Parameters: equal count/direction/ordering/uniqueness, bidirectional Type conformance and direction-dependent interval containment. Consumed by redefinition commands, validation and Inspector; unsupported Type/bound evaluation remains explicit. Conditions, methods, exceptions and templates retain independent validation and do not impose extra consistency requirements."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/classifier_hierarchy.rs", "scope": "User-model Classifier ancestry and conformance over canonical Generalization edges, separate from metaclass conformance; cycle detection, identity deduplication and topological ordering shared by Namespace inheritance. Canonical Generalization edges enforce the official metaclass-based maySpecializeType and non-final-parent rules before ancestry consumers use them."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/feature_compatibility.rs", "scope": "Reusable TypedElement Type conformance and MultiplicityElement interval containment; Property redefinition consistency additionally preserves composite aggregation. Missing/legacy/external Types and unevaluated ValueSpecifications remain explicit unsupported outcomes; shared resolved-Type and known/unsupported Boolean comparisons also support Operation signatures and Property subsetting; no general OCL interpreter is claimed."},
                {"file": "crates/bellatrix_mde/src/KERNEL/uml/namespace_imports.rs", "scope": "Canonical ElementImport/PackageImport aliases, explicit-import precedence, visibility, collisions and public re-exports. SCC decomposition and finite fixed points resolve collision-free cycles; cyclic collisions and alternate import representations remain explicit unsupported capabilities."},
            ],
            "not_executable_from_extraction": [
                "OCL/opaque specification bodies and generated operation definitions are preserved, not interpreted or compiled into behavior.",
                "Descriptor extraction alone does not execute collection/relationship constraints or complete subset/redefinition consistency; the bounded handwritten consumers above implement only their stated contracts. External user-model references remain unsupported.",
                "Derived-property maintenance requires handwritten implementations; unsupported derived setters are blocked independently of isReadOnly.",
                "Stereotype application, complete UMLDI rendering/validation, general ValueSpecification expression evaluation and behavioral execution are not implemented by these descriptors.",
                "Ambiguous source definitions are excluded from typed tables and remain available in the pinned source with this ambiguity ledger.",
                "Package/Profile container definitions, package-import object identities and MOF extension tags are provenance/reference-audited only; their missing typed representation is listed per source.",
                "Finite JSON Real validation is a transport boundary, not a complete mathematical Real implementation.",
            ],
        }


def build() -> tuple[bytes, bytes]:
    contracts = Contracts()
    generated = generate_legacy(contracts.bytes["Uml"]) + contracts.generate() + RUST.encode("utf-8")
    inventory = (json.dumps(contracts.inventory(), indent=2, ensure_ascii=False) + "\n").encode("utf-8")
    return generated, inventory


def generate() -> bytes:
    return build()[0]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Fail if checked-in output differs; do not write")
    parser.add_argument("--output", type=Path, default=ROOT / TARGET, help="Generated Rust file destination")
    parser.add_argument("--inventory-output", type=Path, help="Coverage JSON destination (default: Rust output with .inventory.json suffix)")
    arguments = parser.parse_args()
    inventory_output = arguments.inventory_output or arguments.output.with_suffix(".inventory.json")
    if inventory_output.resolve() == arguments.output.resolve():
        parser.error("Rust output and coverage inventory must be different files")
    try:
        expected, inventory = build()
        for destination, content in ((arguments.output, expected), (inventory_output, inventory)):
            if arguments.check:
                if not destination.is_file() or destination.read_bytes() != content:
                    print(f"UML extraction drift: regenerate {destination}", file=sys.stderr)
                    return 1
                print(f"UML extraction is current: {destination}")
            else:
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(content)
                print(f"Generated {destination}")
    except (OSError, ValueError, ElementTree.ParseError) as error:
        print(f"UML catalog generation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
