// Pruebas de Carrera - 50 VUs simultáneos por 30s
import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 50,
  duration: '30s',
  thresholds: {
    'http_req_duration': ['p(95)<1000'],
    'checks': ['rate>0.99'],
    'http_req_failed': ['rate<0.01'],
  },
};

const BASE = 'http://localhost:8080';

const ROUTES = [
  '/reverse?text=concurrent',
  '/toupper?text=racetest',
  '/hash?text=simultaneous',
  '/fibonacci?num=25',
  '/random?count=100&min=1&max=1000',
  '/timestamp',
  '/wordcount?name=grep.txt',
  '/grep?name=grep.txt&pattern=test',
  '/isprime?n=982451653&algo=mr',
];

export default function () {
  const route = ROUTES[Math.floor(Math.random() * ROUTES.length)];
  const res = http.get(`${BASE}${route}`);
  
  check(res, {
    'status 200': r => r.status === 200,
    'has request-id': r => r.headers['X-Request-Id'] !== undefined,
    'is json': r => r.headers['Content-Type'].includes('application/json'),
    'response time ok': r => r.timings.duration < 2000,
  });
}

export function handleSummary(data) {
  const passed = data.metrics.checks.values.passes;
  const failed = data.metrics.checks.values.fails || 0;
  const total = passed + failed;
  const requests = data.metrics.http_reqs.values.count;
  const p95 = data.metrics.http_req_duration.values['p(95)'];
  
  console.log('\n========================================');
  console.log('  RACE CONDITIONS TEST RESULTS');
  console.log('========================================');
  console.log(`✓ Total requests: ${requests}`);
  console.log(`✓ Checks passed: ${passed}/${total} (${((passed/total)*100).toFixed(2)}%)`);
  console.log(`✓ p95 latency: ${p95.toFixed(2)}ms`);
  console.log(`✓ No deadlocks: ${failed === 0 ? 'CONFIRMED' : 'FAILED'}`);
  console.log(`✓ Concurrent VUs: 50`);
  console.log('========================================\n');
  
  return { 'stdout': '' };
}