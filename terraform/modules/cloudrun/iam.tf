# The service reaches Google APIs through this account's application default credentials.
resource "google_service_account" "cloudrun" {
  account_id   = local.service
  display_name = "Cloud Run service account for sqlite-cloudrun (${var.env})"
}

# Litestream reads and writes the replica.
resource "google_storage_bucket_iam_member" "replica" {
  bucket = google_storage_bucket.replica.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.cloudrun.email}"
}
