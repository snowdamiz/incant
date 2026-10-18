"""Bounded source-only capture using held, non-following directory handles."""
import hashlib
import os
from pathlib import Path, PurePosixPath
import re
import stat

from rust_guest_sandbox import absolute, require

MAX_FILES = 32
MAX_FILE = 64 * 1024
MAX_TOTAL = 256 * 1024
MAX_DEPTH = 8
NAME = re.compile(r'[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}\Z')


def relative_entry(entry):
    require(isinstance(entry, str), 'Entry must be a relative Rust source name')
    path = PurePosixPath(entry)
    require(entry == path.as_posix() and not path.is_absolute() and path.parts
            and len(path.parts) <= MAX_DEPTH and '..' not in path.parts
            and all(NAME.fullmatch(part) for part in path.parts) and path.suffix == '.rs'
            and path.name.casefold() != 'build.rs', 'Invalid relative Rust entry')
    return path


def identity(info):
    return (info.st_dev, info.st_ino, info.st_mode, info.st_nlink, info.st_size,
            info.st_mtime_ns, info.st_ctime_ns)


def open_directory(root):
    """Hold every traversal, so an ancestor replacement cannot insert a link."""
    root = absolute(root)
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_NONBLOCK
    fd = os.open('/', flags)
    try:
        for part in root.parts[1:]:
            next_fd = os.open(part, flags, dir_fd=fd)
            os.close(fd)
            fd = next_fd
        return fd
    except BaseException:
        os.close(fd)
        raise


def capture(root, entry):
    """Returns immutable bytes, never forwards caller paths to the compiler.

    A source writer may race capture. Each file must remain identical while
    read, and every held directory is checked for membership changes. A later
    mutation cannot affect the exact bytes already captured.
    """
    root = absolute(root)
    entry = relative_entry(entry)
    require(hasattr(os, 'O_NOFOLLOW') and os.open in os.supports_dir_fd,
            'Non-following source capture unsupported')
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_NONBLOCK
    fd = open_directory(root)
    files = {}
    total = 0
    entries = 0
    try:
        def walk(directory, prefix, depth):
            nonlocal total, entries
            require(depth <= MAX_DEPTH, 'Source depth exceeded')
            before = identity(os.fstat(directory))
            with os.scandir(directory) as scan:
                # Bounded iteration before sorting; no unbounded listdir/read.
                names = []
                for item in scan:
                    entries += 1
                    require(entries <= 128, 'Source inventory exceeded')
                    require(NAME.fullmatch(item.name), 'Unsupported source path name')
                    names.append(item.name)
            for name in sorted(names):
                path = prefix / name
                info = os.stat(name, dir_fd=directory, follow_symlinks=False)
                if stat.S_ISDIR(info.st_mode):
                    child = os.open(name, flags, dir_fd=directory)
                    try:
                        require(identity(os.fstat(child)) == identity(info), 'Source directory replaced')
                        walk(child, path, depth + 1)
                    finally:
                        os.close(child)
                else:
                    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1,
                            'Source must contain only unlinked regular Rust files')
                    require(path.suffix == '.rs' and name.casefold() != 'build.rs',
                            'Cargo manifests, build scripts and non-Rust inputs are unsupported')
                    require(len(files) < MAX_FILES and info.st_size <= MAX_FILE, 'Source file bound exceeded')
                    handle = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=directory)
                    try:
                        require(identity(os.fstat(handle)) == identity(info), 'Source file replaced')
                        data = bytearray()
                        while len(data) <= MAX_FILE:
                            chunk = os.read(handle, min(8192, MAX_FILE + 1 - len(data)))
                            if not chunk:
                                break
                            data.extend(chunk)
                        require(len(data) <= MAX_FILE and identity(os.fstat(handle)) == identity(info),
                                'Source changed or exceeded bound while read')
                    finally:
                        os.close(handle)
                    require(len(data) == info.st_size, 'Source length changed')
                    bytes(data).decode('utf-8')  # Compiler diagnostics never need binary project inputs.
                    total += len(data)
                    require(total <= MAX_TOTAL, 'Source total bound exceeded')
                    files[path.as_posix()] = bytes(data)
            require(identity(os.fstat(directory)) == before, 'Source directory changed while captured')

        walk(fd, PurePosixPath(), 1)
    finally:
        os.close(fd)
    require(entry.as_posix() in files, 'Entry is absent from captured source')
    return files


def materialize(files, destination):
    destination.mkdir(mode=0o700)
    rows = []
    for name, data in sorted(files.items()):
        path = destination / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(data)
        path.chmod(0o400)
        rows.append({'path': name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
    for directory, _, _ in os.walk(destination, topdown=False):
        Path(directory).chmod(0o500)
    return rows
