terraform {
  required_version = ">= 1.15"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 8.0"
    }
  }

  backend "gcs" {
    bucket = "sqlite-cloudrun-terraform-state"
    prefix = "sqlite-cloudrun/dev"
  }
}

provider "google" {
  project = "kein-432220"
  region  = "asia-northeast1"
}

module "server" {
  source = "../../modules/cloudrun"

  env            = "dev"
  gcp_project_id = "kein-432220"
  gcp_region     = "asia-northeast1"

  # Scaling
  scaling_min_instance_count       = 0
  scaling_max_instance_count       = 1
  max_instance_request_concurrency = 1000

  # Instance
  instance_cpu          = "1"
  instance_memory       = "512Mi"
  execution_environment = "EXECUTION_ENVIRONMENT_GEN2"
  startup_cpu_boost     = false
  ephemeral_disk_size   = "1Gi"

  # Request Timeout
  request_timeout = "300"

  # Startup Probe: 48 x 5s = 240s, the maximum, for stopping the old revision and restoring.
  startup_probe_period_seconds    = 5
  startup_probe_timeout_seconds   = 1
  startup_probe_failure_threshold = 48

  # Replication
  gcs_bucket_name = "sqlite-cloudrun-dev"

  # Uptime Check
  uptime_check_period = "300s"
}
