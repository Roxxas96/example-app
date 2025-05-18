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
