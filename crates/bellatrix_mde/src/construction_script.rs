use crate::Plan;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::path::Component;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConstructionScriptError {
    #[error("artifact path is not valid UTF-8: {0}")]
    NonUtf8Path(String),
}

/// Serializes an already-rendered plan without accessing the filesystem.
/// The returned Python script requires an explicit destination. On Windows, the
/// destination tree must not be modified concurrently while the script runs.
pub fn python_construction_script(plan: &Plan) -> Result<String, ConstructionScriptError> {
    let mut script = String::from(PYTHON_HEADER);
    for artifact in plan.artifacts() {
        let mut parts = Vec::new();
        for component in artifact.path.components() {
            if let Component::Normal(segment) = component {
                let segment = segment.to_str().ok_or_else(|| {
                    ConstructionScriptError::NonUtf8Path(artifact.path.display().to_string())
                })?;
                parts.push(format!("'{}'", STANDARD.encode(segment.as_bytes())));
            }
        }
        let contents = match &artifact.contents {
            Some(bytes) => format!("'{}'", STANDARD.encode(bytes)),
            None => "None".to_owned(),
        };
        script.push_str(&format!("    ([{}], {}),\n", parts.join(", "), contents));
    }
    script.push_str(PYTHON_FOOTER);
    Ok(script)
}

const PYTHON_HEADER: &str = r#"#!/usr/bin/env python3
import base64
import os
import pathlib
import stat
import sys

ARTIFACTS = [
"#;

const PYTHON_FOOTER: &str = r#"]

def decode(value):
    return base64.b64decode(value, validate=True)

def open_directory(parent, name):
    try:
        os.mkdir(name, dir_fd=parent)
    except FileExistsError:
        pass
    return os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent)

def write_file(parent, name, contents):
    try:
        existing = os.stat(name, dir_fd=parent, follow_symlinks=False)
    except FileNotFoundError:
        existing = None
    if existing is not None and not stat.S_ISREG(existing.st_mode):
        raise ValueError('destination is not a regular file: ' + name)
    temporary = None
    for index in range(10000):
        candidate = '.bellatrix-tmp-' + str(index)
        if candidate == name:
            continue
        try:
            descriptor = os.open(candidate, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o666, dir_fd=parent)
            temporary = candidate
            break
        except FileExistsError:
            continue
    if temporary is None:
        raise FileExistsError('no temporary file name available')
    try:
        with os.fdopen(descriptor, 'wb') as output:
            output.write(contents)
        os.replace(temporary, name, src_dir_fd=parent, dst_dir_fd=parent)
    finally:
        try:
            os.unlink(temporary, dir_fd=parent)
        except FileNotFoundError:
            pass

def windows_check(path, directory):
    info = os.lstat(path)
    attributes = getattr(info, 'st_file_attributes', None)
    if attributes is None:
        raise RuntimeError('Windows reparse-point checks are unavailable')
    if attributes & 0x400:
        raise ValueError('reparse points are not allowed: ' + str(path))
    if directory and not stat.S_ISDIR(info.st_mode):
        raise ValueError('destination is not a directory: ' + str(path))
    if not directory and not stat.S_ISREG(info.st_mode):
        raise ValueError('destination is not a regular file: ' + str(path))

def windows_mkdir(path):
    try:
        os.mkdir(path)
    except FileExistsError:
        pass
    windows_check(path, True)

def windows_main(root):
    # Windows Python has no equivalent of POSIX dir_fd traversal. The caller
    # must prevent other processes from changing this tree during execution.
    print('Windows: output tree must not be modified concurrently.', file=sys.stderr)
    root = pathlib.Path(os.path.abspath(root))
    parents = list(reversed(root.parents))
    for parent in parents:
        windows_check(parent, True)
    windows_mkdir(root)
    for encoded_parts, encoded_contents in ARTIFACTS:
        parts = [decode(part).decode('utf-8') for part in encoded_parts]
        current = root
        directories = parts if encoded_contents is None else parts[:-1]
        for part in directories:
            current = current / part
            windows_mkdir(current)
        if encoded_contents is None:
            continue
        destination = current / parts[-1]
        try:
            windows_check(destination, False)
        except FileNotFoundError:
            pass
        temporary = None
        for index in range(10000):
            candidate = current / ('.bellatrix-tmp-' + str(index))
            if candidate == destination:
                continue
            try:
                descriptor = os.open(candidate, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_BINARY, 0o666)
                temporary = candidate
                break
            except FileExistsError:
                continue
        if temporary is None:
            raise FileExistsError('no temporary file name available')
        try:
            with os.fdopen(descriptor, 'wb') as output:
                output.write(decode(encoded_contents))
            os.replace(temporary, destination)
        finally:
            try:
                os.unlink(temporary)
            except FileNotFoundError:
                pass

def main():
    if len(sys.argv) != 2:
        raise SystemExit('usage: construction.py OUTPUT_ROOT')
    if not sys.argv[1]:
        raise SystemExit('OUTPUT_ROOT must not be empty')
    if os.name == 'nt':
        windows_main(sys.argv[1])
        return
    if os.name != 'posix' or not hasattr(os, 'O_NOFOLLOW') or not hasattr(os, 'O_DIRECTORY'):
        raise SystemExit('unsupported platform: POSIX directory file descriptors are required')
    root = os.path.abspath(sys.argv[1])
    root_parts = [part for part in root.split(os.sep) if part]
    root_fd = os.open(os.sep, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for index, part in enumerate(root_parts):
            if index == len(root_parts) - 1:
                child = open_directory(root_fd, part)
            else:
                child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=root_fd)
            os.close(root_fd)
            root_fd = child
    except BaseException:
        os.close(root_fd)
        raise
    try:
        for encoded_parts, encoded_contents in ARTIFACTS:
            parts = [decode(part).decode('utf-8') for part in encoded_parts]
            current = os.dup(root_fd)
            try:
                directories = parts if encoded_contents is None else parts[:-1]
                for part in directories:
                    child = open_directory(current, part)
                    os.close(current)
                    current = child
                if encoded_contents is not None:
                    write_file(current, parts[-1], decode(encoded_contents))
            finally:
                os.close(current)
    finally:
        os.close(root_fd)

if __name__ == '__main__':
    main()
"#;
