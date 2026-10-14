from pathlib import Path
import datetime, importlib.util, json, os, subprocess, sys, signal, threading, time
B = Path('/Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant')
C = B / '.worktrees/0073-upscaling-review'
E = Path(__file__).resolve().parent
p = C / 'handoffs/0073-upscaling-review/owner/owner_runner.py'
s = importlib.util.spec_from_file_location('life', p)
life = importlib.util.module_from_spec(s)
s.loader.exec_module(life)
assert life.sha(p) == 'b9e12895389a046e1c0a0bd4c43f01a215505c793be4106b31fafc18aa2aff34'
config = json.loads((B / 'tools/acp/config.json').read_text())
assert config['model'] == 'claude-opus-5-5' and config['model_display_name'] == 'Opus 5.5' and config['effort'] == 'max'
argv = [sys.executable, 'tools/handoff/main.py', 'run', '0073-upscaling-review', '--permission-mode', 'auto']
paths = subprocess.check_output(['git', 'ls-files', 'handoffs/0073-upscaling-review'], cwd=C, text=True).splitlines()
rows = [{'path': x, 'sha256': life.sha(C / x)} for x in paths]
untracked = subprocess.check_output(['git', 'ls-files', '--others', '--exclude-standard', 'handoffs/0073-upscaling-review'], cwd=C, text=True).splitlines()
wall, mono = time.time(), time.monotonic()
life.write_new_json(E / 'before.json', {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=C, text=True).strip(), 'protected_tracked': rows, 'protected_untracked': [{'path': x, 'sha256': life.sha(C / x)} for x in untracked], 'argv': argv, 'main_sha256': life.sha(B / 'tools/handoff/main.py'), 'config_sha256': life.sha(B / 'tools/acp/config.json'), 'wrapper_pid': os.getpid(), 'started_at': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'wall_epoch_seconds': wall, 'monotonic_seconds': mono, 'limit_seconds_each_clock': 900})
for sig in [signal.SIGTERM, signal.SIGHUP]:
    signal.signal(sig, life.raise_interrupt)
stop = threading.Event()
trigger = []
def watchdog():
    while not stop.wait(0.1):
        w, m = time.time() - wall, time.monotonic() - mono
        if max(w, m) >= 900:
            trigger.append({'wall_seconds': w, 'monotonic_seconds': m, 'reason': 'either-clock deadline'})
            os.kill(os.getpid(), signal.SIGTERM)
            return
with open(E / 'acp.log', 'x') as out, open(E / 'acp-stderr.log', 'x') as err:
    with life.blocked():
        thread = threading.Thread(target=watchdog, daemon=True)
        thread.start()
        try:
            env = os.environ.copy()
            env['CLAUDE_CODE_DISABLE_AUTO_MEMORY'] = '1'
            record, caught = life.launch(argv, B, env, out, err, 900)
        finally:
            stop.set()
            thread.join(timeout=1)
        record.update(argv=argv, auto_memory_environment={'CLAUDE_CODE_DISABLE_AUTO_MEMORY': '1'}, wall_elapsed_seconds=time.time() - wall, monotonic_elapsed_seconds=time.monotonic() - mono, watchdog=trigger, stdout_sha256=life.sha(E / 'acp.log'), stderr_sha256=life.sha(E / 'acp-stderr.log'))
        life.write_new_json(E / 'acp-terminal.json', record)
if caught:
    raise caught
print(json.dumps(record), flush=True)
raise SystemExit(record['exit'] if record['exit'] is not None else 1)
