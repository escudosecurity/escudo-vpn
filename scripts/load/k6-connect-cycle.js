import http from 'k6/http';
import { check, sleep } from 'k6';

const apiBase = (__ENV.API_BASE || '').replace(/\/+$/, '');
const testEmail = __ENV.TEST_EMAIL || '';
const testPassword = __ENV.TEST_PASSWORD || '';
const devicePrefix = __ENV.DEVICE_PREFIX || 'k6-device';

export const options = {
  vus: 3,
  iterations: 12,
  thresholds: {
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<2500'],
  },
};

function requireEnv(name, value) {
  if (!value) {
    throw new Error(`Missing required environment variable: ${name}`);
  }
}

function jsonHeaders(token) {
  const headers = { 'Content-Type': 'application/json' };
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }
  return headers;
}

function authenticate() {
  const payload = JSON.stringify({
    email: testEmail,
    password: testPassword,
  });

  const loginRes = http.post(
    `${apiBase}/api/v1/auth/login`,
    payload,
    { headers: jsonHeaders() },
  );
  if (loginRes.status === 200 && loginRes.json('token')) {
    return loginRes.json('token');
  }

  const registerRes = http.post(
    `${apiBase}/api/v1/auth/register`,
    payload,
    { headers: jsonHeaders() },
  );
  if (registerRes.status === 200) {
    return registerRes.json('token');
  }
  if (registerRes.status !== 409 && registerRes.status !== 429) {
    throw new Error(`Unexpected register status: ${registerRes.status} ${registerRes.body}`);
  }
  check(loginRes, {
    'login 200': (r) => r.status === 200,
    'login token present': (r) => !!r.json('token'),
  });
  return loginRes.json('token');
}

export default function () {
  requireEnv('API_BASE', apiBase);
  requireEnv('TEST_EMAIL', testEmail);
  requireEnv('TEST_PASSWORD', testPassword);

  const token = authenticate();
  const authHeaders = jsonHeaders(token);

  const serversRes = http.get(`${apiBase}/api/v1/servers`, { headers: authHeaders });
  check(serversRes, {
    'servers request ok': (r) => r.status === 200,
    'servers non-empty': (r) => Array.isArray(r.json()) && r.json().length > 0,
  });

  const servers = serversRes.json();
  const serverId = servers[0].id;

  const connectPayload = JSON.stringify({
    server_id: serverId,
    device_name: `${devicePrefix}-${__VU}-${__ITER}`,
  });
  const connectRes = http.post(
    `${apiBase}/api/v1/connect`,
    connectPayload,
    { headers: authHeaders },
  );

  check(connectRes, {
    'connect 200': (r) => r.status === 200,
    'device id present': (r) => !!r.json('device_id'),
    'wg config present': (r) => typeof r.json('config') === 'string' && r.json('config').includes('[Interface]'),
    'qr code present': (r) => typeof r.json('qr_code') === 'string' && r.json('qr_code').length > 32,
  });

  const deviceId = connectRes.json('device_id');
  if (!deviceId) {
    throw new Error(`Connect did not return device_id: ${connectRes.body}`);
  }

  const disconnectRes = http.del(
    `${apiBase}/api/v1/disconnect/${deviceId}`,
    null,
    { headers: authHeaders },
  );
  check(disconnectRes, {
    'disconnect 200': (r) => r.status === 200,
    'disconnect acknowledged': (r) => r.json('message') === 'Disconnected',
  });

  sleep(1);
}
