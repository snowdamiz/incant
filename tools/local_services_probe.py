"""Actual local protocols, using only disposable names and never printing tokens."""
from __future__ import annotations

import base64
import datetime as dt
from email.message import EmailMessage
import hashlib
import hmac
import json
import secrets
import smtplib
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())


def request(url, method='GET', body=None, headers=None):
    req = urllib.request.Request(url, body, headers or {}, method=method)
    try:
        with HTTP.open(req, timeout=20) as response:
            return response.status, response.headers, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.headers, error.read()


def require(value, message):
    if not value:
        raise RuntimeError('Local protocol proof failed: ' + message)


def signed_s3(stack, method, path, payload=b'', query='', secret=None):
    now = dt.datetime.now(dt.UTC)
    date, stamp = now.strftime('%Y%m%d'), now.strftime('%Y%m%dT%H%M%SZ')
    host = urllib.parse.urlsplit(stack.url('S3')).netloc
    digest = hashlib.sha256(payload).hexdigest()
    canonical_headers = f'host:{host}\nx-amz-content-sha256:{digest}\nx-amz-date:{stamp}\n'
    signed_headers = 'host;x-amz-content-sha256;x-amz-date'
    canonical = '\n'.join([method, path, query, canonical_headers, signed_headers, digest])
    scope = date + '/us-east-1/s3/aws4_request'
    string = 'AWS4-HMAC-SHA256\n' + stamp + '\n' + scope + '\n' + hashlib.sha256(canonical.encode()).hexdigest()
    key = ('AWS4' + (secret or stack.values['SECRET_KEY'])).encode()
    for item in (date, 'us-east-1', 's3', 'aws4_request'):
        key = hmac.new(key, item.encode(), hashlib.sha256).digest()
    signature = hmac.new(key, string.encode(), hashlib.sha256).hexdigest()
    authorization = f"AWS4-HMAC-SHA256 Credential={stack.values['ACCESS_KEY']}/{scope}, SignedHeaders={signed_headers}, Signature={signature}"
    return request(stack.url('S3') + path + ('?' + query if query else ''), method, payload,
                   {'Authorization': authorization, 'x-amz-date': stamp, 'x-amz-content-sha256': digest})


def postgres(stack, suffix):
    table = 'local_probe_' + suffix
    sql = f'''CREATE TABLE {table}(id integer PRIMARY KEY, value text NOT NULL);
BEGIN; INSERT INTO {table} VALUES (1, 'committed'); COMMIT;
BEGIN; INSERT INTO {table} VALUES (2, 'rolled back'); ROLLBACK;
SELECT string_agg(id::text || ':' || value, ',' ORDER BY id) FROM {table};
DROP TABLE {table};'''
    output = stack.compose('exec', '-T', 'postgres', 'sh', '-c',
        'PGPASSWORD="$POSTGRES_PASSWORD" psql -h gateway -p 15432 -U incant -d incant -v ON_ERROR_STOP=1 -At', input=sql)
    require('1:committed' in output.splitlines() and '2:rolled back' not in output, 'Postgres commit/rollback')
    try:
        stack.compose('exec', '-T', 'postgres', 'sh', '-c',
            'PGPASSWORD=deliberately-invalid psql -h gateway -p 15432 -U incant -d incant -w -At -c "SELECT 1"')
    except RuntimeError as error:
        require('password authentication failed' in str(error), 'Postgres wrong-password error identity')
    else:
        require(False, 'Postgres accepted a wrong password')
    return ['postgres_tcp_commit', 'postgres_tcp_rollback', 'postgres_wrong_password_refused']


def objects(stack, suffix):
    bucket = '/incant-probe-' + suffix
    key = bucket + '/exact-bytes.bin'
    payload = bytes(range(256)) + b'Incant local object protocol\x00\xff'
    require(signed_s3(stack, 'PUT', bucket)[0] == 200, 'create S3 bucket')
    try:
        require(signed_s3(stack, 'PUT', key, payload)[0] == 200, 'put S3 object')
        status, _, data = signed_s3(stack, 'GET', key)
        require(status == 200 and data == payload, 'get exact S3 bytes')
        status, _, data = signed_s3(stack, 'GET', bucket, query='list-type=2')
        require(status == 200 and any(e.text == 'exact-bytes.bin' for e in ET.fromstring(data).iter() if e.tag.endswith('Key')), 'list S3 object')
        require(signed_s3(stack, 'GET', key, secret='deliberately-invalid')[0] == 403, 'wrong S3 signature refused')
        require(request(stack.url('S3') + key, 'PUT', b'forbidden')[0] in (401, 403), 'anonymous S3 write refused')
        require(signed_s3(stack, 'DELETE', key)[0] in (200, 204), 'delete S3 object')
        require(signed_s3(stack, 'GET', key)[0] == 404, 'missing S3 object refused')
    finally:
        signed_s3(stack, 'DELETE', key)
        signed_s3(stack, 'DELETE', bucket)
    return ['s3_create_put_exact_get_list_delete', 's3_wrong_signature_refused', 's3_anonymous_write_refused', 's3_missing_key_refused']


