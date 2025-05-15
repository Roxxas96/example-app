resource "kubernetes_namespace_v1" "example_app" {
  metadata {
    name = "example-app"
  }
}

variable "harbor_docker_username" {
  description = "Username for the kubernetes robot on harbor docker repo"
  sensitive   = true
  type        = string
}

variable "harbor_docker_password" {
  description = "Username for the kubernetes robot on harbor docker repo"
  sensitive   = true
  type        = string
}

resource "kubernetes_secret_v1" "harbor_credentials" {
  metadata {
    name      = "harbor-credentials"
    namespace = kubernetes_namespace_v1.example_app.metadata[0].name
  }
  type = "kubernetes.io/dockerconfigjson"
  data = {
    ".dockerconfigjson" = <<EOF
      {
        "auths": {
          "harbor.roxxas96.net": {
            "auth": "${base64encode("${var.harbor_docker_username}:${var.harbor_docker_password}")}"
          }
        }
      }
      EOF
  }
}

variable "num_services" {
  description = "Number of services to be deployed"
  type        = number
  default     = 5
}

resource "kubernetes_manifest" "application_example_service" {
  count = var.num_services

  manifest = {
    apiVersion = "argoproj.io/v1alpha1"
    kind       = "Application"
    metadata = {
      name       = "example-service-${count.index + 1}"
      namespace  = kubernetes_namespace_v1.example_app.metadata[0].name
      finalizers = ["resources-finalizer.argocd.argoproj.io"]
    }
    spec = {
      project = kubernetes_manifest.app_project_example_app.manifest.metadata.name
      source = {
        repoURL        = "harbor.${data.cloudflare_zone.roxxas96_dot_net.name}"
        targetRevision = "*.*.*"
        chart          = "example-app-helm/example-service"
        helm = {
          values = templatefile("./templates/example-service.values.yaml", {
            connected-services = "[\"${join("\",\"", setsubtract([for i in range(var.num_services) : "http://example-service-${i + 1}:50051"], ["http://example-service-${count.index + 1}:50051"]))}\"]"
          })
        }
      }
      destination = {
        server    = "https://kubernetes.default.svc"
        namespace = kubernetes_namespace_v1.example_app.metadata[0].name
      }
      syncPolicy = {
        automated = {
          enabled = true
        }
      }
    }
  }
}
