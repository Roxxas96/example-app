resource "kubernetes_namespace_v1" "harbor" {
  metadata {
    name = "harbor"
  }
}

variable "harbor_chart_version" {
  description = "Version to use for the harbor chart"
  type        = string
  default     = "24.5.0"
}

resource "helm_release" "harbor" {
  name      = "harbor"
  namespace = kubernetes_namespace_v1.harbor.metadata[0].name

  chart   = "oci://registry-1.docker.io/bitnamicharts/harbor"
  version = var.harbor_chart_version

  values = [templatefile("./templates/harbor.values.yaml", {
    host = "harbor.internal.${data.cloudflare_zone.roxxas96_dot_net.name}"
  })]
}

resource "cloudflare_dns_record" "harbor_internal" {
  zone_id = data.cloudflare_zone.roxxas96_dot_net.id
  name    = "harbor.internal"
  type    = "A"
  ttl     = 1
  content = var.loadbalancer_ip
}
