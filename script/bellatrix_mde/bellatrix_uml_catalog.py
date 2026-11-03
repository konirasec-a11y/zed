#!/usr/bin/env python3
"""Generate UML metaclass inheritance and declared properties; this is not a UML/OCL validator."""

import argparse
import hashlib
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ElementTree


ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path("docs/bellatrix/standards/uml/20161101/UML.xmi")
TARGET = Path("crates/bellatrix_mde/src/uml/metamodel.rs")
SOURCE_URL = "https://www.omg.org/spec/UML/20161101/UML.xmi"
XMI = "{http://www.omg.org/spec/XMI/20131001}"

RUST = '''
/// Whether a metaclass declares or inherits a named UML property.
/// This is schema availability, not an evaluation of UML constraints or OCL.
pub fn has_property(metaclass: &str, property: &str) -> bool {
    DECLARED_PROPERTIES.iter().any(|(owner, names)| {
        names.contains(&property) && conforms_to(metaclass, owner)
    })
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
    conforms_in(specific, general, DIRECT_GENERALIZATIONS)
}

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


def generate(source_bytes: bytes) -> bytes:
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
//! Official UML 2.5.1 metaclass identity, inheritance and declared property names.
//! The catalog does not imply that all declared metaclasses or UML semantics
//! are implemented by Bellatrix. OCL and property constraints are not evaluated.

pub const SOURCE_URL: &str = "{SOURCE_URL}";
pub const SOURCE_PATH: &str = "{SOURCE.as_posix()}";
pub const SOURCE_SHA256: &str = "{digest}";

#[rustfmt::skip]
pub const DIRECT_GENERALIZATIONS: &[(&str, &[&str])] = &[
'''
    return (header + "\n".join(rows) + "\n];\n" + properties + RUST).encode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Fail if checked-in output differs; do not write")
    parser.add_argument("--output", type=Path, default=ROOT / TARGET, help="Generated Rust file destination")
    arguments = parser.parse_args()
    try:
        expected = generate((ROOT / SOURCE).read_bytes())
        if arguments.check:
            if not arguments.output.is_file() or arguments.output.read_bytes() != expected:
                print(f"UML catalog drift: regenerate {arguments.output}", file=sys.stderr)
                return 1
            print(f"UML catalog is current: {arguments.output}")
        else:
            arguments.output.parent.mkdir(parents=True, exist_ok=True)
            arguments.output.write_bytes(expected)
            print(f"Generated {arguments.output}")
    except (OSError, ValueError, ElementTree.ParseError) as error:
        print(f"UML catalog generation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