def mail(stack, suffix):
    subject = 'Incant local capture ' + suffix
    message = EmailMessage()
    message['From'], message['To'], message['Subject'] = 'local@incant.invalid', 'probe@incant.invalid', subject
    message.set_content('Local-only protocol proof; no external delivery.')
    with smtplib.SMTP('127.0.0.1', int(stack.values['SMTP_PORT']), timeout=10) as smtp:
        smtp.send_message(message)
    status, _, raw = request(stack.url('MAIL') + '/api/v1/messages')
    require(status == 200, 'mail capture listing')
    found = [m for m in json.loads(raw)['messages'] if m['Subject'] == subject]
    require(len(found) == 1, 'SMTP message captured once')
    ident = found[0]['ID']
    try:
        status, _, raw = request(stack.url('MAIL') + '/api/v1/message/' + ident)
        require(status == 200 and json.loads(raw)['Text'].replace('\r\n', '\n') == message.get_content(), 'exact captured SMTP body')
    finally:
        status, _, _ = request(stack.url('MAIL') + '/api/v1/messages', 'DELETE', json.dumps({'IDs': [ident]}).encode(), {'Content-Type': 'application/json'})
        require(status in (200, 204), 'delete only probe mail')
    return ['smtp_local_capture_exact_body']


class Oidc:
    def __init__(self, stack):
        self.base = stack.url('OIDC')
        path = stack.state / 'bootstrap/admin.pat'
        if path.is_symlink():
            raise ValueError('Local bootstrap token cannot be a symlink')
        self.admin = path.read_text().strip()

    def api(self, path, body=None, *, token=None, method=None, expected=(200, 201)):
        status, _, raw = request(self.base + path, method or ('POST' if body is not None else 'GET'),
            json.dumps(body).encode() if body is not None else None,
            {'Authorization': 'Bearer ' + (token or self.admin), 'Content-Type': 'application/json'})
        require(status in expected, 'OIDC API ' + path.split('?')[0] + ' returned HTTP ' + str(status))
        return json.loads(raw) if raw else {}


