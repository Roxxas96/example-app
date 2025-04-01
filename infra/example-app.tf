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
              "harbor.internal.${data.cloudflare_zone.roxxas96_dot_net.name}": {
                  "ServerURL":"https://harbor.internal.${data.cloudflare_zone.roxxas96_dot_net.name}",
                  "Username":"${var.harbor_docker_username}",
                  "Secret":"${var.harbor_docker_password}"
              }
          }
      }
      EOF
  }
}
