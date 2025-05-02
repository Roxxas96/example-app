resource "kubernetes_namespace_v1" "kube_prometheus" {
  metadata {
    name = "kube-prometheus"
  }
}

variable "kube_prometheus_chart_version" {
  description = "Version for the kube-prometheus-stack chart"
  type        = string
  default     = "70.4.1"
}

resource "helm_release" "kube_prometheus" {
  name      = "kube-prometheus"
  namespace = kubernetes_namespace_v1.kube_prometheus.metadata[0].name

  chart      = "kube-prometheus-stack"
  repository = "https://prometheus-community.github.io/helm-charts"
  version    = var.kube_prometheus_chart_version
}
