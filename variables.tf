variable "backend_url" {
  type    = string
  default = "backend.example.com"
}

terraform {
  backend "s3" {
    bucket = "my-bucket"
    key    = "path/to/state"
    region = "us-east-1"
  }
}
