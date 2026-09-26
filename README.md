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

## Deploy

[terraform/](terraform/) provisions the Cloud Run service, the GCS bucket for the replica, Artifact Registry, and an uptime check that calls `/sync` every five minutes.
Enable the Cloud Run, Artifact Registry, IAM, Cloud Storage and Cloud Monitoring APIs first.

```sh
terraform -chdir=terraform/bootstrap init && terraform -chdir=terraform/bootstrap apply  # state bucket, once
terraform -chdir=terraform/environments/dev init && terraform -chdir=terraform/environments/dev apply
make gcloud-ar-login gcloud-build-and-push gcloud-deploy
```

Terraform creates the service with a placeholder image; `make gcloud-deploy` swaps in the application image.

### Handoff between revisions

Only one instance may own the database, so a new revision takes it over during startup:

1. Before it listens on its port (the startup probe), the new instance calls `POST https://$ENDPOINT/stop?revision=$K_REVISION`. The new revision isn't ready yet, so Cloud Run routes the request to the old one.
2. The old revision stops accepting SQL, finishes in-flight queries, waits for Litestream to replicate everything, then closes SQLite and stops Litestream.
3. The new instance restores from GCS, opens SQLite, starts Litestream, and listens.
4. Cloud Run moves traffic to the new revision.

Requests reaching the old revision after step 2 get `503`; clients should retry.
On a cold start the instance's own revision is the one serving, so the stop request can't be answered; the instance gives up after 10 seconds and goes ahead.
An instance answers `409` to a stop request from its own revision, so it never stops itself.
If the new revision fails to start after step 2, the old one keeps returning `503`; deploy again to recover.

## FAQ

**How are transactions handled?**
A request with one statement runs in autocommit mode.
A request with two or more statements runs in a single `BEGIN IMMEDIATE` transaction: all succeed or all are rolled back.
`BEGIN`, `COMMIT` and `ROLLBACK` are not allowed inside such a request, and a transaction can't span requests.

**Is it consistent?**
Yes. The service runs on a single instance (at most one per revision, with the handoff above between revisions), so every request sees the latest committed data.

**Can I lose data?**
A write is committed to the instance's ephemeral disk, then replicated to GCS asynchronously.
If the instance stops before replication, writes since the last sync (up to about five minutes with the uptime check) can be lost.
Deploys and graceful shutdowns replicate everything first.

**Are there cold starts?**
Rarely. The uptime check calls `/sync` every five minutes, which keeps the instance warm.
