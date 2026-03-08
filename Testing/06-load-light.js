// Perfil de Carga LIGERA - 10 VUs por 60s
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  vus: 10,
  duration: '60s',
  thresholds: {
    'http_req_duration': ['p(95)<200', 'p(99)<500'],
    'http_req_failed': ['rate<0.01'],
    'checks': ['rate>0.99'],
  },
};

const BASE = 'http://localhost:8080';

const LIGHT_ROUTES = [
  '/reverse?text=loadtest',
  '/toupper?text=lightload',
  '/hash?text=performance',
  '/timestamp',
  '/fibonacci?num=15',
  '/random?count=50&min=1&max=100',
];

export default function () {
  const route = LIGHT_ROUTES[Math.floor(Math.random() * LIGHT_ROUTES.length)];
  const res = http.get(`${BASE}${route}`);
  
  check(res, {
    'status 200': r => r.status === 200,
    'response time < 500ms': r => r.timings.duration < 500,
  });
  
  sleep(0.5);
}

export function handleSummary(data) {
  const reqs = data.metrics.http_reqs.values.count;
  const rate = data.metrics.http_reqs.values.rate;
  const p50 = data.metrics.http_req_duration.values['p(50)'];
  const p95 = data.metrics.http_req_duration.values['p(95)'];
  const p99 = data.metrics.http_req_duration.values['p(99)'];
  const failed = data.metrics.http_req_failed.values.rate * 100;
  
  console.log('\n========================================');
  console.log('  LOAD TEST - LIGHT PROFILE');
  console.log('========================================');
  console.log(`Profile: 10 VUs × 60s`);
  console.log(`Total requests: ${reqs}`);
  console.log(`Throughput: ${rate.toFixed(2)} req/s`);
  console.log('\nLatencies:');
  console.log(`  p50: ${p50.toFixed(2)}ms`);
  console.log(`  p95: ${p95.toFixed(2)}ms`);
  console.log(`  p99: ${p99.toFixed(2)}ms`);
  console.log(`\nError rate: ${failed.toFixed(2)}%`);
  console.log('========================================\n');
  
  return { 'stdout': '' };
}