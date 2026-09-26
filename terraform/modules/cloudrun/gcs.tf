# Litestream replicates the database here and restores it on startup.
resource "google_storage_bucket" "replica" {
  name     = var.gcs_bucket_name
  location = var.gcp_region

  uniform_bucket_level_access = true
  public_access_prevention    = "enforced"
}
