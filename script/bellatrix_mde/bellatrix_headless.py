#!/usr/bin/env python3
"""Validate exact Bellatrix headless sources without resolving the Zed workspace.

Python 3.11+; preparation does not invoke Cargo or install anything.
Example: bellatrix_headless.py --cache-dir /tmp/bellatrix-headless --prepare-only
Example: bellatrix_headless.py --cache-dir /tmp/bellatrix-headless -- test -p bellatrix_mde --test bdd_diagram_flows

This is supplemental validation, not native-workspace or GUI acceptance. Root
patches, profiles, lints and .cargo configuration are deliberately not inherited.
Never run two instances against the same cache directory concurrently.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib
from datetime import datetime, timezone


PACKAGES = ("bellatrix_mde", "bellatrix_mde_cordis")
BDD_TARGETS = {
    "bellatrix_mde": (
        "bdd_native_class_authoring", "bdd_diagram_flows",
        "bdd_mdj_association_flows", "bdd_mdj_presentation_flows",
    ),
    "bellatrix_mde_cordis": ("bdd_mdj_edit_save_flows",),
}
OWNED_TREES = tuple(f"crates/{name}" for name in PACKAGES) + (
    "docs/bellatrix/standards/uml",
)
MARKER = ".bellatrix-headless-cache.json"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def file_hashes(root):
    hashes = {}
    for relative in OWNED_TREES:
        tree = root / relative
        if not tree.is_dir():
            raise ValueError(f"Missing input tree: {tree}")
        for path in sorted(tree.rglob("*")):
            if path.is_symlink():
                raise ValueError(f"Symlinks are not supported in exact input copies: {path}")
            if path.is_file():
                hashes[path.relative_to(root).as_posix()] = digest(path)
    return hashes


def raw_entries(text, section, required):
    """Fail closed if upstream changes a selected declaration to multiline TOML."""
    active = False
    entries = {}
    for line in text.splitlines():
        if line.lstrip().startswith("["):
            active = line.strip() == f"[{section}]"
        elif active:
            match = re.match(r"\s*([\w-]+)\s*=", line)
            if match and match.group(1) in required:
                key = match.group(1)
                parsed = tomllib.loads(line)
                if set(parsed) != {key}:
                    raise ValueError(f"Unsupported declaration: {section}.{key}")
                entries[key] = line
    if set(entries) != set(required):
        raise ValueError(f"Cannot extract exact {section} entries: {set(required) - set(entries)}")
    return [entries[key] for key in sorted(entries)]


def inherited_dependencies(manifest):
    result = set()
    for section in (manifest, *manifest.get("target", {}).values()):
        for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
            for name, specification in section.get(kind, {}).items():
                if isinstance(specification, dict) and specification.get("workspace"):
                    result.add(name)
    return result


def workspace_manifest(repository):
    source = (repository / "Cargo.toml").read_text()
    workspace = tomllib.loads(source)["workspace"]
    dependencies = set()
    for name in PACKAGES:
        manifest = tomllib.loads((repository / "crates" / name / "Cargo.toml").read_text())
        if manifest["package"]["name"] != name:
            raise ValueError(f"Unexpected package name in {name}")
        dependencies.update(inherited_dependencies(manifest))
        for section in (manifest, *manifest.get("target", {}).values()):
            for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
                for specification in section.get(kind, {}).values():
                    if isinstance(specification, dict) and "path" in specification:
                        path = (repository / "crates" / name / specification["path"]).resolve()
                        if path not in {(repository / "crates" / package).resolve() for package in PACKAGES}:
                            raise ValueError(f"Dependency escapes headless scope: {path}")
    for name in dependencies:
        specification = workspace["dependencies"][name]
        if isinstance(specification, dict) and "path" in specification:
            if specification["path"] not in OWNED_TREES:
                raise ValueError(f"Inherited dependency escapes headless scope: {name}")
    lines = ["# Generated supplemental headless workspace; not native Zed acceptance.",
             "[workspace]", f"resolver = {json.dumps(workspace['resolver'])}",
             "members = " + json.dumps([f"crates/{name}" for name in PACKAGES]),
             "", "[workspace.package]"]
    lines += raw_entries(source, "workspace.package", workspace["package"])
    lines += ["", "[workspace.dependencies]"]
    lines += raw_entries(source, "workspace.dependencies", dependencies)
    result = "\n".join(lines) + "\n"
    parsed = tomllib.loads(result)["workspace"]
    if parsed["package"] != workspace["package"] or parsed["dependencies"] != {
        name: workspace["dependencies"][name] for name in dependencies
    }:
        raise ValueError("Generated workspace inheritance differs from original")
    return result


def prepare(repository, cache, seed_lock):
    if cache == repository or cache.is_relative_to(repository) or repository.is_relative_to(cache):
        raise ValueError("Cache must be outside, and not contain, the source repository")
    marker = {"format": 1, "source_repository": str(repository)}
    marker_path = cache / MARKER
    if cache.exists():
        if not marker_path.is_file() or json.loads(marker_path.read_text()) != marker:
            raise ValueError(f"Refusing unowned or mismatched cache: {cache}")
    else:
        cache.mkdir(parents=True)
        marker_path.write_text(json.dumps(marker, indent=2) + "\n")
    generated_manifest = workspace_manifest(repository)
    source_hashes = file_hashes(repository)
    for relative in OWNED_TREES:
        destination = cache / relative
        # Resolve every destination before deletion so a tampered cache cannot
        # redirect removal through a symlink outside its owned root.
        if destination.resolve() != destination:
            raise ValueError(f"Refusing symlinked cache destination: {destination}")
        if destination.exists():
            shutil.rmtree(destination)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(repository / relative, destination)
    if source_hashes != file_hashes(cache) or source_hashes != file_hashes(repository):
        raise ValueError("Source changed during copy or copied bytes differ; rerun preparation")
    for name in ("Cargo.toml", "Cargo.lock", "provenance.json"):
        if (cache / name).is_symlink():
            raise ValueError(f"Refusing symlinked cache file: {name}")
    (cache / "Cargo.toml").write_text(generated_manifest)
    seeded = False
    if seed_lock and not (cache / "Cargo.lock").exists():
        shutil.copy2(repository / "Cargo.lock", cache / "Cargo.lock")
        seeded = True
    git = lambda *arguments: subprocess.check_output(
        ["git", "-C", str(repository), *arguments], text=True).strip()
    provenance = {
        "prepared_at_utc": datetime.now(timezone.utc).isoformat(),
        "scope": "isolated-headless-only; NOT native-workspace or GUI acceptance",
        "repository": str(repository), "parent_sha": git("rev-parse", "HEAD"),
        "git_status_porcelain": git("status", "--porcelain=v1", "--untracked-files=all"),
        "packages": list(PACKAGES), "source_sha256": source_hashes,
        "source_manifest_fingerprint": hashlib.sha256(json.dumps({
            name: value for name, value in source_hashes.items() if name.endswith("/Cargo.toml")
        }, sort_keys=True).encode()).hexdigest(),
        "source_root_manifest_sha256": digest(repository / "Cargo.toml"),
        "source_lock_sha256": digest(repository / "Cargo.lock"),
        "generated_manifest_sha256": digest(cache / "Cargo.toml"),
        "cache_lock_seeded_this_run": seeded,
        "omitted_root_configuration": ["patch", "profile", "lints", ".cargo", "default-members"],
        "toolchain": tomllib.loads((repository / "rust-toolchain.toml").read_text())["toolchain"]["channel"],
    }
    (cache / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    return provenance


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--cache-dir", required=True, type=Path)
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--target-dir", type=Path, help="Persistent Cargo target directory outside the repository (default: cache-dir/target)")
    parser.add_argument("--no-seed-lock", action="store_true", help="Do not seed a missing cache lockfile from the repository")
    parser.add_argument("cargo_arguments", nargs=argparse.REMAINDER, help="Cargo arguments after -- (default: existing BDD integration targets)")
    options = parser.parse_args()
    repository = Path(__file__).resolve().parents[2]
    cache = options.cache_dir.expanduser().resolve()
    target = (options.target_dir or cache / "target").expanduser().resolve()
    if target == repository or target.is_relative_to(repository):
        parser.error("Target directory must be outside the source repository")
    arguments = options.cargo_arguments
    if arguments[:1] == ["--"]:
        arguments = arguments[1:]
    if any(argument.split("=", 1)[0] in {"--manifest-path", "--target-dir", "--config"} for argument in arguments):
        parser.error("Manifest, target directory and configuration overrides are not supported")
    if arguments[:1] == ["test"]:
        permitted = {
            ("test", "-p", package, "--test", target)
            for package, targets in BDD_TARGETS.items() for target in targets
        }
        if tuple(arguments) not in permitted:
            parser.error("Use test -p PACKAGE --test BDD_TARGET for one existing BDD target, or omit Cargo arguments to run all BDD targets")
    provenance = prepare(repository, cache, not options.no_seed_lock)
    print(f"Prepared {len(PACKAGES)} exact headless packages in {cache}", flush=True)
    print(f"Source provenance: {cache / 'provenance.json'}", flush=True)
    print("Supplemental validation only; no native Zed workspace/GUI acceptance.", flush=True)
    if options.prepare_only:
        return 0
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(target)
    environment["RUSTUP_TOOLCHAIN"] = provenance["toolchain"]
    commands = [["cargo", *arguments]] if arguments else [
        ["cargo", "test", "-p", package, *[
            argument for target in targets for argument in ("--test", target)
        ]]
        for package, targets in BDD_TARGETS.items()
    ]
    provenance["cargo_commands"] = commands
    provenance["cargo_working_directory"] = str(cache)
    provenance["cargo_target_directory"] = str(target)
    provenance["cargo_exit_code"] = 0
    for command in commands:
        provenance["cargo_exit_code"] = subprocess.call(command, cwd=cache, env=environment)
        if provenance["cargo_exit_code"] != 0:
            break
    provenance["cache_lock_sha256_after_cargo"] = digest(cache / "Cargo.lock") if (cache / "Cargo.lock").exists() else None
    provenance["copied_sources_unchanged_after_cargo"] = file_hashes(cache) == provenance["source_sha256"]
    (cache / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    if not provenance["copied_sources_unchanged_after_cargo"]:
        raise ValueError("Cargo invocation modified copied input files")
    return provenance["cargo_exit_code"]


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"bellatrix_headless: {error}", file=sys.stderr)
        sys.exit(1)
