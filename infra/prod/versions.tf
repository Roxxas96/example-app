terraform {
  backend "gcs" {
    bucket = "home-infra-tf-state"
    prefix = "example-app"
  }
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "2.35.1"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "2.17.0"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "5.0.0-alpha1"
    }
  }

  required_version = "1.11.4"
}

variable "kubeconfig_path" {
  description = "The path to the cluster kubeconfig"
  type        = string
  default     = "~/.kube/home.kubeconfig"
}

variable "kubeconfig_context" {
  description = "The context to use for the cluster"
  type        = string
  default     = "admin@home"
}

provider "kubernetes" {
  config_path    = var.kubeconfig_path
  config_context = var.kubeconfig_context
}

provider "helm" {
  kubernetes {
    config_path    = var.kubeconfig_path
    config_context = var.kubeconfig_context
  }
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
}
