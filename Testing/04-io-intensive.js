import http from "k6/http";
import { check, sleep } from "k6";
import { Trend, Counter } from "k6/metrics";
import { textSummary } from "https://jslib.k6.io/k6-summary/0.0.1/index.js";

// Métricas personalizadas
const sortfileDuration = new Trend("sortfile_duration", true);
const compressDuration = new Trend("compress_duration", true);
const hashfileDuration = new Trend("hashfile_duration", true);

const sortfileErrors = new Counter("sortfile_errors");
const compressErrors = new Counter("compress_errors");
const hashfileErrors = new Counter("hashfile_errors");

export const options = {
  summaryTrendStats: ["min", "med", "p(90)", "p(95)", "p(99)", "max", "avg"],
  scenarios: {
    // Ejecutar secuencialmente para evitar competencia por recursos
    sortfile: {
      executor: "constant-vus",
      exec: "testSortFile",
      vus: 5,
      duration: "1m",
      startTime: "0s",
    },
    compress: {
      executor: "constant-vus",
      exec: "testCompress",
      vus: 5,
      duration: "1m",
      startTime: "1m5s",
    },
    hashfile: {
      executor: "constant-vus",
      exec: "testHashFile",
      vus: 5,
      duration: "1m",
      startTime: "2m10s",
    },
  },
  thresholds: {
    // Usar 'med' en lugar de 'p(50)' para que funcione correctamente
    sortfile_duration: ["med<5000", "p(95)<10000", "p(99)<15000"],
    compress_duration: ["med<10000", "p(95)<15000", "p(99)<20000"],
    hashfile_duration: ["med<5000", "p(95)<8000", "p(99)<12000"],
    checks: ["rate>0.95"],
    http_req_failed: ["rate<0.05"],
  },
};

const BASE = "http://localhost:8080";

// Test 1: SortFile con merge sort
export function testSortFile() {
  const res = http.get(`${BASE}/sortfile?name=data-sort-50mb.txt&algo=merge`, {
    timeout: "30s",
  });

  sortfileDuration.add(res.timings.duration);

  const success = check(res, {
    "sortfile status 200": (r) => r.status === 200,
    "sortfile has valid body": (r) => {
      if (r.status !== 200) return false;
      try {
        const json = r.json();
        return (
          json.filename_out &&
          json.algo === "merge" &&
          json.elapsed_ms !== undefined
        );
      } catch (e) {
        return false;
      }
    },
  });

  if (!success) {
    sortfileErrors.add(1);
  }

  sleep(1);
}

// Test 2: Compress con gzip
export function testCompress() {
  const res = http.get(
    `${BASE}/compress?name=data-sort-50mb.txt&codec=gzip&impl=lib`,
    {
      timeout: "30s",
    }
  );

  compressDuration.add(res.timings.duration);

  const success = check(res, {
    "compress status 200": (r) => r.status === 200,
    "compress has valid body": (r) => {
      if (r.status !== 200) return false;
      try {
        const json = r.json();
        return json.bytes_in > 0 && json.bytes_out > 0 && json.codec === "gzip";
      } catch (e) {
        return false;
      }
    },
  });

  if (!success) {
    compressErrors.add(1);
  }

  sleep(1);
}

// Test 3: HashFile con SHA256
export function testHashFile() {
  const res = http.get(`${BASE}/hashfile?name=data-sort-50mb.txt&algo=sha256`, {
    timeout: "30s",
  });

  hashfileDuration.add(res.timings.duration);

  const success = check(res, {
    "hashfile status 200": (r) => r.status === 200,
    "hashfile has valid hash": (r) => {
      if (r.status !== 200) return false;
      try {
        const json = r.json();
        return json.hash && json.hash.length === 64 && json.algo === "sha256";
      } catch (e) {
        return false;
      }
    },
  });

  if (!success) {
    hashfileErrors.add(1);
  }

  sleep(1);
}

