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

variable "public_ip" {
  description = "The public ip of the cluster"
  type        = string
  default     = "95.90.150.138"
}
