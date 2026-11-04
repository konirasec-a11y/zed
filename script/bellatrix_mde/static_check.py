"""Validate Bellatrix ownership and source references within Zed's workspace.

This is a repository consistency check, not a replacement for Cargo or GPUI tests.
"""
from pathlib import Path
import re
import tomllib
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
CRATES = [
    "bellatrix_project",
    "bellatrix_mde",
    "bellatrix_mde_contracts",
    "bellatrix_mde_protocol",
    "bellatrix_mde_templates",
    "bellatrix_mde_cordis",
    "bellatrix_mde_rust_generator",
    "bellatrix_mde_html_exporter",
    "bellatrix_mde_cli",
    "bellatrix_zed_host",
    "bellatrix_mde_workbench",
    "bellatrix_design_system",
    "bellatrix_ui_uml",
    "bellatrix_ui_symbols",
    "bellatrix_ui_canvas",
    "bellatrix_ui_c4",
    "bellatrix_ui_flow",
    "bellatrix_ui_mindmap",
    "bellatrix_ui_tree",
    "bellatrix_ui_panels",
]
HEADLESS = {
    "bellatrix_project",
    "bellatrix_mde",
    "bellatrix_mde_contracts",
    "bellatrix_mde_protocol",
    "bellatrix_mde_templates",
    "bellatrix_mde_cordis",
    "bellatrix_mde_rust_generator",
    "bellatrix_mde_html_exporter",
}

def manifest(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))

def check_native_api_source(body: str, path: Path, icon_variants: set[str]) -> None:
    """Guard known API regressions; this does not type-check Rust or replace Cargo."""
    for variant in re.findall(r"\bIconName::(\w+)", body):
        assert variant in icon_variants, f"Unknown IconName::{variant} in {path}"
    for call in re.finditer(r"\bwindow\s*\.\s*focus\s*\(", body):
        depth = 1
        argument_start = call.end()
        arguments = []
        for index in range(call.end(), len(body)):
            character = body[index]
            if character in "([{":
                depth += 1
            elif character in ")]}":
                depth -= 1
                if depth == 0:
                    arguments.append(body[argument_start:index].strip())
                    break
            elif character == "," and depth == 1:
                arguments.append(body[argument_start:index].strip())
                argument_start = index + 1
        if arguments and not arguments[-1]:
            arguments.pop()
        assert len(arguments) == 2 and arguments[-1] == "cx", (
            f"Window::focus requires a handle and cx in {path}: {arguments}"
        )

def check() -> None:
    workspace = manifest(ROOT / "Cargo.toml")["workspace"]
    assert "crates/zed" in workspace["members"]
    dependencies = workspace["dependencies"]
    assert "cordis-rs" in dependencies
    for package in CRATES:
        directory = ROOT / "crates" / package
        cargo = manifest(directory / "Cargo.toml")
        assert cargo["package"]["name"] == package, package
        assert f"crates/{package}" in workspace["members"], package
        assert package in dependencies, package
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            for key, entry in cargo.get(section, {}).items():
                if not isinstance(entry, dict) or "path" not in entry:
                    continue
                target = (directory / entry["path"] / "Cargo.toml").resolve()
                assert target.is_file(), f"Broken dependency path {package} -> {key}: {target}"
                expected_package = entry.get("package", key)
                assert manifest(target)["package"]["name"] == expected_package, (
                    package, key, expected_package
                )
        if package in HEADLESS:
            assert all(
                key not in cargo.get("dependencies", {})
                for key in ("gpui", "workspace", "editor", "ui")
            ), f"Backend engine has GUI dependency: {package}"

    for obsolete in ("bellatrix_mde_model", "bellatrix_mde_modeling", "bellatrix_uml_metamodel"):
        assert f"crates/{obsolete}" not in workspace["members"], obsolete
        assert obsolete not in dependencies, obsolete
        assert not (ROOT / "crates" / obsolete / "Cargo.toml").exists(), obsolete

    includes = 0
    icons = (ROOT / "crates/icons/src/icons.rs").read_text(encoding="utf-8")
    icon_enum = re.search(r"pub enum IconName\s*\{(.*?)\n\}", icons, re.DOTALL)
    assert icon_enum, "IconName enum declaration not found"
    icon_variants = set(re.findall(r"^\s*(\w+)\s*,", icon_enum.group(1), re.MULTILINE))
    assert icon_variants, "IconName enum has no recognized variants"
    include_rx = re.compile(r'include_(?:str|bytes)!\s*\(\s*"([^"]+)"\s*\)')
    for package in CRATES:
        for path in (ROOT / "crates" / package).rglob("*.rs"):
            body = path.read_text(encoding="utf-8")
            check_native_api_source(body, path, icon_variants)
            assert "gpui_kit::" not in body, f"Legacy GPUI Kit type in {path}"
            for item in include_rx.finditer(body):
                include = (path.parent / item.group(1)).resolve()
                assert include.is_file(), f"Unresolved include in {path}: {item.group(1)}"
                includes += 1

    # RightClickMenu inserts an auto-height layout node: size_full on its trigger
    # alone collapses figures to their title or zero when children are absolute.
    workbench = (ROOT / "crates/bellatrix_mde_workbench/src/UI/diagram_canvas.rs").read_text(encoding="utf-8")
    figure_trigger = re.search(
        r'ui::right_click_menu\(format!\("mde-view-menu-.*?\.trigger\(move \|_, _, _\| \{(.*?)\}\)',
        workbench,
        re.DOTALL,
    )
    assert figure_trigger, "Diagram figure menu trigger must have explicit layout"
    trigger = figure_trigger.group(1)
    assert ".w(px(local_width * zoom))" in trigger
    assert ".h(px(local_height * zoom))" in trigger
    assert ".child(figure)" in trigger
    for filename in ["class.rs", "figures.rs"]:
        renderer = (ROOT / "crates/bellatrix_ui_uml/src" / filename).read_text(encoding="utf-8")
        assert "appearance.stroke(" in renderer, filename
    shared_style = (ROOT / "crates/bellatrix_ui_uml/src/class.rs").read_text(encoding="utf-8")
    assert "palette.diagram_outline(false)" in shared_style
    assert "rgb(palette.selection)" in shared_style
    assert workbench.count(".view_style(&view.style)") == 3, "Every native shape/edge renderer must receive its View style"

    zed_main = (ROOT / "crates/zed/src/main.rs").read_text(encoding="utf-8")
    assert "initialize_workspace(app_state.clone(), cx);" in zed_main
    assert "bellatrix_zed_host::install(cx).log_err();" in zed_main
    assert (ROOT / "crates/bellatrix_mde/fixtures/todo.json").is_file()
    assert (ROOT / "crates/bellatrix_mde/schema/mde-model-v1.proto").is_file()
    subprocess.run([sys.executable, str(ROOT / "script/bellatrix_mde/bellatrix_uml_catalog.py"), "--check"], check=True)
    print(f"Bellatrix Zed source contracts OK: {len(CRATES)} crates; {includes} includes")

if __name__ == "__main__":
    check()
