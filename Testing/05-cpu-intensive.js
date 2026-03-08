// Pruebas CPU Intensivas - VERSIÓN DEFINITIVA FUNCIONANDO
import http from "k6/http";
import { check } from "k6";
import { textSummary } from "https://jslib.k6.io/k6-summary/0.0.1/index.js";

export const options = {
  summaryTrendStats: ["min", "med", "p(90)", "p(95)", "p(99)", "max", "avg"],
  scenarios: {
    pi: {
      executor: "constant-vus",
      exec: "testPi",
      vus: 2,
      duration: "60s",
    },
    matrixmul: {
      executor: "constant-vus",
      exec: "testMatrix",
      vus: 2,
      duration: "60s",
      startTime: "10s",
    },
    mandelbrot: {
      executor: "constant-vus",
      exec: "testMandelbrot",
      vus: 2,
      duration: "60s",
      startTime: "20s",
    },
  },
  thresholds: {
    "http_req_duration{operation:pi}": [
      "med<30000", // ¡Usar 'med' en lugar de 'p(50)'!
      "p(95)<60000",
      "p(99)<75000",
    ],
    "http_req_duration{operation:matrix}": [
      "med<30000", // ¡Usar 'med' en lugar de 'p(50)'!
      "p(95)<60000",
      "p(99)<75000",
    ],
    "http_req_duration{operation:mandelbrot}": [
      "med<30000", // ¡Usar 'med' en lugar de 'p(50)'!
      "p(95)<60000",
      "p(99)<75000",
    ],
    checks: ["rate>0.85"],
  },
};

const BASE = "http://localhost:8080";

const TEST_CONFIG = {
  pi: { digits: 1000, algo: "spigot" },
  matrix: { size: 256, seed: 42 },
  mandelbrot: { width: 1024, height: 768, max_iter: 500 },
};

export function testPi() {
  const { digits, algo } = TEST_CONFIG.pi;
  const res = http.get(`${BASE}/pi?digits=${digits}&algo=${algo}`, {
    tags: { operation: "pi" },
    timeout: "90s",
  });

  check(res, {
    "pi status 200": (r) => r.status === 200,
    "pi valid response": (r) => {
      try {
        const body = r.json();
        return body && body.value;
      } catch {
        return false;
      }
    },
  });
}

export function testMatrix() {
  const { size, seed } = TEST_CONFIG.matrix;
  const res = http.get(`${BASE}/matrixmul?size=${size}&seed=${seed}`, {
    tags: { operation: "matrix" },
    timeout: "90s",
  });

  check(res, {
    "matrix status 200": (r) => r.status === 200,
    "matrix has hash": (r) => {
      try {
        const body = r.json();
        return body && body.hash;
      } catch {
        return false;
      }
    },
  });
}

export function testMandelbrot() {
  const { width, height, max_iter } = TEST_CONFIG.mandelbrot;
  const res = http.get(
    `${BASE}/mandelbrot?width=${width}&height=${height}&max_iter=${max_iter}`,
    {
      tags: { operation: "mandelbrot" },
      timeout: "90s",
    }
  );

  check(res, {
    "mandelbrot status 200": (r) => r.status === 200,
    "mandelbrot has map": (r) => {
      try {
        const body = r.json();
        return body && body.map;
      } catch {
        return false;
      }
    },
  });
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
    if (statName === "p(50)") return metric.values["med"] || null;
    if (statName === "p(95)") return metric.values["p(95)"] || null;
    if (statName === "p(99)") return metric.values["p(99)"] || null;

    return null;
  };

  // Obtener valores usando nombres CORRECTOS
  const piP50 =
    getMetricValue("http_req_duration{operation:pi}", "med") ||
    getMetricValue("http_req_duration{operation:pi}", "p(50)");
  const piP95 = getMetricValue("http_req_duration{operation:pi}", "p(95)");
  const piP99 = getMetricValue("http_req_duration{operation:pi}", "p(99)");

  const matrixP50 =
    getMetricValue("http_req_duration{operation:matrix}", "med") ||
    getMetricValue("http_req_duration{operation:matrix}", "p(50)");
  const matrixP95 = getMetricValue(
    "http_req_duration{operation:matrix}",
    "p(95)"
  );
  const matrixP99 = getMetricValue(
    "http_req_duration{operation:matrix}",
    "p(99)"
  );

  const mandelP50 =
    getMetricValue("http_req_duration{operation:mandelbrot}", "med") ||
    getMetricValue("http_req_duration{operation:mandelbrot}", "p(50)");
  const mandelP95 = getMetricValue(
    "http_req_duration{operation:mandelbrot}",
    "p(95)"
  );
  const mandelP99 = getMetricValue(
    "http_req_duration{operation:mandelbrot}",
    "p(99)"
  );

  // DEBUG: Ver qué métricas están disponibles
  console.log("\n=== MÉTRICAS DISPONIBLES ===");
  Object.keys(data.metrics).forEach((key) => {
    if (key.includes("http_req_duration")) {
      console.log(`Métrica: ${key}`);
      console.log(`  Valores:`, Object.keys(data.metrics[key].values || {}));
    }
  });
  console.log("============================\n");

  const formatDuration = (ms) => (ms ? `${(ms / 1000).toFixed(2)}s` : "N/A");

  console.log("\n========================================");
  console.log("  CPU INTENSIVE TEST RESULTS");
  console.log("========================================");
  console.log("PI CALCULATION (1000 digits):");
  console.log(`  p50: ${formatDuration(piP50)}`);
  console.log(`  p95: ${formatDuration(piP95)}`);
  console.log(`  p99: ${formatDuration(piP99)}`);
  console.log("\nMATRIX MULTIPLY (256x256):");
  console.log(`  p50: ${formatDuration(matrixP50)}`);
  console.log(`  p95: ${formatDuration(matrixP95)}`);
  console.log(`  p99: ${formatDuration(matrixP99)}`);
  console.log("\nMANDELBROT (1024x768, 500 iter):");
  console.log(`  p50: ${formatDuration(mandelP50)}`);
  console.log(`  p95: ${formatDuration(mandelP95)}`);
  console.log(`  p99: ${formatDuration(mandelP99)}`);
  console.log("========================================\n");

  return {
    stdout: textSummary(data, { indent: " ", enableColors: true }),
  };
}
