data "cloudflare_zone" "roxxas96_dot_net" {
  zone_id = "7b8a1b76ded8189b4e25422a6aed2882"
}

data "kubernetes_namespace_v1" "argocd" {
  metadata {
    name = "argocd"
  }
}

resource "kubernetes_manifest" "app_project_example_app" {
  manifest = {
    apiVersion = "argoproj.io/v1alpha1"
    kind       = "AppProject"
    metadata = {
      name       = "example-app"
      namespace  = data.kubernetes_namespace_v1.argocd.metadata[0].name
      finalizers = ["resources-finalizer.argocd.argoproj.io"]
    }
    spec = {
      description = "Project containing all home infra applications"
      sourceRepos = ["*"]
      destinations = [
        {
          server    = "*"
          namespace = "*"
        }
      ]
      clusterResourceWhitelist = [
        {
          group = "*"
          kind  = "*"
        }
      ]
      sourceNamespaces = ["*"]
    }
  }
}