def oidc(stack, suffix):
    c = Oidc(stack)
    discovery = json.loads(request(c.base + '/.well-known/openid-configuration')[2])
    require(discovery['issuer'] == c.base and 'S256' in discovery['code_challenge_methods_supported'], 'local OIDC issuer and PKCE discovery')
    for key in ['authorization_endpoint', 'token_endpoint', 'userinfo_endpoint', 'jwks_uri']:
        require(discovery[key].startswith(c.base + '/'), 'OIDC endpoint escaped local issuer')
    project = user = machine = None
    try:
        project = c.api('/management/v1/projects', {'name': 'incant-probe-' + suffix})['id']
        redirect = 'http://127.0.0.1:18765/callback'
        app = c.api(f'/management/v1/projects/{project}/apps/oidc', {
            'name': 'local-native-pkce', 'redirectUris': [redirect],
            'responseTypes': ['OIDC_RESPONSE_TYPE_CODE'], 'grantTypes': ['OIDC_GRANT_TYPE_AUTHORIZATION_CODE'],
            'appType': 'OIDC_APP_TYPE_NATIVE', 'authMethodType': 'OIDC_AUTH_METHOD_TYPE_NONE',
            'version': 'OIDC_VERSION_1_0', 'devMode': True, 'skipNativeAppSuccessPage': True,
            # Probe performs the login API flow directly; this is not a shipped login UI.
            'loginVersion': {'loginV2': {'baseUri': c.base + '/local-probe-login'}}})
        client = app['clientId']
        password = 'Local1!' + secrets.token_hex(24)
        user = c.api('/v2/users/human', {'username': 'probe-' + suffix,
            'profile': {'givenName': 'Local', 'familyName': 'Probe'},
            'email': {'email': 'probe-' + suffix + '@incant.invalid', 'isVerified': True},
            'password': {'password': password, 'changeRequired': False}})['userId']
        machine = c.api('/management/v1/users/machine', {'userName': 'login-' + suffix, 'name': 'Disposable local login probe'})['userId']
        c.api('/admin/v1/members', {'userId': machine, 'roles': ['IAM_LOGIN_CLIENT']})
        expiry = (dt.datetime.now(dt.UTC) + dt.timedelta(minutes=10)).strftime('%Y-%m-%dT%H:%M:%SZ')
        login = c.api(f'/management/v1/users/{machine}/pats', {'expirationDate': expiry})['token']
        session = c.api('/v2/sessions', {'checks': {'user': {'userId': user}, 'password': {'password': password}}, 'lifetime': '300s'}, token=login)
        status, _, _ = request(c.base + '/v2/sessions', 'POST', json.dumps({'checks': {
            'user': {'userId': user}, 'password': {'password': 'deliberately-invalid'}}}).encode(),
            {'Authorization': 'Bearer ' + login, 'Content-Type': 'application/json'})
        require(status in (400, 401, 403), 'OIDC wrong password refused')
        def authorize(verifier):
            challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b'=').decode()
            nonce, state = secrets.token_urlsafe(24), secrets.token_urlsafe(24)
            query = {'client_id': client, 'response_type': 'code', 'scope': 'openid profile email',
                'redirect_uri': redirect, 'state': state, 'nonce': nonce,
                'code_challenge': challenge, 'code_challenge_method': 'S256'}
            status, headers, _ = request(discovery['authorization_endpoint'] + '?' + urllib.parse.urlencode(query))
            require(status == 302, 'OIDC authorization redirect')
            location = urllib.parse.urlsplit(headers['Location'])
            require(location.netloc == urllib.parse.urlsplit(c.base).netloc, 'login redirect escaped local issuer')
            auth = urllib.parse.parse_qs(location.query)['authRequest'][0]
            callback = c.api('/v2/oidc/auth_requests/' + auth, {'session': {
                'sessionId': session['sessionId'], 'sessionToken': session['sessionToken']}}, token=login)['callbackUrl']
            uri = urllib.parse.urlsplit(callback)
            require(urllib.parse.urlunsplit((uri.scheme, uri.netloc, uri.path, '', '')) == redirect, 'OIDC exact callback')
            values = urllib.parse.parse_qs(uri.query)
            require(values['state'] == [state] and 'code' in values, 'OIDC callback state/code')
            return values['code'][0], nonce
        def exchange(code, verifier):
            body = urllib.parse.urlencode({'grant_type': 'authorization_code', 'client_id': client,
                'redirect_uri': redirect, 'code': code, 'code_verifier': verifier}).encode()
            status, _, raw = request(discovery['token_endpoint'], 'POST', body, {'Content-Type': 'application/x-www-form-urlencoded'})
            return status, json.loads(raw)
        bad_verifier = secrets.token_urlsafe(48)
        bad_code, _ = authorize(bad_verifier)
        status, body = exchange(bad_code, secrets.token_urlsafe(48))
        require(status == 400 and body['error'] == 'invalid_grant', 'OIDC wrong PKCE verifier refused')
        verifier = secrets.token_urlsafe(48)
        code, nonce = authorize(verifier)
        status, tokens = exchange(code, verifier)
        require(status == 200 and tokens['token_type'] == 'Bearer', 'OIDC PKCE token exchange')
        parts = tokens['id_token'].split('.')
        claims = json.loads(base64.urlsafe_b64decode(parts[1] + '=' * (-len(parts[1]) % 4)))
        header = json.loads(base64.urlsafe_b64decode(parts[0] + '=' * (-len(parts[0]) % 4)))
        keys = json.loads(request(discovery['jwks_uri'])[2])['keys']
        require(any(k['kid'] == header['kid'] for k in keys), 'issued ID token references published signing key')
        require(claims['iss'] == c.base and claims['sub'] == user and claims['nonce'] == nonce
            and client in ([claims['aud']] if isinstance(claims['aud'], str) else claims['aud']), 'OIDC issuer/subject/audience/nonce')
        status, _, raw = request(discovery['userinfo_endpoint'], headers={'Authorization': 'Bearer ' + tokens['access_token']})
        require(status == 200 and json.loads(raw)['sub'] == user, 'provider accepted issued access token')
        status, body = exchange(code, verifier)
        require(status == 400 and body['error'] in ('invalid_grant', 'invalid_request'), 'OIDC code replay refused (' + str(status) + '/' + str(body.get('error')) + ')')
        c.api('/v2/sessions/' + session['sessionId'], method='DELETE', token=login)
    finally:
        if project:
            c.api('/management/v1/projects/' + project, method='DELETE')
        for ident in [user, machine]:
            if ident:
                c.api('/management/v1/users/' + ident, method='DELETE')
    return ['oidc_discovery_local_issuer_jwks', 'oidc_pkce_code_exchange_userinfo',
            'oidc_state_nonce_audience_subject', 'oidc_wrong_password_refused',
            'oidc_wrong_pkce_refused', 'oidc_code_replay_refused']


def run(stack):
    stack.health()
    suffix = secrets.token_hex(8)
    checks = postgres(stack, suffix) + objects(stack, suffix) + mail(stack, suffix) + oidc(stack, suffix)
    return {'scope': 'local protocol fixtures only', 'checks': checks, 'count': len(checks),
            'hosted_gates_satisfied': False, 'id_token_signature_validation': 'not a client-library validation; provider userinfo validates the access token'}
