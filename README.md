# NetGate

NetGate is a small demo middleware sitting between a frontend "order portal" and NetBox (infrastructure source of truth).

## Requirements Satisfied

### Technology Stack

- **Language**: Rust stable
- **Runtime**: tokio (async runtime)
- **Web Framework**: poem + poem-openapi
- **HTTP Client**: reqwest (for future NetBox integration)
- **Serialization**: serde
- **Logging**: tracing

### Implemented Endpoints

1. **GET /health** - Simple health check endpoint
2. **POST /orders/site** - Accepts a JSON `CreateSiteOrder` and creates a site for a tenant (in-memory mock)
3. **GET /tenants/{tenant_id}/sites** - Returns a list of sites for that tenant (from in-memory storage)

### Features

- ✅ **Tenant Separation**: Tenant is identified by header `X-Tenant-Id`
- ✅ **Tenant Isolation**: Tenant separation is enforced - a tenant can only see its own sites
- ✅ **OpenAPI Specification**: Uses poem-openapi to define types and generate OpenAPI spec
- ✅ **In-Memory Storage**: Uses `RwLock<HashMap<...>>` to keep sites per tenant
- ✅ **Comprehensive Testing**: Unit tests and integration tests included

### Project Structure

```
src/
  main.rs          - Application entry point, server setup
  config.rs        - Configuration from environment variables
  logging.rs       - Tracing/logging initialization
  error.rs         - Error types and handling
  
  api/
    mod.rs        - API module exports
    health.rs     - Health check endpoint
    orders.rs     - Site order creation endpoint
    tenants.rs    - Tenant sites retrieval endpoint
  
  domain/
    mod.rs        - Domain module exports
    order.rs      - Order and Site domain models
    tenant.rs     - Tenant store (in-memory storage)
  
  netbox/
    mod.rs        - NetBox module exports
    client.rs     - NetBox HTTP client (placeholder)
    models.rs     - NetBox data models (placeholder)
  
  security/
    mod.rs        - Security module exports
    auth.rs       - Tenant ID extraction from headers
```

## Getting Started

### Prerequisites

- Rust stable (1.70+)
- Cargo

### Building

```bash
cargo build
```

### Running

```bash
# Using cargo
cargo run

# Using make
make

# Or with custom port
PORT=9090 cargo run
```

The server will start on port 8080 by default (configurable via `PORT` environment variable).

### Environment Variables

- `PORT` - Server port (default: 8080)
- `NETBOX_URL` - NetBox API URL (default: http://localhost:8000)
- `NETBOX_TOKEN` - NetBox API token (default: empty string)

## API Documentation

Once the server is running, you can access:

- **Swagger UI**: http://localhost:8080/docs
- **OpenAPI Spec**: http://localhost:8080/spec

## Testing

### Unit Tests

Run all unit tests:

```bash
cargo test --lib
# or
make test
```

### Integration Tests

Integration tests require the server to be running. They are marked with `#[ignore]` by default.

1. Start the server in one terminal:
   ```bash
   cargo run
   ```

2. Run integration tests in another terminal:
   ```bash
   cargo test --test integration_test -- --ignored
   ```

### Test Coverage

- **12 unit tests** covering:
  - Domain logic (tenant store, site creation)
  - Configuration parsing
  - Security (tenant ID extraction)
  
- **7 integration tests** covering:
  - All API endpoints
  - Tenant isolation
  - Error handling (missing headers, mismatched tenant IDs)

## API Usage Examples

### Health Check

```bash
curl http://localhost:8080/health
```

### Create Site Order

```bash
curl -X POST http://localhost:8080/orders/site \
  -H "Content-Type: application/json" \
  -H "X-Tenant-Id: tenant1" \
  -d '{
    "name": "My Site",
    "description": "A test site",
    "address": "123 Main St"
  }'
```

### Get Sites for Tenant

```bash
curl http://localhost:8080/tenants/tenant1/sites \
  -H "X-Tenant-Id: tenant1"
```

## Architecture

### Tenant Isolation

NetGate enforces strict tenant separation:

1. All requests must include the `X-Tenant-Id` header
2. Sites are stored per tenant in an in-memory `HashMap`
3. When retrieving sites, the tenant ID in the path must match the header
4. Tenants can only access their own sites

### Storage

Currently uses in-memory storage (`RwLock<HashMap<TenantId, Vec<Site>>>`). This means:
- Data is lost on server restart
- Suitable for MVP/demo purposes
- Can be easily replaced with a database in the future

### Error Handling

The application uses structured error handling:
- `AppError` enum for application-specific errors
- Automatic conversion to HTTP status codes
- Proper error responses via poem-openapi

## Development

### Project Status

- ✅ MVP endpoints implemented
- ✅ Tenant separation enforced
- ✅ OpenAPI specification generated
- ✅ Unit and integration tests
- ⏳ NetBox integration (placeholder - not yet implemented)

### Future Enhancements

- [ ] Replace in-memory storage with database (PostgreSQL/SQLite)
- [ ] Implement actual NetBox API integration
- [ ] Add authentication/authorization
- [ ] Add request validation
- [ ] Add rate limiting
- [ ] Add metrics and monitoring

## License

This is a demo project.

## Contributing

This is a demo project for learning purposes.

