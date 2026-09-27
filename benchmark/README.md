# Benchmark

Latency, throughput and handoff downtime of the dev service, measured with [bench.py](bench.py).
The script uses only the Python standard library. It drops and recreates a table named `bench`.

```sh
URL=https://sqlite-cloudrun-dev-355496381128.asia-northeast1.run.app
benchmark/bench.py latency $URL
benchmark/bench.py throughput $URL
benchmark/bench.py handoff $URL -- make gcloud-deploy   # with .env.dev loaded
```

## Setup

- Service: `asia-northeast1`, 1 vCPU, 512 MiB, gen2, max 1 instance, concurrency 1000 ([terraform/environments/dev](../terraform/environments/dev/main.tf))
- Image: commit `6d73f45`
- Client: a laptop on a home internet connection, 2026-09-27
- Queries: `SELECT 1`; `read` = `SELECT v FROM bench WHERE id = ?` over 1,000 rows; `write` = `INSERT INTO bench (v) VALUES (?)` with a 100-byte value

## Results

### Latency

200 sequential requests over one keep-alive connection.

| Query | p50 | p90 | p99 |
| --- | ---: | ---: | ---: |
| `SELECT 1` | 15.3 ms | 24.0 ms | 32.9 ms |
| read | 14.7 ms | 19.1 ms | 25.9 ms |
| write | 14.4 ms | 17.2 ms | 21.0 ms |

Reads, writes and `SELECT 1` take about the same time, so the latency is almost entirely network and Cloud Run's frontend.
The server's own logs put a `/sql` request at about 0.5 ms (p50).

### Throughput

Each client sends requests back to back for 10 seconds.

| Query | Clients | req/s | p50 | p99 | Errors |
| --- | ---: | ---: | ---: | ---: | ---: |
| read | 1 | 63 | 14.7 ms | 24.4 ms | 0 |
| read | 10 | 688 | 13.7 ms | 28.2 ms | 0 |
| read | 50 | 560 | 22.2 ms | 1071 ms | 0 |
| read | 100 | 593 | 42.6 ms | 1124 ms | 0 |
| write | 1 | 66 | 14.5 ms | 25.2 ms | 0 |
| write | 10 | 693 | 13.6 ms | 28.2 ms | 0 |
| write | 50 | 532 | 23.1 ms | 1106 ms | 0 |
| write | 100 | 503 | 50.4 ms | 1247 ms | 0 |

Throughput tops out at about 700 req/s for both reads and writes.
Above 10 clients it stops growing and some requests wait about one second.
That wait is added outside the container: server-side latency stayed below 130 ms and CPU utilization below about 60%.
Two client processes together reached the same total as one, so the client is not the limit.

### Handoff

A single client inserts one row after another while `make gcloud-deploy` rolls out a new revision.
Downtime runs from the last successful write before the failures to the first successful write after them.

| Run | Deploy command | Failed requests | Downtime | Restore | Acked writes lost |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 17.1 s | 113 | 7.9 s | 6.1 s | 0 |
| 2 | 16.5 s | 140 | 9.4 s | 7.4 s | 0 |
| 3 | 15.3 s | 123 | 8.4 s | 7.2 s | 0 |
| 4 (after 10 min idle) | 16.3 s | 146 | 9.6 s | 7.7 s | 0 |

Restore is measured from `stopped the serving revision` to `listening on` in the new revision's logs.

- The new instance stops the old revision within 0.8 s of starting, and `POST /stop` itself takes about 0.1 s.
- Most of the downtime is `litestream restore`, at 6–8 s.
- The restore is not slow because of data: the replica is only 4.4 MB in about 400 LTX files. Why it takes this long has not been investigated.
- The first deploy of this image, before the latency and throughput runs, restored in 2.3 s and had 3.1 s of downtime.
- No acknowledged write was lost in any run.
