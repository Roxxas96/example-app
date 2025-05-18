data "cloudflare_zone" "roxxas96_dot_net" {
  zone_id = "7b8a1b76ded8189b4e25422a6aed2882"
}

resource "cloudflare_dns_record" "harbor" {
  zone_id = data.cloudflare_zone.roxxas96_dot_net.id
  name    = "harbor"
  type    = "A"
  ttl     = 1
  content = var.public_ip
}