// Resumen personalizado corregido
export function handleSummary(data) {
  // Función mejorada que busca los nombres CORRECTOS
  const getMetricValue = (metricName, statName) => {
    const metric = data.metrics[metricName];
    if (!metric || !metric.values) return null;

    // K6 usa diferentes nombres para los estadísticos
    const value = metric.values[statName];
    if (value !== undefined) return value;

    // Fallback: buscar nombres alternativos
    if (statName === "p(50)") return metric.values["med"] || null;
    if (statName === "p(95)") return metric.values["p(95)"] || null;
    if (statName === "p(99)") return metric.values["p(99)"] || null;

    return null;
  };

  // Obtener valores usando nombres CORRECTOS
  const sortP50 =
    getMetricValue("sortfile_duration", "med") ||
    getMetricValue("sortfile_duration", "p(50)");
  const sortP95 = getMetricValue("sortfile_duration", "p(95)");
  const sortP99 = getMetricValue("sortfile_duration", "p(99)");

  const compP50 =
    getMetricValue("compress_duration", "med") ||
    getMetricValue("compress_duration", "p(50)");
  const compP95 = getMetricValue("compress_duration", "p(95)");
  const compP99 = getMetricValue("compress_duration", "p(99)");

  const hashP50 =
    getMetricValue("hashfile_duration", "med") ||
    getMetricValue("hashfile_duration", "p(50)");
  const hashP95 = getMetricValue("hashfile_duration", "p(95)");
  const hashP99 = getMetricValue("hashfile_duration", "p(99)");

  // DEBUG: Ver qué métricas están disponibles
  console.log("\n=== MÉTRICAS DISPONIBLES IO ===");
  Object.keys(data.metrics).forEach((key) => {
    if (
      key.includes("sortfile") ||
      key.includes("compress") ||
      key.includes("hashfile")
    ) {
      console.log(`Métrica: ${key}`);
      console.log(`  Valores:`, Object.keys(data.metrics[key].values || {}));
    }
  });
  console.log("===============================\n");

  // Función para formatear duración
  const formatDuration = (ms) => {
    if (ms === null || ms === undefined) return "N/A";
    if (ms < 1000) return `${ms.toFixed(0)}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  // Obtener conteos de muestras
  const sortCount = data.metrics.sortfile_duration?.values?.count || 0;
  const compCount = data.metrics.compress_duration?.values?.count || 0;
  const hashCount = data.metrics.hashfile_duration?.values?.count || 0;

  // Calcular tasa de checks
  const checksMetric = data.metrics.checks;
  const checksRate = checksMetric ? (checksMetric.values.rate || 0) * 100 : 0;

  // Calcular tasa de errores HTTP
  const httpFailedMetric = data.metrics.http_req_failed;
  const httpFailRate = httpFailedMetric
    ? (httpFailedMetric.values.rate || 0) * 100
    : 0;

  // Generar reporte en el mismo formato que el código funcionando
  console.log("\n========================================");
  console.log("  IO INTENSIVE TEST RESULTS");
  console.log("========================================");

  console.log("\nSORTFILE (merge sort - 50MB):");
  console.log(`  Samples: ${sortCount}`);
  console.log(`  p50: ${formatDuration(sortP50)}`);
  console.log(`  p95: ${formatDuration(sortP95)}`);
  console.log(`  p99: ${formatDuration(sortP99)}`);

  console.log("\nCOMPRESS (gzip/lib - 50MB):");
  console.log(`  Samples: ${compCount}`);
  console.log(`  p50: ${formatDuration(compP50)}`);
  console.log(`  p95: ${formatDuration(compP95)}`);
  console.log(`  p99: ${formatDuration(compP99)}`);

  console.log("\nHASHFILE (SHA256 - 50MB):");
  console.log(`  Samples: ${hashCount}`);
  console.log(`  p50: ${formatDuration(hashP50)}`);
  console.log(`  p95: ${formatDuration(hashP95)}`);
  console.log(`  p99: ${formatDuration(hashP99)}`);

  console.log("\nQUALITY METRICS:");
  console.log(`  Checks passed: ${checksRate.toFixed(1)}%`);
  console.log(`  HTTP failures: ${httpFailRate.toFixed(1)}%`);

  console.log("========================================\n");

  return {
    stdout: textSummary(data, { indent: " ", enableColors: true }),
  };
}
