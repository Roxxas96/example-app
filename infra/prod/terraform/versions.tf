terraform {
  backend "gcs" {
    bucket = "home-infra-tf-state"
    prefix = "example-app"
  }
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "5.0.0-alpha1"
    }
  }

  required_version = "1.12.0"
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
}
