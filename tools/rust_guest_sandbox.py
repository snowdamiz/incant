"""Mac-only deny-default policy for the explicit untrusted Rust build service.

Apple documents sandbox-exec as deprecated. This is a limited build backend,
not an App Sandbox, a portable security guarantee, or a fallback to bare rustc.
The actual compiler/escape controls must qualify each supported OS/toolchain.
"""
from dataclasses import dataclass
import ctypes
import errno
import os
from pathlib import Path
import stat
import sys


class Refused(ValueError):
    """Input, authority, resource or child refusal; no successful publication."""


class Unsupported(Refused):
    """The required isolation backend/toolchain is unavailable."""


def require(condition, message):
    if not condition:
        raise Refused(message)


def absolute(path):
    path = Path(path)
    require(path.is_absolute() and '..' not in path.parts, 'Absolute nonescaping host path required')
    # Do not normalize away a link supplied by the caller. All ancestors must
    # themselves be directories; /tmp callers must use /private/tmp on macOS.
    for part in reversed((path, *path.parents)):
        if part.exists() or part.is_symlink():
            require(not part.is_symlink(), 'Symbolic host paths are unsupported')
    return path


def trusted_file(path):
    path = absolute(path)
    info = path.stat(follow_symlinks=False)
    require(stat.S_ISREG(info.st_mode) and info.st_uid in (0, os.getuid())
            and not info.st_mode & 0o022, 'Trusted tool must be an owner-controlled regular file')
    require(info.st_mode & 0o111, 'Trusted tool is not executable')
    return path


def clean_environment(run, rustc):
    """No ambient credentials, wrappers, loader overrides, Cargo or rustup state."""
    return {'PATH': str(rustc.parent), 'HOME': str(run / 'home'),
            'TMPDIR': str(run / 'tmp') + '/', 'LANG': 'C', 'LC_ALL': 'C',
            'RAYON_NUM_THREADS': '1', 'SOURCE_DATE_EPOCH': '0'}


def _literal(path):
    # SBPL is Scheme, not shell/JSON. Reject controls and escape both syntax
    # characters; project names never become policy expressions.
    value = str(absolute(path))
    require(all(ord(char) >= 32 and ord(char) != 127 for char in value), 'Control in policy path')
    return '"' + value.replace('\\', '\\\\').replace('"', '\\"') + '"'


@dataclass(frozen=True)
class MacPolicy:
    rustc: Path
    linker: Path
    validator: Path
    sysroot: Path
    sdk: Path
    source: Path
    run: Path

    @staticmethod
    def backend():
        if sys.platform != 'darwin':
            raise Unsupported('Untrusted Rust compilation requires the experimental macOS backend')
        try:
            return trusted_file('/usr/bin/sandbox-exec')
        except (OSError, Refused) as error:
            raise Unsupported('Required sandbox-exec backend unavailable; no unrestricted fallback') from error

    def text(self):
        # No import of Apple's private system.sb (which can broaden policy).
        # Metadata only on traversed parents. Data is confined to these vetted
        # trees and public, OS-owned loader libraries; no /Users or /Library rule.
        reads = (self.sysroot, self.sdk, self.source, self.run,
                 Path('/usr/lib'), Path('/System/Library'))
        executables = (self.rustc, self.linker, self.validator)
        parents = sorted({parent for path in (*reads, *executables)
                          for parent in absolute(path).parents}, key=str)
        lines = ['(version 1)', '(deny default)', '(deny network*)', '(deny system-socket)',
                 '(allow process-fork)',
                 '(allow process-exec ' + ' '.join('(literal ' + _literal(p) + ')'
                                                    for p in executables) + ')',
                 '(allow sysctl-read (sysctl-name-regex #"^hw\\."))',
                 '(allow sysctl-read (sysctl-name "kern.osrelease") (sysctl-name "kern.osversion")'
                 ' (sysctl-name "kern.ostype"))']
        lines += ['(allow file-read* (subpath ' + _literal(path) + '))' for path in reads]
        # Exact executable bytes/mmap, including a separately admitted canary
        # outside these trees. Never grant its enclosing user directory data.
        lines += ['(allow file-read* (literal ' + _literal(path) + '))' for path in executables]
        # Executable mmap is a separate operation on current Apple profiles.
        # Source, SDK rlibs and writable run files get no executable mapping.
        code_libraries = (self.sysroot / 'lib', Path('/usr/lib'),
                          Path('/System/Library/Frameworks'), Path('/System/Library/PrivateFrameworks'),
                          Path('/System/Library/dyld'))
        lines += ['(allow file-map-executable (literal ' + _literal(path) + '))'
                  for path in executables]
        lines += ['(allow file-map-executable (subpath ' + _literal(path) + '))'
                  for path in code_libraries]
        lines += ['(allow file-read-metadata (literal ' + _literal(path) + '))' for path in parents]
        lines += ['(allow file-read* (literal "/dev/null") (literal "/dev/urandom"))',
                  '(allow file-write* (literal "/dev/null") (subpath ' + _literal(self.run) + '))']
        return '\n'.join(lines) + '\n'

    def command(self, policy_path, command):
        require(command and Path(command[0]) in (self.rustc, self.validator),
                'Only the admitted compiler or normal validator starts a build phase')
        return [str(self.backend()), '-f', str(policy_path), *map(str, command)]


class _TaskInfo(ctypes.Structure):
    # Public SDK sys/proc_info.h, struct proc_taskinfo / PROC_PIDTASKINFO=4.
    _fields_ = [(name, ctypes.c_uint64) for name in (
        'virtual_size', 'resident_size', 'total_user', 'total_system',
        'threads_user', 'threads_system')] + [(name, ctypes.c_int32) for name in (
        'policy', 'faults', 'pageins', 'cow_faults', 'messages_sent', 'messages_received',
        'syscalls_mach', 'syscalls_unix', 'csw', 'threadnum', 'numrunning', 'priority')]


def group_sample(group):
    """Bounded live group census and sampled RSS; unknown inspection fails closed."""
    if sys.platform != 'darwin':
        raise Unsupported('macOS process sampling required')
    api = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
    api.proc_listpgrppids.argtypes = (ctypes.c_int, ctypes.c_void_p, ctypes.c_int)
    api.proc_listpgrppids.restype = ctypes.c_int
    api.proc_pidinfo.argtypes = (ctypes.c_int, ctypes.c_int, ctypes.c_uint64, ctypes.c_void_p, ctypes.c_int)
    api.proc_pidinfo.restype = ctypes.c_int
    pids = (ctypes.c_int * 256)()
    ctypes.set_errno(0)
    count = api.proc_listpgrppids(group, pids, ctypes.sizeof(pids))
    require(0 <= count < len(pids) and not (count == 0 and ctypes.get_errno()),
            'Cannot inspect bounded owned group')
    rows = []
    for pid in pids[:count]:
        if pid <= 0:
            continue
        try:
            require(os.getpgid(pid) == group, 'Owned group identity changed')
        except ProcessLookupError:
            continue
        info = _TaskInfo()
        ctypes.set_errno(0)
        size = api.proc_pidinfo(pid, 4, 0, ctypes.byref(info), ctypes.sizeof(info))
        if size == 0 and ctypes.get_errno() == errno.ESRCH:
            continue
        require(size == ctypes.sizeof(info), 'Cannot inspect owned task RSS')
        rows.append({'pid': pid, 'pgid': group, 'rss': info.resident_size})
    return rows
