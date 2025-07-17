# ocloud - Cloud File Storage Server Documentation

A production-ready Rust-based cloud file storage server built with Axum, SQLx, and PostgreSQL, following Zero to Production best practices.

## Features

- **File Upload & Management**: RESTful API for file operations
- **Database Storage**: PostgreSQL with SQLx for metadata and file tracking
- **Production-Ready**: Comprehensive error handling, logging, and observability
- **Testing Infrastructure**: Unit and integration tests with database isolation
- **CI/CD Pipeline**: Automated testing, formatting, and linting
- **Security**: Input validation, CORS support, and secure configuration management

## Quick Start

### Prerequisites

- Rust 1.70+ 
- Docker and Docker Compose
- PostgreSQL (via Docker)

### Setup

1. **Start the database**:
   ```bash
   docker-compose up -d
   ```

2. **Run database migrations**:
   ```bash
   sqlx migrate run
   ```

3. **Start the server**:
   ```bash
   cargo run -- server run
   ```

The server will start on `http://localhost:8000` by default.

## Project Structure

```
ocloud/
├── src/
│   ├── cli/              # Command-line interface
│   ├── config/           # Configuration management
│   └── server/           # Web server components
│       ├── controllers/  # Business logic
│       ├── web/         # HTTP handlers and middleware
│       ├── error.rs     # Structured error handling
│       ├── validation.rs # Input validation
│       └── db_utils.rs  # Database utilities
├── tests/               # Integration tests
├── migrations/          # Database migrations
├── .github/workflows/   # CI/CD pipeline
└── docker-compose.yml   # Development database
```

## Configuration

The application uses a hierarchical configuration system:

1. **Default Configuration**: Hardcoded defaults in `src/config/server.rs`
2. **TOML File**: `server.toml` (auto-generated on first run)
3. **Environment Variables**: Override any setting

### Configuration Files

- `server.toml`: Server configuration (auto-generated)
- `compose.yaml`: Development PostgreSQL database

### Environment Variables

```bash
# Database
POSTGRES_HOST=localhost
POSTGRES_PORT=9432
POSTGRES_USER=user
POSTGRES_PASSWORD=pass
POSTGRES_DATABASE=ocloud

# Application
HOST=0.0.0.0
PORT=8000
```

## Testing

The project includes comprehensive testing infrastructure with database isolation.

### Test Types

1. **Unit Tests**: Test individual functions and modules
2. **Integration Tests**: Test HTTP endpoints with real database

### Running Tests

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test health_check

# Run tests with logging
TEST_LOG=1 cargo test
```

### Test Database

Integration tests automatically:
- Create isolated test databases for each test
- Run migrations on test databases
- Clean up databases after tests complete

### Adding New Tests

#### Unit Tests

Add unit tests directly in the module files:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_your_function() {
        // Test implementation
    }
}
```

#### Integration Tests

Create new test files in the `tests/` directory:

```rust
// tests/my_new_test.rs
mod common;

use common::TestApp;

#[tokio::test]
async fn test_my_endpoint() {
    let app = TestApp::spawn().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/my-endpoint", &app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
}
```

#### Test Utilities

The `tests/common/mod.rs` provides utilities for integration testing:

```rust
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub db_name: String,
}

impl TestApp {
    pub async fn spawn() -> TestApp {
        // Creates isolated test environment
    }
}
```

Each test gets:
- Unique random port
- Isolated test database
- Proper cleanup after completion

## CI/CD Pipeline

The project includes a comprehensive GitHub Actions pipeline (`.github/workflows/ci.yml`):

### Pipeline Steps

1. **Setup**: Install Rust toolchain and start PostgreSQL
2. **Dependencies**: Cache and install dependencies
3. **Format Check**: Verify code formatting with `rustfmt`
4. **Lint**: Run `clippy` for code quality
5. **Test**: Run all tests with database
6. **Coverage**: Generate test coverage reports

### Running CI Locally

```bash
# Format check
cargo fmt --check

# Linting
cargo clippy -- -D warnings

# Tests
cargo test

# All checks
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

### Setting Up CI

The pipeline is automatically triggered on:
- Push to `main` branch
- Pull requests to `main`

Required secrets: None (uses public PostgreSQL service)

## Error Handling

The application uses structured error handling with the `thiserror` crate:

```rust
#[derive(Error, Debug, Clone)]
pub enum ServerError {
    #[error("Database query failed: {message}")]
    DatabaseQueryError { message: String },
    
