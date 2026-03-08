# Concurrent HTTP/1.0 Server in Rust

A production-grade, multi-threaded HTTP/1.0 server built from scratch in Rust with zero high-level framework dependencies. Designed to handle heterogeneous workloads through specialized worker pools, priority-based job scheduling, and configurable backpressure mechanisms.

---

## Table of Contents

- [Highlights](#highlights)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
- [Configuration](#configuration)
- [API Reference](#api-reference)
- [Job System](#job-system)
- [Observability](#observability)
- [Testing](#testing)
- [Performance](#performance)
- [Project Structure](#project-structure)
- [Design Decisions](#design-decisions)
- [Authors](#authors)
- [License](#license)

---

## Highlights

- **From-scratch HTTP/1.0 implementation** -- TCP listener, request parser, response builder, and router built directly on `std::net` with no web framework dependencies.
- **Specialized worker pools** -- Separate thread pools for CPU-bound, I/O-bound, and per-endpoint tasks, each independently configurable in size and queue depth.
- **Priority scheduling with backpressure** -- Three-tier priority queues (low/normal/high) backed by `Condvar`-based blocking, with automatic HTTP 503 rejection when queues saturate.
- **Asynchronous job system** -- Long-running tasks are submitted, tracked, polled, and cancelled through a dedicated job API with ephemeral persistence for graceful restart recovery.
- **Memory safety guarantees** -- Leverages Rust's ownership model, `Arc<Mutex<T>>`, and MPSC channels to achieve zero data races verified at compile time.
- **92% test coverage** -- 40+ unit tests, 3 integration test suites, and load tests covering up to 100 concurrent clients.
- **24 operational endpoints** -- Spanning text manipulation, cryptographic hashing, prime testing (Miller-Rabin), arbitrary-precision Pi computation, Mandelbrot set generation, external merge sort, GZIP compression, regex search, and matrix multiplication.

---

## Architecture

```
                          ┌──────────────┐
                          │  TCP Clients  │
                          └──────┬───────┘
                                 │
                          ┌──────▼───────┐
                          │  TCP Listener │  (std::net::TcpListener, non-blocking)
                          └──────┬───────┘
                                 │
                     ┌───────────▼───────────┐
                     │   Connection Handler   │  (thread-per-connection)
                     │  parse → route → respond│
                     └───────────┬───────────┘
                                 │
                     ┌───────────▼───────────┐
                     │        Router         │  (HashMap<path, HandlerFn>)
                     └───┬───────┬───────┬───┘
                         │       │       │
              ┌──────────▼┐ ┌───▼────┐ ┌▼──────────┐
              │ CPU Pool   │ │IO Pool │ │ Endpoint   │
              │ (N threads)│ │(M thds)│ │  Pools     │
              └──────┬─────┘ └───┬────┘ └──┬────────┘
                     │           │         │
              ┌──────▼───────────▼─────────▼──┐
              │         TaskQueue<Job>         │
              │  High ─► Normal ─► Low (FIFO) │
              │  Condvar wait / backpressure   │
              └───────────────────────────────┘
                                 │
                     ┌───────────▼───────────┐
                     │     Job Manager       │
                     │  submit/status/cancel  │
                     │  ephemeral persistence │
                     └───────────────────────┘
```

**Key architectural properties:**

- **Isolation by workload type.** CPU-intensive operations (prime factorization, matrix multiplication) cannot starve I/O-intensive operations (file sort, compression) because they run in separate pools.
- **Per-endpoint scaling.** Any endpoint can be given its own dedicated pool with custom worker count and queue depth via CLI flags.
- **Graceful degradation.** When a pool's queue reaches capacity, new requests receive HTTP 503 immediately rather than accumulating unbounded latency.
- **Thread-safe by construction.** All shared state flows through `Arc<Mutex<T>>` or typed MPSC channels. The Rust compiler rejects any code that violates `Send`/`Sync` contracts.

---

## Getting Started

### Prerequisites

| Dependency | Version | Purpose |
|---|---|---|
| Rust toolchain | >= 1.70 | Compiler and Cargo |
| Git | any | Clone the repository |

All Rust crate dependencies are resolved automatically by Cargo.

### Build

```bash
git clone https://github.com/<your-username>/concurrent-http-server-rust.git
cd concurrent-http-server-rust/Code

# Release build (optimized)
cargo build --release

# Verify no warnings
cargo check
```

### Run

```bash
# Minimal
cargo run --release -- --port=8080 --data-dir=data

# With tuned pools
cargo run --release -- \
  --port=8080 \
  --data-dir=data \
  --workers./isprime=8 \
  --workers./sortfile=2 \
  --queue./sortfile=10 \
  --io-timeout=900000

# Verify
curl http://localhost:8080/status
```

---

## Configuration

All configuration is done through CLI flags. No config files are required.

| Flag | Default | Description |
|---|---|---|
| `--port` | `8080` | TCP listen port |
| `--data-dir` | `./data` | Root directory for file I/O endpoints |
| `--workers.<route>` | `4` | Worker thread count for a specific endpoint |
| `--queue.<route>` | `100` | Max queue depth for a specific endpoint |
| `--cpu-timeout` | `60000` | CPU operation timeout in milliseconds |
| `--io-timeout` | `300000` | I/O operation timeout in milliseconds |

Environment variables `HTTP_PORT` and `DATA_DIR` are also supported as fallbacks.

**Example -- production-like configuration:**

```bash
cargo run --release -- \
  --port=3000 \
  --data-dir=/var/data/httpserver \
  --workers./isprime=8 \
  --workers./sortfile=2 \
  --workers./compress=4 \
  --queue./sortfile=10 \
  --io-timeout=900000 \
  --cpu-timeout=120000
```

---

## API Reference

All endpoints accept `GET` requests with query parameters and return `application/json`. Every response includes `X-Request-Id` and `X-Worker-Pid` headers for traceability.

### Basic Operations

| Endpoint | Parameters | Description |
|---|---|---|
| `/reverse` | `text` | Reverse a string |
| `/toupper` | `text` | Convert to uppercase |
| `/hash` | `text` | SHA-256 hash of input |
| `/fibonacci` | `num` | Compute Fibonacci number |
| `/random` | `count`, `min`, `max` | Generate random integers |
| `/timestamp` | -- | Current UTC timestamp |

### CPU-Intensive Operations

| Endpoint | Parameters | Description |
|---|---|---|
| `/isprime` | `n`, `algo` (`mr`/`trial`) | Primality test (Miller-Rabin or trial division) |
| `/factor` | `n` | Prime factorization |
| `/pi` | `digits`, `algo` (`spigot`/`chudnovsky`) | Compute digits of Pi |
| `/mandelbrot` | `width`, `height`, `max_iter` | Generate Mandelbrot set iteration map |
| `/matrixmul` | `size`, `seed` | Deterministic matrix multiplication with hash verification |

### I/O-Intensive Operations

| Endpoint | Parameters | Description |
|---|---|---|
| `/createfile` | `name`, `content`, `repeat` | Create a data file |
| `/sortfile` | `name`, `algo` (`merge`/`quick`) | External merge sort for large files |
| `/wordcount` | `name` | Count words in a file |
| `/grep` | `name`, `pattern`, `icase`, `overlap` | Regex search with configurable overlap counting |
| `/compress` | `name`, `codec` (`gzip`) | GZIP compression (pure Rust or library-backed) |
| `/hashfile` | `name`, `algo` (`sha256`) | Cryptographic hash of a file |

### System Endpoints

| Endpoint | Description |
|---|---|
| `/help` | List all registered routes |
| `/status` | Server PID, uptime, running state |
| `/metrics` | Per-route request counts, latencies (avg/min/max), queue depths, worker counts |

---

## Job System

Long-running operations can be submitted asynchronously through the job API. Jobs support priority scheduling, status polling, result retrieval, and cancellation.

### Workflow

```bash
# 1. Submit a job
curl "http://localhost:8080/jobs/submit?route=/sortfile&name=large.txt&algo=merge&prio=high"
# => {"job_id":"0000019a378360fe-00000000","status":"queued"}

# 2. Poll status
curl "http://localhost:8080/jobs/status?id=0000019a378360fe-00000000"
# => {"status":"running","started_at":1730000000000}

# 3. Retrieve result
curl "http://localhost:8080/jobs/result?id=0000019a378360fe-00000000"
# => {"result":"...","elapsed_ms":4523}

# 4. Cancel (if still queued/running)
curl "http://localhost:8080/jobs/cancel?id=0000019a378360fe-00000000"

# 5. List all jobs
curl "http://localhost:8080/jobs/list"
```

### Job Lifecycle

```
Queued ──► Running ──► Done
  │            │
  │            └──► Failed
  └──► Canceled
```

Job metadata is persisted to disk. On server restart, queued and running jobs are automatically recovered and re-enqueued.

---

## Observability

### `/status`

Returns server health information including PID, uptime, and running state.

### `/metrics`

Returns per-route telemetry with the following fields for each endpoint:

- `requests`, `successes`, `failures`
- `s2xx`, `s4xx`, `s5xx` (status code buckets)
- `avg_us`, `min_us`, `max_us` (latency in microseconds)
- `queue_len`, `queue_cap`, `q_high`, `q_normal`, `q_low`
- `workers`

Job queue global counters (`jobs_size`, `jobs_high`, `jobs_normal`, `jobs_low`) are also included.

### Logging

Structured logging with configurable verbosity:

```bash
RUST_LOG=debug cargo run --release -- --port=8080
```

Levels: `error` | `warn` | `info` (default) | `debug` | `trace`

Each log entry includes a unique `RequestContext` with request ID, timestamp, and client address.

---

## Testing

### Unit Tests

40+ tests covering algorithms, HTTP parsing, parameter validation, error handling, and utilities.

```bash
cargo test
```

### Run by Module

```bash
cargo test algorithms      # Math and computation
cargo test server          # HTTP parsing and connections
cargo test handlers        # Endpoint logic

cargo test -- --nocapture  # Show stdout during tests
```

### Integration Tests

Three integration suites validate end-to-end behavior: endpoint correctness, concurrent access, and comparative performance.

### Load Tests

K6-based test scripts are included under the test directory:

| Script | Profile | Description |
|---|---|---|
| `06-load-light.js` | 10 VUs / 60s | Baseline latency with fast endpoints |
| `07-load-medium.js` | 50 VUs / 60s | Mixed basic + light CPU operations |
| `08-load-heavy.js` | 100 VUs / 60s | Full mix including heavy CPU and I/O |

### Coverage

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir coverage
```

Current coverage: **92%** (exceeds the 90% project requirement).

### Postman Collection

A complete Postman collection is provided at `Postman/SO_Proyecto_HTTP_Server.postman_collection.json` covering all 24 endpoints with valid and invalid request examples.

---

## Performance

Results measured on a standard development machine under the load test profiles described above.

| Metric | Light (10 VUs) | Medium (50 VUs) | Heavy (100 VUs) |
|---|---|---|---|
| p50 latency | < 10 ms | < 50 ms | < 50 ms |
| p95 latency | < 200 ms | < 2 s | < 10 s |
| p99 latency | < 500 ms | < 5 s | < 10 s |
| Error rate | 0% | < 1% | < 10% (timeouts) |

**I/O throughput:** Stable processing of files up to 500 MB through external merge sort with 16 MB chunk size.

**CPU scaling:** Linear throughput increase with worker count up to hardware core saturation.

---

## Project Structure

```
Code/
├── src/
│   ├── main.rs                  # Entry point, CLI parsing, server bootstrap
│   ├── lib.rs                   # Library root, module declarations
│   ├── server/
│   │   ├── http_server.rs       # TCP listener, accept loop, connection dispatch
│   │   ├── connection.rs        # Buffered read/write, timeout management
│   │   ├── requests.rs          # HTTP/1.0 request parser (method, path, headers, query)
│   │   ├── response.rs          # HTTP response builder with header injection
│   │   └── router.rs            # Path-based routing with HashMap<String, HandlerFn>
│   ├── workers/
│   │   ├── worker_pool.rs       # Fixed-size thread pool with bounded TaskQueue
│   │   ├── worker_types.rs      # WorkPriority enum, Job type alias
│   │   ├── task_queue.rs        # Priority-aware blocking queue (Mutex + Condvar)
│   │   └── worker_manager.rs    # Pool registry, submit_cpu/submit_io/submit_for
│   ├── jobs/
│   │   ├── job_manager.rs       # Job lifecycle: submit, dispatch, recover
│   │   ├── job_storage.rs       # In-memory + file-backed persistence
│   │   ├── job_queue.rs         # Priority queue for async job scheduling
│   │   ├── job_scheduler.rs     # Dispatcher thread (spawn-once pattern)
│   │   └── job_types.rs         # Job struct, JobStatus, JobPriority
│   ├── handlers/
│   │   ├── basics.rs            # reverse, toupper, hash, fibonacci, random, timestamp
│   │   ├── cpu_intensive.rs     # isprime, factor, pi, mandelbrot, matrixmul
│   │   ├── io_intensive.rs      # sortfile, wordcount, grep, compress, hashfile
│   │   ├── handler_traits.rs    # Shared handler utilities
│   │   ├── job_endpoints.rs     # /jobs/submit, status, result, cancel, list
│   │   └── metrics.rs           # /status, /metrics
│   ├── algorithms/              # Miller-Rabin, Spigot Pi, Mandelbrot, matrix ops
│   ├── io_operations/           # External merge sort, GZIP, file hashing, grep
│   ├── utils/                   # JSON builder, crypto helpers, logging, metrics
│   └── error/                   # ServerError enum with HTTP status mapping
├── tests/                       # Integration test suites
├── data/                        # Runtime data directory for file endpoints
└── Cargo.toml
```

---

## Design Decisions

### Why Rust?

Rust's ownership system eliminates data races at compile time. In a concurrent server where multiple threads share queues, metrics, and job storage, this guarantee removes an entire class of runtime bugs. The `Send` and `Sync` trait bounds enforce that only thread-safe types cross thread boundaries, turning what would be runtime crashes in C/C++ or subtle bugs in Go into compiler errors.

### Why thread-per-connection + worker pools (not async)?

The server targets a workload where most operations are either CPU-bound (prime testing, matrix math) or involve blocking file I/O (sort, compress). Async runtimes like Tokio excel at I/O multiplexing for thousands of idle connections, but they add complexity when the actual work is compute-heavy. A simpler model -- accept on a dedicated thread, dispatch to typed worker pools -- provides predictable scheduling behavior and straightforward debugging while still achieving good concurrency for the target use case.

### Why per-endpoint pools?

Different endpoints have fundamentally different resource profiles. A `/sortfile` call on a 500 MB file ties up a thread for seconds and generates heavy disk I/O. A `/fibonacci?num=20` call completes in microseconds. Sharing a single pool means a burst of sort requests would block fibonacci responses. Dedicated pools with independent queue depths allow each endpoint to scale and shed load independently.

### Why MPSC channels for result delivery?

Each handler submits a closure to a worker pool and blocks on a typed `mpsc::channel` for the result. This gives a clean request-response semantic per handler invocation without shared mutable state. The channel receiver enforces a configurable timeout, making deadline enforcement trivial.

---

## Authors

- **Anthony Barrantes Jimenez**
- **Samir Fernando Cabrera Tabash**

Built for the IC-6600 Operating Systems Principles course at Instituto Tecnologico de Costa Rica (TEC), Cartago campus.

---

## License

This project was developed for academic purposes. See the repository for licensing details.