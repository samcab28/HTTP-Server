// Perfil de Carga PESADA - 100 VUs por 60s
import http from 'k6/http';
import { check, sleep } from 'k6';
import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.1/index.js';

export const options = {
  vus: 100,
  duration: '60s',
  thresholds: {
    'http_req_duration': ['med<5000', 'p(95)<10000', 'p(99)<30000'], // Usar 'med' para p50
    'http_req_failed': ['rate<0.10'],
    'checks': ['rate>0.90'],
  },
};

const BASE = 'http://localhost:8080';

const HEAVY_ROUTES = [
  '/reverse?text=heavyload',
  '/toupper?text=stress',
  '/hash?text=performance',
  '/fibonacci?num=35',
  '/random?count=1000&min=1&max=10000',
  '/isprime?n=982451653&algo=mr&rounds=10',
  '/factor?n=218714821',
  '/wordcount?name=grep.txt',
  '/grep?name=grep.txt&pattern=test&icase=1',
  '/hashfile?name=test.txt&algo=sha256',
  '/pi?digits=100&algo=spigot',
  '/matrixmul?size=128&seed=42',
];

export default function () {
  const route = HEAVY_ROUTES[Math.floor(Math.random() * HEAVY_ROUTES.length)];
  const res = http.get(`${BASE}${route}`, {
    timeout: '60s',
  });
  
  check(res, {
    'status ok': r => r.status === 200 || r.status === 408,
    'response time < 60s': r => r.timings.duration < 60000,
  });
  
  sleep(0.1);
}

export function handleSummary(data) {
  // Función mejorada que busca los nombres CORRECTOS
  const getMetricValue = (metricName, statName) => {
    const metric = data.metrics[metricName];
    if (!metric || !metric.values) return null;
    
    // K6 usa diferentes nombres para los estadísticos
    const value = metric.values[statName];
    if (value !== undefined) return value;
    
    // Fallback: buscar nombres alternativos
    if (statName === 'p(50)') return metric.values['med'] || null;
    if (statName === 'p(95)') return metric.values['p(95)'] || null;
    if (statName === 'p(99)') return metric.values['p(99)'] || null;
    
    return null;
  };

  // DEBUG: Ver qué métricas están disponibles
  console.log('\n=== MÉTRICAS DISPONIBLES LOAD TEST ===');
  Object.keys(data.metrics).forEach(key => {
    if (key.includes('http_req')) {
      console.log(`Métrica: ${key}`);
      console.log(`  Valores:`, Object.keys(data.metrics[key].values || {}));
    }
  });
  console.log('======================================\n');

  // Obtener valores usando nombres CORRECTOS
  const reqs = data.metrics.http_reqs?.values?.count || 0;
  const rate = data.metrics.http_reqs?.values?.rate || 0;
  
  const p50 = getMetricValue('http_req_duration', 'med') || 
              getMetricValue('http_req_duration', 'p(50)');
  const p95 = getMetricValue('http_req_duration', 'p(95)');
  const p99 = getMetricValue('http_req_duration', 'p(99)');
  
  const failed = (data.metrics.http_req_failed?.values?.rate || 0) * 100;

  // Función para formatear seguramente
  const formatDuration = (ms) => {
    if (!ms && ms !== 0) return 'N/A';
    return `${(ms/1000).toFixed(2)}s`;
  };

  console.log('\n========================================');
  console.log('  LOAD TEST - HEAVY PROFILE');
  console.log('========================================');
  console.log(`Profile: 100 VUs × 60s`);
  console.log(`Total requests: ${reqs}`);
  console.log(`Throughput: ${rate.toFixed(2)} req/s`);
  console.log('\nLatencies:');
  console.log(`  p50: ${formatDuration(p50)}`);
  console.log(`  p95: ${formatDuration(p95)}`);
  console.log(`  p99: ${formatDuration(p99)}`);
  console.log(`\nError rate: ${failed.toFixed(2)}%`);
  console.log('========================================\n');
  
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true })
  };
}