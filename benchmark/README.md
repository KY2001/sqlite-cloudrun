# Benchmark

Latency, throughput and handoff downtime of the dev service, measured with [bench.py](bench.py).
The script uses only the Python standard library. It drops and recreates a table named `bench`.

```sh
URL=https://sqlite-cloudrun-dev-355496381128.asia-northeast1.run.app
benchmark/bench.py latency $URL
benchmark/bench.py throughput $URL
benchmark/bench.py handoff $URL -- make gcloud-deploy   # with .env.dev loaded
```

Every response carries a `Server-Timing: app;dur=<ms>` header with the time the server spent on the request.
Latencies below come from that header, so they exclude the network round trip.

## Setup

- Service: `asia-northeast1`, 1 vCPU, 512 MiB, gen2, max 1 instance, concurrency 1000 ([terraform/environments/dev](../terraform/environments/dev/main.tf))
- Image: commit `868d2b8`
- Client: a laptop on a home internet connection, 2026-09-27
- Queries: `SELECT 1`; `read` = `SELECT v FROM bench WHERE id = ?` over 1,000 rows; `write` = `INSERT INTO bench (v) VALUES (?)` with a 100-byte value

## Results

### Latency

200 sequential requests per row.

| Request | p50 | p90 | p99 |
| --- | ---: | ---: | ---: |
| `/sql` `SELECT 1` | 0.61 ms | 1.27 ms | 3.16 ms |
| `/sql` read | 0.91 ms | 1.73 ms | 10.59 ms |
| `/sql` write | 0.81 ms | 1.40 ms | 4.09 ms |
| `GET /health` | 43.3 ms | 50.6 ms | 62.0 ms |
| `POST /sync` | 42.5 ms | 51.4 ms | 61.1 ms |
| `POST /stop` | 99–130 ms (3 handoffs) | | |

`/health` and `/sync` both go through Litestream, which costs about 40 ms.
`/stop` can't be called without handing the database off, so its time comes from the old revision's request log during the handoffs below.

### Throughput

Each client sends requests back to back for 10 seconds.
The req/s column counts completed requests at the client, so it is the only column that includes the network round trip; one client gets only about 50 req/s for that reason.

| Query | Clients | req/s | p50 | p99 | Errors |
| --- | ---: | ---: | ---: | ---: | ---: |
| read | 1 | 50 | 0.66 ms | 3.30 ms | 0 |
| read | 10 | 568 | 0.37 ms | 1.78 ms | 0 |
| read | 50 | 530 | 0.40 ms | 5.66 ms | 0 |
| read | 100 | 549 | 0.47 ms | 22.16 ms | 0 |
| write | 1 | 55 | 0.65 ms | 3.26 ms | 0 |
| write | 10 | 543 | 0.41 ms | 4.79 ms | 0 |
| write | 50 | 426 | 1.17 ms | 36.80 ms | 0 |
| write | 100 | 538 | 2.71 ms | 96.79 ms | 0 |

Throughput levels off at about 550 req/s from 10 clients on, for reads and writes alike.
The server is not the limit: it spends under 3 ms per request at p50, and its CPU stayed below about 60%.
Two client processes together reached the same total as one, so the client is not the limit either.
The cap is in front of the container.

### Handoff

A single client inserts one row after another while `make gcloud-deploy` rolls out a new revision.
Downtime runs from the last successful write before the failures to the first successful write after them.
Restore is measured from `stopped the serving revision` to `listening on` in the new revision's logs.

| Run | Deploy command | Failed requests | Downtime | `/stop` | Restore | Acked writes lost |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 22.2 s | 185 | 16.5 s | 127 ms | 15.5 s | 0 |
| 2 | 27.2 s | 244 | 16.1 s | 99 ms | 14.6 s | 0 |
| 3 | 21.0 s | 194 | 14.4 s | 130 ms | 12.8 s | 0 |

- No acknowledged write was lost in any run.
- The new instance stops the old revision within about 0.7 s of starting; after that, nearly all of the downtime is `litestream restore`.
- Restore gets slower as the Litestream replica in GCS accumulates files, even though the database stays small:

| Handoffs | Replica files after | Replica size after | Restore |
| --- | ---: | ---: | ---: |
| First deploy of the day | not counted | not counted | 2.3 s |
| Next 4, after a throughput run | 410 | 4.4 MB | 6–8 s |
| The 3 above, after two more throughput runs | 624 | 8.5 MB | 13–16 s |

Most of the new files are level-0 LTX files (340 of 624).
Why they aren't compacted away has not been investigated.
