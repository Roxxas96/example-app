variable "loadbalancer_ip" {
  description = "The IP of the loadbalancer in the local network"
  type        = string
  default     = "192.168.0.240"
}

variable "cloudflare_api_token" {
  description = "The token to talk to the cloudflare API"
  type        = string
  sensitive   = true
}
