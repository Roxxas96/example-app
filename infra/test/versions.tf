terraform {
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "2.35.1"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "2.17.0"
    }
  }

  required_version = "1.12.0"
}

variable "kubeconfig_path" {
  description = "The path to the cluster kubeconfig"
  type        = string
  default     = "~/.kube/config"
}

variable "kubeconfig_context" {
  description = "The context to use for the cluster"
  type        = string
  default     = "kind-chart-testing"
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
