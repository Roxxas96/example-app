resource "kubernetes_namespace_v1" "example-app" {
  metadata {
    name = "example-app"
  }
}

variable "harbor_credentials" {
  description = "Credentials for the kubernetes robot on harbor"
  sensitive   = true
  type        = string
}

resource "kubernetes_secret_v1" "harbor_credentials" {
  metadata {
    name      = "harbor-credentials"
    namespace = kubernetes_namespace_v1.example-app.metadata[0].name
  }
  type = "kubernetes.io/dockerconfigjson"
  data = {
    ".dockerconfigjson" = var.harbor_credentials
  }
}
