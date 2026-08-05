# Example Terraform configuration for dependency discovery
# Run: pyblastradius scan --terraform .

terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = "us-east-1"
}

# ============================================================================
# RDS - PostgreSQL Database (CRITICAL)
# ============================================================================
resource "aws_db_instance" "production" {
  identifier              = "production-postgres"
  engine                  = "postgres"
  engine_version          = "15.3"
  instance_class          = "db.t3.medium"
  allocated_storage       = 100
  storage_encrypted       = true
  skip_final_snapshot     = false
  final_snapshot_identifier = "production-postgres-final-snapshot"

  db_name  = "production"
  username = "postgres"
  password = random_password.db_password.result

  backup_retention_period      = 30
  backup_window                = "03:00-04:00"
  maintenance_window           = "sun:04:00-sun:05:00"
  multi_az                     = true
  publicly_accessible          = false
  vpc_security_group_ids       = [aws_security_group.database.id]
  db_subnet_group_name         = aws_db_subnet_group.default.name

  tags = {
    Name        = "production-db"
    Criticality = "critical"
    Owner       = "data-team"
  }
}

resource "random_password" "db_password" {
  length  = 32
  special = true
}

# ============================================================================
# ElastiCache - Redis Cache
# ============================================================================
resource "aws_elasticache_cluster" "redis" {
  cluster_id           = "production-redis"
  engine               = "redis"
  node_type            = "cache.t3.micro"
  num_cache_nodes      = 3
  parameter_group_name = aws_elasticache_parameter_group.default.name
  engine_version       = "7.0"
  port                 = 6379

  automatic_failover_enabled = true
  multi_az_enabled          = true
  security_group_ids        = [aws_security_group.redis.id]
  subnet_group_name         = aws_elasticache_subnet_group.default.name

  tags = {
    Name  = "production-redis"
    Owner = "platform-team"
  }

  depends_on = [aws_db_instance.production]
}

# ============================================================================
# ECS - Container Orchestration
# ============================================================================
resource "aws_ecs_cluster" "production" {
  name = "production-cluster"

  setting {
    name  = "containerInsights"
    value = "enabled"
  }

  tags = {
    Name = "production-cluster"
  }
}

# API Gateway Service
resource "aws_ecs_service" "api_gateway" {
  name            = "api-gateway"
  cluster         = aws_ecs_cluster.production.id
  task_definition = aws_ecs_task_definition.api_gateway.arn
  desired_count   = 3

  load_balancer {
    target_group_arn = aws_lb_target_group.api_gateway.arn
    container_name   = "api-gateway"
    container_port   = 8080
  }

  depends_on = [
    aws_ecs_task_definition.api_gateway,
    aws_lb_target_group.api_gateway,
  ]

  tags = {
    Name = "api-gateway-service"
  }
}

resource "aws_ecs_task_definition" "api_gateway" {
  family                   = "api-gateway"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "256"
  memory                   = "512"

  container_definitions = jsonencode([{
    name      = "api-gateway"
    image     = "myregistry/api-gateway:latest"
    essential = true
    portMappings = [{
      containerPort = 8080
      hostPort      = 8080
      protocol      = "tcp"
    }]
    environment = [
      {
        name  = "AUTH_SERVICE_URL"
        value = "http://${aws_ecs_service.auth_service.name}:8080"
      },
      {
        name  = "DATABASE_URL"
        value = "postgresql://${aws_db_instance.production.endpoint}/production"
      },
    ]
  }])

  depends_on = [aws_db_instance.production]
}

# Authentication Service
resource "aws_ecs_service" "auth_service" {
  name            = "auth-service"
  cluster         = aws_ecs_cluster.production.id
  task_definition = aws_ecs_task_definition.auth_service.arn
  desired_count   = 2

  load_balancer {
    target_group_arn = aws_lb_target_group.auth_service.arn
    container_name   = "auth-service"
    container_port   = 8080
  }

  depends_on = [
    aws_ecs_task_definition.auth_service,
    aws_elasticache_cluster.redis,
    aws_db_instance.production,
  ]

  tags = {
    Name        = "auth-service"
    Criticality = "high"
  }
}

resource "aws_ecs_task_definition" "auth_service" {
  family                   = "auth-service"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "256"
  memory                   = "512"

  container_definitions = jsonencode([{
    name      = "auth-service"
    image     = "myregistry/auth-service:latest"
    essential = true
    portMappings = [{
      containerPort = 8080
      hostPort      = 8080
      protocol      = "tcp"
    }]
    environment = [
      {
        name  = "DATABASE_URL"
        value = "postgresql://${aws_db_instance.production.endpoint}/production"
      },
      {
        name  = "REDIS_URL"
        value = "redis://${aws_elasticache_cluster.redis.cache_nodes[0].address}:6379"
      },
    ]
  }])

  depends_on = [
    aws_db_instance.production,
    aws_elasticache_cluster.redis,
  ]
}