    #[error("File not found: {filename}")]
    FileNotFound { filename: String },
    // ... other variants
}
```

### Error Features

- **Structured Errors**: Consistent error types with detailed context
- **HTTP Status Mapping**: Automatic conversion to appropriate HTTP status codes
- **Logging Integration**: Errors are automatically logged with context
- **Client-Safe Messages**: Sensitive details are filtered from client responses

## Logging and Observability

### Request Tracing

Every HTTP request gets:
- Unique request ID (UUID)
- Request timing
- Automatic logging of request/response

```rust
// Middleware automatically adds tracing
pub async fn trace_request(request: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    // ... tracing logic
}
```

### Logging Configuration

```bash
# Enable debug logging for tests
TEST_LOG=1 cargo test

# Set log level
RUST_LOG=debug cargo run

# Production logging (errors only for tests)
cargo test  # Automatically uses ERROR level
```

## Database

### Schema Management

Database schema is managed through SQLx migrations in the `migrations/` directory:

```bash
# Create new migration
sqlx migrate add create_users_table

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

### Database Utilities

The `db_utils.rs` module provides transaction utilities:

```rust
use crate::server::db_utils::execute_in_transaction;

let result = execute_in_transaction(&pool, |tx| async move {
    // Your database operations here
    sqlx::query!("INSERT INTO files ...")
        .execute(tx)
        .await?;
    Ok(())
}).await?;
```

### Connection Management

- **Connection Pooling**: SQLx PgPool for efficient connection management
- **Lazy Connections**: Connections are established on first use
- **Migration Support**: Automatic migration running on startup

## Input Validation

The application includes comprehensive input validation:

```rust
use crate::server::validation::{validate_filename, validate_path};

// Validate file names
validate_filename("my-file.txt")?;

// Validate file paths
validate_path("/safe/path")?;

// Sanitize path components
let clean = sanitize_path_component("user input");
```

### Validation Rules

- **Filename Validation**: No empty names, path traversal protection
- **Path Validation**: Prevents directory traversal attacks
- **Sanitization**: Removes dangerous characters from user input

## Development Workflow

### Getting Started

1. **Clone and setup**:
   ```bash
   git clone <repo>
   cd ocloud/app
   docker-compose up -d
   ```

2. **Run migrations**:
   ```bash
   sqlx migrate run
   ```

3. **Start development**:
   ```bash
   cargo run -- server run
   ```

### Development Commands

```bash
# Format code
cargo fmt

# Check code
cargo clippy

# Run tests with logging
TEST_LOG=1 cargo test

# Watch for changes
cargo watch -x check -x test

# Database operations
sqlx migrate run
sqlx migrate revert
```

### Pre-commit Checklist

Before committing code:

```bash
# 1. Format code
cargo fmt

# 2. Fix clippy warnings
cargo clippy --fix

# 3. Run all tests
cargo test

# 4. Check CI pipeline will pass
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

## Production Deployment

### Build for Production

```bash
# Build optimized binary
cargo build --release

# Binary location
./target/release/ocloud
```

### Environment Setup

```bash
# Set production database
export POSTGRES_HOST=prod-db-host
export POSTGRES_PORT=5432
export POSTGRES_USER=prod_user
export POSTGRES_PASSWORD=secure_password
export POSTGRES_DATABASE=ocloud_prod

# Run migrations
./target/release/ocloud migrate

# Start server
./target/release/ocloud server run --host 0.0.0.0 --port 8000
```

### Production Considerations

1. **Database**: Use managed PostgreSQL service
2. **Secrets**: Use environment variables or secret management
3. **Logging**: Configure structured logging for production
4. **Monitoring**: Set up health checks and metrics
5. **SSL**: Use reverse proxy (nginx/caddy) for HTTPS

## API Documentation

### Endpoints

Based on the existing API design:

- `GET /health` - Health check endpoint
- `GET /files/{path}` - Download file or list directory
- `POST /files/{path}` - Upload file or create directory
- `DELETE /files/{path}` - Delete file

### Health Check

```bash
curl http://localhost:8000/health
# Returns: 200 OK (empty body)
```

### File Operations

```bash
# Upload a file
curl -X POST http://localhost:8000/files/my-file.txt \
  -F "file=@local-file.txt"

