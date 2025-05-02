variable "harbor_host" {
  description = "Host to Harbor"
  type        = string
  default     = "harbor.roxxas96.net"
}

variable "harbor_docker_username" {
  description = "Username for the kubernetes robot on harbor docker repo"
  sensitive   = true
  type        = string
}

variable "harbor_docker_password" {
  description = "Username for the kubernetes robot on harbor docker repo"
  sensitive   = true
  type        = string
}