# Payment Service
resource "aws_ecs_service" "payment_service" {
  name            = "payment-service"
  cluster         = aws_ecs_cluster.production.id
  task_definition = aws_ecs_task_definition.payment_service.arn
  desired_count   = 2

  load_balancer {
    target_group_arn = aws_lb_target_group.payment_service.arn
    container_name   = "payment-service"
    container_port   = 8080
  }

  depends_on = [
    aws_ecs_task_definition.payment_service,
    aws_db_instance.production,
  ]

  tags = {
    Name        = "payment-service"
    Criticality = "critical"
  }
}

resource "aws_ecs_task_definition" "payment_service" {
  family                   = "payment-service"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "512"
  memory                   = "1024"

  container_definitions = jsonencode([{
    name      = "payment-service"
    image     = "myregistry/payment-service:latest"
    essential = true
    portMappings = [{
      containerPort = 8080
      hostPort      = 8080
      protocol      = "tcp"
    }]
    environment = [
      {
        name  = "DATABASE_URL"
        value = "postgresql://${aws_db_instance.production.endpoint}/production"
      },
    ]
  }])

  depends_on = [aws_db_instance.production]
}

# ============================================================================
# Load Balancer
# ============================================================================
resource "aws_lb" "main" {
  name               = "production-lb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.lb.id]
  subnets            = aws_subnet.public[*].id

  enable_deletion_protection = true

  tags = {
    Name = "production-lb"
  }
}

resource "aws_lb_target_group" "api_gateway" {
  name        = "api-gateway-tg"
  port        = 8080
  protocol    = "HTTP"
  vpc_id      = aws_vpc.main.id
  target_type = "ip"

  health_check {
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout             = 3
    interval            = 30
    path                = "/health"
    matcher             = "200"
  }
}

resource "aws_lb_target_group" "auth_service" {
  name        = "auth-service-tg"
  port        = 8080
  protocol    = "HTTP"
  vpc_id      = aws_vpc.main.id
  target_type = "ip"
}

resource "aws_lb_target_group" "payment_service" {
  name        = "payment-service-tg"
  port        = 8080
  protocol    = "HTTP"
  vpc_id      = aws_vpc.main.id
  target_type = "ip"
}

# ============================================================================
# VPC & Networking
# ============================================================================
resource "aws_vpc" "main" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_hostnames = true
  enable_dns_support   = true

  tags = {
    Name = "production-vpc"
  }
}

resource "aws_subnet" "public" {
  count = 2

  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.0.${count.index + 1}.0/24"
  availability_zone       = data.aws_availability_zones.available.names[count.index]
  map_public_ip_on_launch = true

  tags = {
    Name = "public-subnet-${count.index + 1}"
  }
}

resource "aws_subnet" "private" {
  count = 2

  vpc_id            = aws_vpc.main.id
  cidr_block        = "10.0.${count.index + 10}.0/24"
  availability_zone = data.aws_availability_zones.available.names[count.index]

  tags = {
    Name = "private-subnet-${count.index + 1}"
  }
}

# ============================================================================
# Security Groups
# ============================================================================
resource "aws_security_group" "database" {
  name        = "database-sg"
  description = "Security group for RDS database"
  vpc_id      = aws_vpc.main.id

  ingress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/16"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_security_group" "redis" {
  name        = "redis-sg"
  description = "Security group for Redis cache"
  vpc_id      = aws_vpc.main.id

  ingress {
    from_port   = 6379
    to_port     = 6379
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/16"]
  }
}

resource "aws_security_group" "lb" {
  name        = "load-balancer-sg"
  description = "Security group for load balancer"
  vpc_id      = aws_vpc.main.id

  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

# ============================================================================
# Data Sources
# ============================================================================
data "aws_availability_zones" "available" {
  state = "available"
}

resource "aws_db_subnet_group" "default" {
  name       = "production-db-subnet-group"
  subnet_ids = aws_subnet.private[*].id

  tags = {
    Name = "production-db-subnet-group"
  }
}

resource "aws_elasticache_subnet_group" "default" {
  name       = "production-elasticache-subnet-group"
  subnet_ids = aws_subnet.private[*].id
}

# ============================================================================
# Outputs
# ============================================================================
output "database_endpoint" {
  value       = aws_db_instance.production.endpoint
  description = "RDS database endpoint"
}

output "redis_endpoint" {
  value       = aws_elasticache_cluster.redis.cache_nodes[0].address
  description = "Redis cluster endpoint"
}

output "load_balancer_dns" {
  value       = aws_lb.main.dns_name
  description = "Load balancer DNS name"
}

output "ecs_cluster_name" {
  value       = aws_ecs_cluster.production.name
  description = "ECS cluster name"
}
