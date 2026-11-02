"""Static checks for the Bellatrix monorepo; never a substitute for Cargo."""
from pathlib import Path
import base64
import json
import math
import re
import tomllib

ROOT = Path(__file__).resolve().parents[3]
MDE = ROOT / "engines" / "mde"
WORKSPACE = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]
MEMBERS = WORKSPACE["members"]
EXPECTED = {
    "apps/desktop": "bellatrix",
    "platform/project": "bellatrix-project",
    "engines/mde/model": "bellatrix-mde-model",
    "engines/mde/modeling": "bellatrix-mde-modeling",
    "engines/mde/cordis": "bellatrix-mde-cordis",
    "engines/mde/generator-rust": "bellatrix-mde-rust-generator",
    "engines/mde/export-html": "bellatrix-mde-html-exporter",
    "engines/mde/cli": "bellatrix-mde-cli",
    "ui/desktop": "bellatrix-ui",
    "ui/design-system": "bellatrix-design",
    **{f"ui/components/{kind}": f"bellatrix-components-{kind}"
       for kind in ("navigation", "panels", "toolbox", "tree", "symbols",
                    "canvas", "uml", "c4", "mindmap", "flow")},
}

assert len(MEMBERS) == len(set(MEMBERS)), "Duplicate workspace member"
assert set(WORKSPACE["default-members"]) <= set(MEMBERS)
assert set(EXPECTED) <= set(MEMBERS)
assert (ROOT / "docs" / "design" / "src" / "app" / "App.tsx").is_file()
assert (ROOT / "docs" / "ref" / "staruml" / "src" / "ui" / "diagram-editor.js").is_file()
assert (ROOT / "vendor" / "gpui-kit" / "skills" / "gpui-kit" / "SKILL.md").is_file()
assert (ROOT / "vendor" / "gpui-kit" / "skills" / "gpui-kit-design-guides" / "SKILL.md").is_file()
assert (ROOT / "vendor" / "cordis-rs" / "CONTEXT.md").is_file()
assert (ROOT / "docs" / "history" / "specs" / "006-project-composition" / "acceptance.feature").is_file()

package_names = {}
dependency_count = 0
for member in MEMBERS:
    directory = ROOT / member
    manifest = directory / "Cargo.toml"
    assert manifest.is_file(), f"Missing Cargo manifest: {member}"
    spec = tomllib.loads(manifest.read_text(encoding="utf-8"))
    name = spec["package"]["name"]
    assert name not in package_names, f"Duplicate package {name}"
    package_names[name] = member
    if member in EXPECTED:
        assert name == EXPECTED[member], (member, name, EXPECTED[member])
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for dep, value in spec.get(section, {}).items():
            if not isinstance(value, dict) or "path" not in value:
                continue
            dependency_count += 1
            target = (directory / value["path"] / "Cargo.toml").resolve()
            assert target.is_file(), f"Broken {section}: {member} -> {dep}: {target}"
            target_package = tomllib.loads(target.read_text(encoding="utf-8"))["package"]["name"]
            assert target_package == value.get("package", dep), (
                f"Wrong Cargo dependency name: {member} -> {dep}, actual {target_package}"
            )

included = 0
long_files = []
include_rx = re.compile(r'include_(?:str|bytes)!\s*\(\s*"([^"]+)"\s*\)')
source_dirs = (ROOT / "apps", ROOT / "platform", ROOT / "engines", ROOT / "ui")
for base in source_dirs:
    for code in base.rglob("*.rs"):
        body = code.read_text(encoding="utf-8")
        lines = body.splitlines()
        if len(lines) > 200:
            long_files.append(str(code.relative_to(ROOT)))
        for old in ("src/modeling/", "src/bellatrix/", "src/ui/", "crates/components/",
                    "crates/platform/", "engines/application-builder/"):
            assert old not in body, f"Stale repository path {old} in {code}"
        for match in include_rx.finditer(body):
            included += 1
            included_file = (code.parent / match.group(1)).resolve()
            assert included_file.is_file(), f"Missing include in {code}: {match.group(1)}"

# Public Cordis Events are shared contracts, not per-engine buses.
event_sources = (
    ROOT / "platform/project/src/events.rs",
    MDE / "cordis/src/lib.rs",
    MDE / "cordis/src/task_events.rs",
    MDE / "cordis/src/project_integration.rs",
    MDE / "cordis/src/view_edits.rs",
)
event_names = []
for source in event_sources:
    data = source.read_text(encoding="utf-8")
    event_names.extend(re.findall(r'const NAME:\s*&\x27static str\s*=\s*"([^"]+)"', data))
assert len(event_names) == len(set(event_names)), "Duplicate public Cordis Event name"
assert {"bellatrix.project.state-changed.v1", "bellatrix.project.closed.v1",
        "bellatrix.modeling.move-view.v1", "bellatrix.modeling.rename.v1"} <= set(event_names)
assert "InjectSpec::none().require(ProjectService::NAME)" in (
    MDE / "cordis/src/project_integration.rs").read_text(encoding="utf-8")
assert (ROOT / "ui/desktop/src/pages/mde_workbench/project.rs").is_file()
assert not (ROOT / "ui/desktop/src/project.rs").is_file()

cli_manifest = MDE / "cli"
assert (cli_manifest / "../fixtures/todo.json").is_file(), "CLI fixture path is broken"
cli_test = (cli_manifest / "tests" / "cli.rs").read_text(encoding="utf-8")
assert '.join("../../crates/bellatrix_mde_model/fixtures/todo.json")' in cli_test, "CLI fixture must be relative to Cargo manifest"

for fixture in (MDE / "fixtures").glob("*.json"):
    document = json.loads(fixture.read_text(encoding="utf-8"))
    assert document["schema_version"] == 1, fixture
    elements = {e["id"]: e for e in document["elements"]}
    assert len(elements) == len(document["elements"]), fixture
    for relation in document["relations"]:
        assert relation["from"] in elements and relation["to"] in elements
    subjects = elements.keys() | {r["id"] for r in document["relations"]}
    for diagram in document["diagrams"]:
        for view in diagram["views"]:
            assert view.get("subject") is None or view["subject"] in subjects
            geometry = view["geometry"]
            if geometry["kind"] == "rect":
                assert geometry["width"] >= 0 and geometry["height"] >= 0
            if geometry["kind"] == "path":
                assert all(x["kind"] in ("line", "cubic") for x in geometry["segments"])
            assert all(math.isfinite(x) for x in view.get("style", {}).get("dash_pattern", []))
    for element in document["elements"]:
        if element.get("content", {}).get("encoding") == "base64":
            base64.b64decode(element["content"]["data"], validate=True)
    print(f"Valid MDE fixture: {fixture.name} ({len(elements)} elements)")

# 200 lines is a maintenance preference, not a correctness condition.
if long_files:
    print(f"MAINTENANCE: {len(long_files)} Rust files exceed 200 lines; review separately")
print(f"STATIC CHECKS PASSED: {len(MEMBERS)} crates, {dependency_count} local Cargo references, "
      f"{included} Rust includes, {len(event_names)} public Cordis events. "
      "Native compilation and GPUI behavior unverified.")
