import importlib.util
from pathlib import Path
import struct
import unittest


def load(name):
    spec = importlib.util.spec_from_file_location(
        name, Path(__file__).parents[1] / 'probes/physics-parity' / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


probe = load('run')
hosts = load('compare_hosts')


class PhysicsParity(unittest.TestCase):
    def test_reports_velocity_quaternion_and_sleep_mismatches_after_initial_frame(self):
        # A position-only comparison must not accept these corruptions.
        baseline = bytearray(64 + 2 * 3 * 15 * 4)
        struct.pack_into('<I', baseline, 12, 3)
        for field, name in [(5, 'quaternion.y'), (9, 'linear_velocity.y'),
                            (13, 'angular_velocity.z'), (14, 'sleeping')]:
            changed = bytearray(baseline)
            struct.pack_into('<I', changed, 64 + (3 + 2) * 60 + field * 4, 1)
            with self.assertRaisesRegex(ValueError, f'frame 1, body 2, {name}'):
                probe.compare(baseline, changed)

    def test_rejects_missing_frames_and_unexercised_sleep_or_joint_scenarios(self):
        for scenario, bodies, joints in [(0, 273, 0), (1, 217, 192)]:
            data = bytearray(64 + bodies * 641 * 60)
            struct.pack_into('<16I', data, 0, 0x50415231, 1, scenario, bodies, 641, 15,
                             0x3C800000, 0xC1000000, 20, 16, 16, joints, 0, 0, 0, 0)
            probe.inspect(data, scenario)
            with self.assertRaisesRegex(ValueError, 'missing frames'):
                probe.inspect(data[:-60], scenario)
            for word in ([8, 9, 10] if scenario == 0 else [8, 11]):
                changed = bytearray(data)
                struct.pack_into('<I', changed, word * 4, 0)
                with self.assertRaisesRegex(ValueError, 'coverage'):
                    probe.inspect(changed, scenario)

    def test_cross_host_check_requires_real_mode_coverage_and_browser_evidence(self):
        reports = []
        for system in ['Darwin', 'Linux', 'Windows']:
            browser = system == 'Linux'
            modes = [('serial', 0), ('parallel', 1), ('parallel', 4)]
            if browser:
                modes.append(('browser-wasm-serial', 0))
            reports.append({'host': {'system': system}, 'status': 'passed', 'trials': 3,
                            'browser_executed': browser, 'browser': [{}, {}, {}] if browser else [],
                            'reference': [{'scenario': s, 'sha256': str(s)} for s in (0, 1)],
                            'records': [dict(trial=t, mode=m, workers=w, scenario=s, sha256=str(s))
                                        for t in range(3) for m, w in modes for s in (0, 1)]})
        hosts.compare_hosts(reports)
        reports[1]['browser_executed'] = False
        browser_records = reports[1]['records']
        reports[1]['records'] = [r for r in browser_records if r['mode'] != 'browser-wasm-serial']
        with self.assertRaisesRegex(ValueError, 'browser/WASM execution'):
            hosts.compare_hosts(reports)
        reports[1]['browser_executed'] = True
        reports[1]['records'] = browser_records
        reports[0]['records'][0]['sha256'] = 'diverged'
        with self.assertRaisesRegex(ValueError, 'nonmatching'):
            hosts.compare_hosts(reports)
        reports[0]['records'].pop(0)
        with self.assertRaisesRegex(ValueError, 'missing or duplicated'):
            hosts.compare_hosts(reports)
        with self.assertRaisesRegex(ValueError, 'each desktop OS'):
            hosts.compare_hosts([reports[1]] * 3)


if __name__ == '__main__':
    unittest.main()
