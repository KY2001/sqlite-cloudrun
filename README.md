# sqlite-cloudrun

A small HTTP server that runs raw SQL against SQLite (`/data/app.db`) on Cloud Run. Litestream
replicates the database to Google Cloud Storage and restores it on startup.

```
curl -X POST --data-binary "SELECT 1 AS x" $URL/sql
{"changes":0,"columns":["x"],"rows":[[1]]}
curl -X POST $URL/sync   # 204 once all changes are in GCS
```

See [openapi.yaml](openapi.yaml) for the API.
