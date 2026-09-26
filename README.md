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
A write is committed to the instance's in-memory filesystem, then replicated to GCS asynchronously.
If the instance stops before replication, writes since the last sync (up to about a minute with the uptime check) can be lost.

**Are there cold starts?**
Rarely. Calling `/sync` every minute (see above) keeps the instance warm.
