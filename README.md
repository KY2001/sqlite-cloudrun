# sqlite-cloudrun

A small HTTP server that runs raw SQL against SQLite (`/data/app.db`) on Cloud Run. Litestream
replicates the database to Google Cloud Storage and restores it on startup.

```
curl -X POST -H 'Content-Type: application/json' $URL/sql \
  -d '[{"sql": "SELECT ? AS x", "params": [1]}]'
[{"columns":["x"],"types":[""],"rows":[[1]],"changes":0}]
curl -X POST $URL/sync   # 204 once all changes are in GCS
```

See [openapi/openapi.yaml](openapi/openapi.yaml) for the API.
