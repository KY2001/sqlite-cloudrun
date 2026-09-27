# Benchmark

Latency, throughput and handoff downtime of the dev service, measured with [bench.py](bench.py).

```sh
URL=https://sqlite-cloudrun-dev-355496381128.asia-northeast1.run.app
benchmark/bench.py latency $URL
benchmark/bench.py throughput $URL
benchmark/bench.py handoff $URL -- make gcloud-deploy   # with .env.dev loaded
```

Latencies below come from that header, so they exclude the network round trip.

## Setup

- Service: `asia-northeast1`, 1 vCPU, 512 MiB, gen2, max 1 instance, concurrency 1000 ([terraform/environments/dev](../terraform/environments/dev/main.tf))
- Image: commit `86d88ef`, with hourly Litestream snapshots ([litestream.yaml](../litestream.yaml))
- Client: a laptop on a home internet connection, 2026-09-27
- Queries: `SELECT 1`; `read` = `SELECT v FROM bench WHERE id = ?` over 1,000 rows; `write` = `INSERT INTO bench (v) VALUES (?)` with a 100-byte value

## Results

### Latency

200 sequential requests per row.

| Request | p50 | p90 | p99 |
| --- | ---: | ---: | ---: |
| `/sql` `SELECT 1` | 0.92 ms | 1.77 ms | 2.91 ms |
| `/sql` read | 1.32 ms | 2.44 ms | 3.71 ms |
| `/sql` write | 1.16 ms | 2.18 ms | 3.89 ms |
| `GET /health` | 43.3 ms | 49.6 ms | 56.2 ms |
| `POST /sync` | 47.4 ms | 54.9 ms | 69.1 ms |
| `POST /stop` | 114–167 ms (3 handoffs) | | |

`/health` and `/sync` both go through Litestream, which costs about 40 ms.

### Throughput

Each client sends requests back to back for 10 seconds.

| Query | Clients | req/s | p50 | p99 | Errors |
| --- | ---: | ---: | ---: | ---: | ---: |
| read | 1 | 54 | 1.17 ms | 3.58 ms | 0 |
| read | 10 | 675 | 0.45 ms | 1.61 ms | 0 |
| read | 50 | 560 | 0.51 ms | 14.90 ms | 0 |
| read | 100 | 555 | 1.59 ms | 68.37 ms | 0 |
| write | 1 | 61 | 0.86 ms | 3.21 ms | 0 |
| write | 10 | 538 | 0.57 ms | 9.58 ms | 0 |
| write | 50 | 480 | 1.49 ms | 102.58 ms | 0 |
| write | 100 | 475 | 6.74 ms | 200.15 ms | 0 |

Throughput levels off at 500–700 req/s from 10 clients on.

### Handoff

A single client inserts one row after another while `make gcloud-deploy` rolls out a new revision.
Downtime runs from the last successful write before the failures to the first successful write after them.
Restore is measured from `stopped the serving revision` to `listening on` in the new revision's logs.
Restore downloads the latest snapshot plus every LTX file written since, so its time depends on how much was written since the last snapshot:

| When | Files restored | Restore | Downtime |
| --- | ---: | ---: | ---: |
| Right after a snapshot | not counted | 2.2–2.7 s | 4.8–5.7 s |
| After ~50,000 writes since the snapshot | 203 | 6.4–7.9 s | 8.7–9.5 s |