# Download a file
curl http://localhost:8000/files/my-file.txt

# List directory
curl http://localhost:8000/files/my-folder/

# Delete a file
curl -X DELETE http://localhost:8000/files/my-file.txt
```

## Testing Infrastructure Deep Dive

### Test Database Isolation

Each integration test runs in complete isolation:

1. **Unique Database**: Every test gets a randomly named database
2. **Fresh Migrations**: Each database runs the full migration suite
3. **Automatic Cleanup**: Databases are dropped after test completion
4. **No Shared State**: Tests can run in parallel without interference

### Test App Creation

```rust
let app = TestApp::spawn().await;
// app.address contains unique server URL (e.g., http://127.0.0.1:45678)
// app.db_pool contains isolated database connection
// app.db_name contains unique database name (e.g., test_a1b2c3d4)
```

### Test Server Architecture

The test server:
- Binds to random available port
- Uses isolated test database
- Runs all the same middleware as production
- Includes proper error handling and logging

### Adding Integration Tests

When adding new integration tests:

1. **Create test file**: `tests/my_feature_test.rs`
2. **Import common utilities**: `mod common; use common::TestApp;`
3. **Use `#[tokio::test]`**: For async test functions
4. **Spawn test app**: `let app = TestApp::spawn().await;`
5. **Make HTTP requests**: Use `reqwest::Client` for API testing

Example test structure:
```rust
mod common;
use common::TestApp;

#[tokio::test]
async fn test_file_upload() {
    // Arrange
    let app = TestApp::spawn().await;
    let client = reqwest::Client::new();
    
    // Act
    let response = client
        .post(&format!("{}/files/test.txt", &app.address))
        .multipart(/* file data */)
        .send()
        .await
        .expect("Failed to send request");
    
    // Assert
    assert!(response.status().is_success());
}
```

## CI/CD Pipeline Details

### GitHub Actions Workflow

The `.github/workflows/ci.yml` file defines a comprehensive CI pipeline:

```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_USER: user
          POSTGRES_PASSWORD: pass
          POSTGRES_DB: postgres
        ports:
          - 9432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
```

### Pipeline Stages

1. **Environment Setup**:
   - Ubuntu latest runner
   - PostgreSQL 15 service
   - Rust toolchain installation

2. **Dependency Management**:
   - Cargo cache for faster builds
   - Dependency installation

3. **Code Quality Checks**:
   - Format verification (`cargo fmt --check`)
   - Linting (`cargo clippy -- -D warnings`)

4. **Testing**:
   - Unit tests (`cargo test --lib`)
   - Integration tests (`cargo test --test`)
   - Full test suite (`cargo test`)

5. **Coverage** (optional):
   - Test coverage generation
   - Coverage reporting

### Local CI Simulation

To simulate CI locally:

```bash
# Install required tools
rustup component add rustfmt clippy

# Run the same checks as CI
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# Or run all at once
./scripts/ci-check.sh  # If you create this script
```

## Zero to Production Features

This project implements production-ready patterns from "Zero to Production in Rust":

### ✅ Testing Infrastructure
- **Unit Tests**: Test individual functions with `#[test]`
- **Integration Tests**: Test HTTP endpoints with real database
- **Database Isolation**: Each test gets unique database
- **Test Utilities**: Shared test infrastructure in `tests/common/`

### ✅ CI/CD Pipeline
- **Automated Testing**: All tests run on every commit
- **Code Quality**: Format and lint checks
- **Database Integration**: PostgreSQL service in CI
- **Pull Request Validation**: Prevents broken code merging

### ✅ Structured Error Handling
- **Custom Error Types**: Using `thiserror` for structured errors
- **HTTP Status Mapping**: Errors automatically map to HTTP status codes
- **Error Context**: Detailed error information with context
- **Client-Safe Errors**: Sensitive information filtered from responses

