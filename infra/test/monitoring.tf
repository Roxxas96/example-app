resource "kubernetes_namespace_v1" "kube_prometheus" {
  metadata {
    name = "kube-prometheus"
  }
}

variable "prometheus_crds_chart_version" {
  description = "Version for the kube-prometheus-stack chart"
  type        = string
  default     = "20.0.0"
}

resource "helm_release" "prometheus_crds" {
  name      = "prometheus-crds"
  namespace = kubernetes_namespace_v1.kube_prometheus.metadata[0].name

  chart      = "prometheus-operator-crds"
  repository = "https://prometheus-community.github.io/helm-charts"
  version    = var.prometheus_crds_chart_version
}
