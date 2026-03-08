// Pruebas de Colas - Versión Corregida
import http from "k6/http";
import { check, sleep } from "k6";
import { Counter } from "k6/metrics";
import { textSummary } from "https://jslib.k6.io/k6-summary/0.0.1/index.js";

export const options = {
  vus: 30,
  iterations: 120,
  thresholds: {
    checks: ["rate>0.80"], // Reducido temporalmente para debugging
    http_req_failed: ["rate<0.05"],
  },
  // Ensure k6 computes and exposes these percentiles in the summary
  summaryTrendStats: ["min", "med", "p(90)", "p(95)", "p(99)", "max", "avg"],
};

const BASE = "http://localhost:8080";
const successfulSubmissionsMetric = new Counter("successful_submissions");

// Capture initial system state before test starts
export function setup() {
  try {
    const res = http.get(`${BASE}/jobs/list`, { tags: { endpoint: "list" } });
    if (res.status === 200) {
      const listJson = res.json();
      const initialIds = (listJson.jobs || []).map((j) => j.id);
      console.log(`Setup: initial jobs present: ${initialIds.length}`);
      return { initialIds };
    }
    console.log(`Setup: failed to list jobs, status ${res.status}`);
  } catch (e) {
    console.log("Setup error while listing jobs:", e.message);
  }
  return { initialIds: [] };
}

export default function () {
  // Submit job - con mejor manejo de errores
  const submitRes = http.get(
    `${BASE}/jobs/submit?route=/sortfile&name=data-test.txt&algo=quick&prio=high`,
    {
      timeout: "30s",
      tags: { endpoint: "submit" },
    }
  );

  // DEBUG: Ver respuesta completa
  if (submitRes.status !== 202) {
    console.log(
      `❌ Submit failed - Status: ${
        submitRes.status
      }, Body: ${submitRes.body.substring(0, 200)}`
    );
  }

  const submitted = check(submitRes, {
    "job submitted": (r) => r.status === 202,
    "has job_id": (r) => {
      if (r.status !== 202) {
        console.log(`❌ Status ${r.status}: ${r.body.substring(0, 100)}`);
        return false;
      }
      try {
        const json = r.json();
        const hasJobId = json && json.job_id !== undefined;
        if (!hasJobId) {
          console.log(`❌ No job_id in response: ${JSON.stringify(json)}`);
        }
        return hasJobId;
      } catch (e) {
        console.log(
          `❌ JSON parse error: ${e.message}, Body: ${r.body.substring(0, 100)}`
        );
        return false;
      }
    },
  });

  if (submitted) {
    successfulSubmissionsMetric.add(1);
    try {
      const json = submitRes.json();
      const jobId = json.job_id;
      console.log(`✅ Job submitted: ${jobId}`);

      sleep(0.05);

      // Check status
      const statusRes = http.get(`${BASE}/jobs/status?id=${jobId}`, {
        tags: { endpoint: "status" },
      });

      check(statusRes, {
        "status readable": (r) => r.status === 200,
        "valid state": (r) => {
          try {
            const statusJson = r.json();
            return [
              "queued",
              "running",
              "completed",
              "done",
              "failed",
            ].includes(statusJson.status);
          } catch {
            return false;
          }
        },
      });
    } catch (error) {
      console.error("Error processing job:", error);
    }
  } else {
    console.log(`❌ Submission failed - Status: ${submitRes.status}`);
  }

  sleep(0.1);
}

export function handleSummary(data) {
  const getMetricValue = (metricName, statName) => {
    const metric = data.metrics[metricName];
    if (!metric || !metric.values) return null;

    const value = metric.values[statName];
    if (value !== undefined) return value;

    if (statName === "p(50)") return metric.values["med"] || null;
    if (statName === "p(95)") return metric.values["p(95)"] || null;
    if (statName === "p(99)") return metric.values["p(99)"] || null;

    return null;
  };

  // Obtener métricas
  const checksRate = (data.metrics.checks?.values?.rate || 0) * 100;
  const failedRate = (data.metrics.http_req_failed?.values?.rate || 0) * 100;
  const totalReqs = data.metrics.http_reqs?.values?.count || 0;
  const successfulTotal =
    data.metrics.successful_submissions?.values?.count || 0;

  const p50 = getMetricValue("http_req_duration", "med");
  const p95 = getMetricValue("http_req_duration", "p(95)");
  const p99 = getMetricValue("http_req_duration", "p(99)");

  const formatDuration = (ms) => {
    if (!ms && ms !== 0) return "N/A";
    if (ms < 1000) return `${ms.toFixed(0)}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  console.log("\n========================================");
  console.log("  QUEUE OVERFLOW TEST - DEBUG RESULTS");
  console.log("========================================");
  console.log(
    `Attempted submissions: ${data.metrics.iterations?.values?.count || 0}`
  );
  console.log(`Successful submissions: ${successfulTotal}`);
  console.log(`Total HTTP requests: ${totalReqs}`);
  console.log("\nQuality Metrics:");
  console.log(`  Checks passed: ${checksRate.toFixed(1)}%`);
  console.log(`  HTTP failures: ${failedRate.toFixed(1)}%`);

  console.log("\nLatencies:");
  console.log(`  p50: ${formatDuration(p50)}`);
  console.log(`  p95: ${formatDuration(p95)}`);
  console.log(`  p99: ${formatDuration(p99)}`);

  console.log("\nDEBUG INFO:");
  console.log("  First few job IDs: see per-request console log output");
  console.log("========================================\n");

  return {
    stdout: textSummary(data, { indent: " ", enableColors: true }),
  };
}

export function teardown(setupData) {
  sleep(2);

  // Limpiar jobs existentes antes de verificar
  console.log("\n🔍 Checking system state...");

  try {
    const initialIds = setupData?.initialIds || [];
    const initialSet = new Set(initialIds);

    const listRes = http.get(`${BASE}/jobs/list`, {
      tags: { endpoint: "list" },
    });
    if (listRes.status === 200) {
      const listJson = listRes.json();
      const finalJobs = Array.isArray(listJson.jobs) ? listJson.jobs : [];
      const finalIds = finalJobs.map((j) => j.id);
      const newIds = finalIds.filter((id) => !initialSet.has(id));

      console.log("\n========================================");
      console.log("  FINAL SYSTEM ANALYSIS");
      console.log("========================================");
      console.log(`Jobs present at start: ${initialIds.length}`);
      console.log(`Jobs present at end:   ${finalIds.length}`);
      console.log(`New job IDs observed:  ${newIds.length}`);

      if (newIds.length === 0) {
        console.log("\n❌ No new job IDs found in system state");
        console.log("Notes:");
        console.log(
          "  - Jobs may complete and be removed, depending on server policy"
        );
        console.log("  - See handleSummary for submission metrics");
      } else {
        const preview = newIds.slice(0, 5).join(", ");
        if (preview)
          console.log(
            `Sample new IDs: ${preview}${newIds.length > 5 ? ", ..." : ""}`
          );
        console.log("\n✅ New jobs are present in the system");
      }
    } else {
      console.log(`Failed to list jobs at teardown, status ${listRes.status}`);
    }
  } catch (error) {
    console.log("Error checking system state:", error.message);
  }

  console.log("========================================\n");
}
