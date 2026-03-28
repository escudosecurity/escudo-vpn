import http from 'k6/http';
import ws from 'k6/ws';
import { check, sleep } from 'k6';

const apiBase = (__ENV.API_BASE || '').replace(/\/+$/, '');
const testEmail = __ENV.TEST_EMAIL || '';
const testPassword = __ENV.TEST_PASSWORD || '';

export const options = {
  vus: 5,
  iterations: 20,
  thresholds: {
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<1500'],
    ws_sessions: ['count>=1'],
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

function registerOrLogin() {
  const payload = JSON.stringify({
    email: testEmail,
    password: testPassword,
  });

  const registerRes = http.post(
    `${apiBase}/api/v1/auth/register`,
    payload,
    { headers: jsonHeaders() },
  );

  if (registerRes.status === 200) {
    const body = registerRes.json();
    return body.token;
  }

  if (registerRes.status !== 409) {
    throw new Error(`Unexpected register status: ${registerRes.status} ${registerRes.body}`);
  }

  const loginRes = http.post(
    `${apiBase}/api/v1/auth/login`,
    payload,
    { headers: jsonHeaders() },
  );

  check(loginRes, {
    'login succeeded': (r) => r.status === 200,
    'login returned token': (r) => !!r.json('token'),
  });

  return loginRes.json('token');
}

export default function () {
  requireEnv('API_BASE', apiBase);
  requireEnv('TEST_EMAIL', testEmail);
  requireEnv('TEST_PASSWORD', testPassword);

  const healthRes = http.get(`${apiBase}/health`);
  check(healthRes, {
    'health 200': (r) => r.status === 200,
    'health body OK': (r) => r.body === 'OK',
  });

  const token = registerOrLogin();

  const serversRes = http.get(`${apiBase}/api/v1/servers`, {
    headers: jsonHeaders(token),
  });
  check(serversRes, {
    'servers 200': (r) => r.status === 200,
    'servers is array': (r) => Array.isArray(r.json()),
  });

  const dnsStatsRes = http.get(`${apiBase}/api/v1/stats/dns`, {
    headers: jsonHeaders(token),
  });
  check(dnsStatsRes, {
    'dns stats 200': (r) => r.status === 200,
    'dns stats shaped': (r) =>
      typeof r.json('blocked_today') === 'number' &&
      typeof r.json('queries_today') === 'number' &&
      typeof r.json('blocked_all_time') === 'number',
  });

  const wsBase = apiBase.replace(/^http/, 'ws');
  const wsUrl = `${wsBase}/api/v1/ws/stats`;
  ws.connect(
    wsUrl,
    { headers: { Authorization: `Bearer ${token}` } },
    function (socket) {
      socket.on('open', function () {
        socket.close();
      });
    }
  );

  sleep(1);
}
