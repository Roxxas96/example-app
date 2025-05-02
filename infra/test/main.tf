resource "kubernetes_namespace_v1" "chart-testing" {
  metadata {
    name = "chart-testing"
  }
}

resource "kubernetes_secret_v1" "harbor_credentials" {
  metadata {
    name      = "harbor-credentials"
    namespace = kubernetes_namespace_v1.chart-testing.metadata[0].name
  }
  type = "kubernetes.io/dockerconfigjson"
  data = {
    ".dockerconfigjson" = <<EOF
      {
        "auths": {
          "${var.harbor_host}": {
            "auth": "${base64encode("${var.harbor_docker_username}:${var.harbor_docker_password}")}"
          }
        }
      }
      EOF
  }
}