### ✅ Request Tracing
- **Unique Request IDs**: Every request gets UUID
- **Request Timing**: Track request duration
- **Structured Logging**: Consistent log format
- **Request/Response Logging**: Automatic logging of HTTP traffic

### ✅ Configuration Management
- **Environment Variables**: Production configuration via env vars
- **Configuration Files**: TOML-based config with defaults
- **Hierarchical Config**: Environment overrides file overrides defaults
- **Secure Secrets**: Sensitive data handled via environment

### ✅ Input Validation
- **Path Validation**: Prevent directory traversal attacks
- **Filename Validation**: Sanitize file names
- **Input Sanitization**: Clean user input before processing
- **Security-First**: Reject dangerous input patterns

### ✅ Database Management
- **Connection Pooling**: Efficient database connections
- **Migrations**: Version-controlled schema changes
- **Transactions**: ACID compliance with transaction utilities
- **Lazy Connections**: Connect only when needed

### ✅ Observability
- **Structured Logging**: JSON-formatted logs for production
- **Request Tracing**: Track requests across the system
- **Error Monitoring**: Comprehensive error tracking
- **Performance Metrics**: Request timing and database metrics

## Contributing

1. **Fork the repository**
2. **Create feature branch**: `git checkout -b feature/my-feature`
3. **Write tests** for your changes
4. **Ensure CI passes**: Run the pre-commit checklist
5. **Submit pull request**

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Address all clippy warnings
- Write tests for new functionality
- Update documentation for API changes

## Troubleshooting

### Common Issues

1. **Database Connection Failed**:
   ```bash
   # Start database
   docker-compose up -d
   
   # Check database is running
   docker-compose ps
   ```

2. **Tests Failing**:
   ```bash
   # Clean rebuild
   cargo clean
   cargo test
   
   # Run with logs
   TEST_LOG=1 cargo test
   ```

3. **Port Already in Use**:
   ```bash
   # Use different port
   cargo run -- server run --port 8001
   ```

4. **Migration Errors**:
   ```bash
   # Reset database
   docker-compose down -v
   docker-compose up -d
   sqlx migrate run
   ```

5. **Integration Tests Timeout**:
   ```bash
   # Increase test timeout
   cargo test -- --test-threads=1
   
   # Check database connectivity
   psql -h localhost -p 9432 -U user -d postgres
   ```

### Database Troubleshooting

1. **Check Database Status**:
   ```bash
   docker-compose ps
   docker-compose logs postgres
   ```

2. **Manual Database Connection**:
   ```bash
   # Connect to database
   psql -h localhost -p 9432 -U user -d ocloud
   
   # List tables
   \dt
   
   # Check migrations
   SELECT * FROM _sqlx_migrations;
   ```

3. **Reset Everything**:
   ```bash
   # Stop and remove all containers and volumes
   docker-compose down -v
   
   # Remove build artifacts
   cargo clean
   
   # Start fresh
   docker-compose up -d
   sqlx migrate run
   cargo test
   ```

### CI/CD Troubleshooting

1. **GitHub Actions Failing**:
   - Check the Actions tab in your GitHub repository
   - Look for specific error messages in the job logs
   - Ensure PostgreSQL service is starting correctly

2. **Local CI Mismatch**:
   ```bash
   # Use exact same commands as CI
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   ```

3. **Cache Issues**:
   - Clear GitHub Actions cache if builds are inconsistent
   - Use `cargo clean` locally to clear Rust cache

### Getting Help

- Check the logs for detailed error messages
- Ensure PostgreSQL is running on port 9432
- Verify environment variables are set correctly
- Run tests to ensure everything is working: `cargo test`
- Check that all dependencies are installed: `cargo check`

## Next Steps

Consider implementing these additional production features:

1. **Authentication & Authorization**: JWT tokens, user management
2. **Rate Limiting**: Request rate limiting per IP/user
3. **Caching**: Redis cache for frequently accessed files
4. **Metrics**: Prometheus metrics for monitoring
5. **Health Checks**: Detailed health endpoints with dependency checks
6. **API Documentation**: OpenAPI/Swagger documentation
7. **Container Deployment**: Docker images and Kubernetes manifests
8. **Backup & Recovery**: Automated database backups
9. **Security Headers**: Additional security middleware
10. **Load Testing**: Performance testing infrastructure