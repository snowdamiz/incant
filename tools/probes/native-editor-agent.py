#!/usr/bin/env python3
"""Opt-in local live account/native editor acceptance on a disposable project.

Build incant_editor with custom-protocol,agent-probe and incant_headless first.
Uses the existing saved connection without changing it; four real model turns
consume the account's plan usage. No credentials or account labels are recorded.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import native_process


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--editor', type=Path, required=True)
    parser.add_argument('--headless', type=Path, required=True)
    parser.add_argument('--model', required=True)
    parser.add_argument('--live', action='store_true', required=True,
                        help='Explicitly use the saved real provider connection')
    args = parser.parse_args()
    output, editor, headless = args.output.resolve(), args.editor.resolve(), args.headless.resolve()
    output.mkdir(parents=True, exist_ok=False)
    project = output / 'project.json'
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()

    def run(command, timeout=180):
        result = native_process.run(command, cwd=ROOT, timeout=timeout)
        # Never propagate captured provider/model/browser output into evidence.
        require(result.returncode == 0, f'Owned probe exited with code {result.returncode}; inspect its fixed-diagnostic receipt')

    run([str(headless), 'init', str(project), '--entities', '0', '--name', 'Native live agent acceptance'])
    run([sys.executable, str(ROOT / 'tools/probes/native-editor-play.py'), str(output),
         '--editor', str(editor), '--headless', str(headless), '--author'])
    inputs = {name: digest(output / name) for name in ['project.json', 'move.ts']}
    first_receipt, reopen_receipt = output / 'live.json', output / 'reopen.json'
    run([str(editor), str(project), '--agent-probe', str(first_receipt), args.model], timeout=800)
    first = json.loads(first_receipt.read_text())
    require(first.get('passed') is True, 'Live native acceptance did not pass')
    require(inputs == {name: digest(output / name) for name in inputs}, 'Probe changed saved project or source bytes')
    run([str(editor), str(project), '--agent-probe-restore', str(reopen_receipt), args.model], timeout=90)
    reopened = json.loads(reopen_receipt.read_text())
    require(reopened.get('passed') is True and reopened.get('provider_turns') == 0, 'Saved connection did not recover on reopening')
    history = first.get('journal_after_undo')
    require(isinstance(history, dict) and history.get('redo') == 2, 'Expected agent Undo history is absent')
    require(reopened.get('journal_after_undo') == history, 'Fresh editor did not recover the same journal revision and transaction IDs')
    usage = first.get('budget_after_undo')
    require(isinstance(usage, dict) and usage.get('used_tokens', 0) > 0,
            'Durable project usage was not measured')
    require(reopened.get('budget_after_undo') == usage
            and reopened.get('accounting_uncertain') is False,
            'Fresh editor did not recover exact project usage')
    require(inputs == {name: digest(output / name) for name in inputs}, 'Reopen changed saved project or source bytes')
    evidence = {'passed': True, 'live': first, 'reopen': reopened, 'authored_sha256': inputs,
                'editor_sha256': digest(editor), 'headless_sha256': digest(headless),
                'journal_sha256_after_undo': digest(output / 'project.journal.jsonl'),
                'fixture_authored_through_incant_cmd': True,
                'saved_project_and_sources_unchanged': True,
                'journal_revision_and_transaction_ids_recovered': True,
                'project_usage_recovered_after_undo_and_reopen': True,
                'scope': 'Local real provider/native handler acceptance, not WebView dispatch, rendered pixels or a phase gate'}
    (output / 'result.json').write_text(json.dumps(evidence, indent=2) + '\n')
    print(json.dumps({'passed': True, 'evidence': str(output / 'result.json')}))


if __name__ == '__main__':
    main()
