# sqlite-cloudrun

An HTTP server that runs raw SQL against a SQLite database on Cloud Run.
[Litestream](https://litestream.io) replicates the database to Google Cloud Storage and restores it on startup.

## Highlights

- **Affordable**: request-based billing and scale to zero. You pay only while requests are running, plus GCS storage.
- **Simple**: plain SQL over HTTP, JSON in and out.
- **Close to your users**: deploy to any Google Cloud region.

## Endpoints

| Endpoint | Description |
| --- | --- |
| `POST /sql` | Runs statements in order and returns one result per statement. Two or more statements run in one transaction. |
| `POST /sync` | Waits until all changes are replicated to GCS. |
| `POST /stop` | Hands the database off to a new revision. Called by the new revision on startup. |
| `GET /health` | Checks Litestream and the database. |

```sh
curl -X POST $URL/sql -H 'Content-Type: application/json' \
  -d '[{"sql": "SELECT ? AS x", "params": [1]}]'
# [{"columns":["x"],"types":[""],"rows":[[1]],"changes":0}]

curl -X POST $URL/sync   # 204 once all changes are in GCS
curl $URL/health         # 204 if Litestream and the database are responsive
```

See [openapi/openapi.yaml](openapi/openapi.yaml) for the full API.

## FAQ

**How are transactions handled?**
A request with one statement runs in autocommit mode.
A request with two or more statements runs in a single `BEGIN IMMEDIATE` transaction: all succeed or all are rolled back.
`BEGIN`, `COMMIT` and `ROLLBACK` are not allowed inside such a request, and a transaction can't span requests.

**Is it consistent?**
Yes. The service runs on a single instance (`--max-instances=1`), so every request sees the latest committed data.

**Can I lose data?**
Basically No. On a normal shutdown, Cloud Run sends `SIGTERM` and the server syncs to GCS before exiting. Recent writes can be lost if the instance crashes.

**Are there cold starts?**
Rarely. The uptime check calls `/sync` every five minutes, which keeps the instance warm.

**What happens on deploy?**
The new revision takes the database over before it starts serving. It calls `POST /stop`, which Cloud Run routes to the old revision; the old revision finishes in-flight queries, syncs to GCS and closes the database. The new revision then restores from GCS and starts serving.
Requests during the handoff (a few seconds) get `503`; clients should retry.
