resource "kubernetes_namespace_v1" "harbor" {
  metadata {
    name = "harbor"
  }
}

resource "kubernetes_manifest" "application_harbor" {
  manifest = {
    apiVersion = "argoproj.io/v1alpha1"
    kind       = "Application"
    metadata = {
      name       = "harbor"
      namespace  = kubernetes_namespace_v1.harbor.metadata[0].name
      finalizers = ["resources-finalizer.argocd.argoproj.io"]
    }
    spec = {
      project = kubernetes_manifest.app_project_example_app.manifest.metadata.name
      source = {
        repoURL        = "registry-1.docker.io/bitnamicharts"
        targetRevision = "*.*.*"
        chart          = "harbor"
        helm = {
          values = templatefile("./templates/harbor.values.yaml", {
            host = "harbor.${data.cloudflare_zone.roxxas96_dot_net.name}"
          })
        }
      }
      destination = {
        server    = "https://kubernetes.default.svc"
        namespace = kubernetes_namespace_v1.harbor.metadata[0].name
      }
    }
  }
}

resource "kubernetes_ingress_v1" "harbor_internal" {
  metadata {
    name      = "harbor-internal"
    namespace = kubernetes_namespace_v1.harbor.metadata[0].name
    annotations = {
      "cert-manager.io/cluster-issuer"              = "internal-prod"
      "nginx.ingress.kubernetes.io/proxy-body-size" = "0"
    }
  }
  spec {
    ingress_class_name = "nginx"
    rule {
      host = "harbor.internal.${data.cloudflare_zone.roxxas96_dot_net.name}"
      http {
        path {
          path      = "/api/"
          path_type = "ImplementationSpecific"
          backend {
            service {
              name = "harbor-core"
              port {
                name = "http"
              }
            }
          }
        }
        path {
          path      = "/service/"
          path_type = "ImplementationSpecific"
          backend {
            service {
              name = "harbor-core"
              port {
                name = "http"
              }
            }
          }
        }
        path {
          path      = "/v2"
          path_type = "ImplementationSpecific"
          backend {
            service {
              name = "harbor-core"
              port {
                name = "http"
              }
            }
          }
        }
        path {
          path      = "/chartrepo/"
          path_type = "ImplementationSpecific"
          backend {
            service {
              name = "harbor-core"
              port {
                name = "http"
              }
            }
          }
        }
        path {
          path      = "/c/"
          path_type = "ImplementationSpecific"
          backend {
            service {
              name = "harbor-core"
              port {
                name = "http"
              }
            }
          }
        }
        path {
          path      = "/"
          path_type = "ImplementationSpecific"
          backend {
            service {
              name = "harbor-portal"
              port {
                name = "http"
              }
            }
          }
        }
      }
    }
    tls {
      hosts       = ["harbor.internal.${data.cloudflare_zone.roxxas96_dot_net.name}"]
      secret_name = "harbor-internal-tls"
    }
  }
}

resource "cloudflare_dns_record" "harbor_internal" {
  zone_id = data.cloudflare_zone.roxxas96_dot_net.id
  name    = "harbor.internal"
  type    = "A"
  ttl     = 1
  content = var.loadbalancer_ip
}

resource "cloudflare_dns_record" "harbor" {
  zone_id = data.cloudflare_zone.roxxas96_dot_net.id
  name    = "harbor"
  type    = "A"
  ttl     = 1
  content = var.public_ip
}

variable "harbor_helm_username" {
  description = "Username for the kubernetes robot on harbor helm repo"
  sensitive   = true
  type        = string
}

variable "harbor_helm_password" {
  description = "Username for the kubernetes robot on harbor helm repo"
  sensitive   = true
  type        = string
}

resource "kubernetes_secret_v1" "name" {
  metadata {
    name      = "harbor-repo"
    namespace = data.kubernetes_namespace_v1.argocd.metadata[0].name
    labels = {
      "argocd.argoproj.io/secret-type" = "repository"
    }
  }
  data = {
    "enableOCI" = "true"
    "name"      = "harbor"
    "type"      = "helm"
    "url"       = "harbor.internal.${data.cloudflare_zone.roxxas96_dot_net.name}"
    "username"  = var.harbor_helm_username
    "password"  = var.harbor_helm_password
  }
}
