"""API26 Vulkan/AAudio application contract, distinct from API24 CPU archives."""
import re
import shlex
from pathlib import Path
import android_archive
import native_wasm
from native_package import read, require

API = 26
FEATURES = ['mobile-ffi', 'mobile-android', 'project']
JNI = tuple('Java_dev_incant_player_NativeBridge_' + name for name in
            ('open', 'close', 'active', 'surface', 'frame', 'touch'))
MAX_PACKAGE = 512 * 1024**2


def preflight(project, scene):
    require(native_wasm.selection(project, scene) is None,
            'API26 device host currently supports TypeScript only; use the API24 archive route for static Wasm')
    android_archive.preflight_project(project, scene)


def toolchain(root):
    ndk = android_archive.toolchain(root)
    for name, suffix in [('clang', 'clang'), ('clangxx', 'clang++')]:
        ndk[name] = ndk['llvm'] / f'bin/aarch64-linux-android{API}-{suffix}'
        require(ndk[name].is_file(), 'Missing API26 compiler')
    return ndk


def environment(source, rustc, project, scene, ndk):
    for name in source:
        require(not name.startswith(('CARGO_FEATURE_', 'CXXFLAGS', 'CPPFLAGS')),
                'Unset host compiler/feature override: ' + name)
    env, selected = android_archive.environment(source, rustc, project, scene, ndk)
    key = 'BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android'
    selected[key] = shlex.join([f'--target=aarch64-linux-android{API}',
                               '--sysroot=' + str(ndk['sysroot']), '-resource-dir=' + str(ndk['resource'])])
    env.update(selected)
    env['CARGO_BUILD_JOBS'] = '2'
    env['RAYON_NUM_THREADS'] = '2'
    return env, selected


def graph(text):
    names = android_archive.normal_dependencies(text)
    require(not set(names) & {'android-activity', 'ndk-glue', 'tao', 'gilrs'},
            'Conflicting Android context/window/input backend')
    for package in ('ndk', 'ndk-context', 'jni', 'cpal', 'wgpu', 'incant_render_gpu_host'):
        require(package in names, 'Missing real Android host dependency: ' + package)
    return names


def compiler(ndk, execute):
    macros = execute([ndk['clang'], '-dM', '-E', '-x', 'c', '/dev/null']).stdout
    require('#define __ANDROID_MIN_SDK_VERSION__ 26' in macros
            and '#define __aarch64__ 1' in macros, 'Wrong Android compiler API or architecture')
    return macros


def index_files(data):
    rows = []
    total = 0
    for path in sorted(data.rglob('*')):
        if path.is_dir() and not path.is_symlink():
            continue
        name = path.relative_to(data).as_posix()
        require(re.fullmatch(r'[a-zA-Z0-9_-]+(?:/[a-zA-Z0-9_-]+){0,3}(?:\.[a-zA-Z0-9_-]+)?', name),
                'Android asset path unsupported')
        payload = read(path, 128 * 1024**2)
        # Java boundedLine excludes the newline and accepts exactly 512 ASCII chars.
        require(64 + 2 + len(str(len(payload))) + len(name) <= 512,
                'Android asset catalog line exceeds 512 characters')
        total += len(payload)
        require(total <= MAX_PACKAGE and len(rows) < 8192, 'Android package exceeds bounds')
        rows.append((name, payload))
    require(any(name == 'package.json' for name, _ in rows), 'Missing package.json')
    return rows


def java_tools(sdk, java):
    paths = {name: sdk / 'build-tools/36.0.0' / name for name in ('d8', 'aapt2', 'zipalign')}
    paths.update(android=sdk / 'platforms/android-36/android.jar', javac=java / 'bin/javac')
    require(all(p.is_file() for p in paths.values()), 'Android SDK36/build-tools36 or JDK tools missing')
    return paths
