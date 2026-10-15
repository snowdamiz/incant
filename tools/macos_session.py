"""Read-only preflight for explicitly requested native-window checks on macOS.

IOConsoleLocked supplies an explicit lock state. Some unlocked macOS sessions
omit CGSSessionScreenIsLocked; its absence is not a positive lock indication.
Require a completed console login for the calling user as well. This is a test
launcher preflight, not an authentication boundary or a guarantee against the
screen locking after the query. No app or capture API is invoked here.
"""
import os
import plistlib
import subprocess
import sys


def admit(value, uid):
    def refuse():
        raise ValueError('Native window requires an unlocked local console session')

    # ioreg's selected-root plist can be a dictionary or a one-root array.
    if isinstance(value, list) and len(value) == 1:
        value = value[0]
    if not isinstance(value, dict):
        refuse()
    root = value
    if root.get('IOConsoleLocked') is not False:
        refuse()
    users = root.get('IOConsoleUsers')
    if not isinstance(users, list) or not users or any(not isinstance(u, dict) for u in users):
        refuse()
    if any(type(u.get('kCGSSessionOnConsoleKey')) is not bool for u in users):
        refuse()
    current = [u for u in users if u['kCGSSessionOnConsoleKey']]
    if len(current) != 1:
        refuse()
    user = current[0]
    if (type(user.get('kCGSSessionUserIDKey')) is not int
            or user['kCGSSessionUserIDKey'] != uid
            or user.get('kCGSessionLoginDoneKey') is not True):
        refuse()
    # Explicit contradictory or malformed secondary state still refuses.
    for node in (root, user):
        if 'CGSSessionScreenIsLocked' in node and node['CGSSessionScreenIsLocked'] is not False:
            refuse()
    return {'console_unlocked': True, 'own_console_login_complete': True,
            'secondary_lock_field_present': any('CGSSessionScreenIsLocked' in n for n in (root, user))}


def query():
    if sys.platform != 'darwin':
        raise ValueError('macOS session preflight requires macOS')
    result = subprocess.run(['/usr/sbin/ioreg', '-a', '-n', 'Root', '-d', '1'],
                            capture_output=True, timeout=5, check=False)
    if result.returncode or result.stderr or len(result.stdout) > 4 * 1024**2:
        raise ValueError('Session metadata query failed')
    return admit(plistlib.loads(result.stdout), os.getuid())
