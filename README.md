# NetGate - Production-Ready Middleware for NetBox Integration

**NetGate** is a high-performance, production-ready middleware solution that sits between frontend order portals and NetBox (infrastructure source of truth). Built with Rust and modern async patterns, it provides enterprise-grade features including tenant separation, business rules engine, resilience patterns, caching, and extensible plugin architecture.

## 🎯 Project Overview

NetGate transforms simple order requests into fully enriched NetBox resources through a sophisticated pipeline that includes validation, transformation, enrichment, and resilient API integration. The system is designed with clean architecture principles, making it maintainable, testable, and extensible.

### Key Highlights

- ✅ **265+ Unit Tests** - Comprehensive test coverage
- ✅ **Production-Ready** - Resilience patterns, caching, observability
- ✅ **Extensible Architecture** - Plugin pattern for easy extension
- ✅ **Enterprise Features** - Tenant separation, business rules, workflow management
- ✅ **Full NetBox Integration** - Complete CRUD operations with retry and circuit breaker
- ✅ **Observability** - Metrics, health checks, structured logging

## 🏗️ Architecture

### System Architecture

```
┌─────────────┐
│   Frontend  │
│ Order Portal│
└──────┬──────┘
       │ HTTP/REST
       │ X-Tenant-Id Header
       ▼
┌─────────────────────────────────────────────────┐
│              NetGate Middleware                 │
│  ┌──────────────────────────────────────────┐ │
│  │  API Layer (poem-openapi)                 │ │
│  │  - /health, /metrics, /orders, /tenants   │ │
│  └──────────────────────────────────────────┘ │
│  ┌──────────────────────────────────────────┐ │
│  │  Business Logic Layer                     │ │
│  │  - Validation → Transformation →          │ │
│  │    Enrichment → Workflow Management       │ │
│  └──────────────────────────────────────────┘ │
│  ┌──────────────────────────────────────────┐ │
│  │  Plugin System (Extensible)             │ │
│  │  - OrderProcessor trait                  │ │
│  │  - OrderTypeRegistry                    │ │
│  └──────────────────────────────────────────┘ │
│  ┌──────────────────────────────────────────┐ │
│  │  Resilience Layer                       │ │
│  │  - Retry with exponential backoff       │ │
│  │  - Circuit breaker pattern              │ │
│  │  - Graceful degradation                 │ │
│  └──────────────────────────────────────────┘ │
│  ┌──────────────────────────────────────────┐ │
│  │  Caching Layer                          │ │
│  │  - TTL-based cache                      │ │
│  │  - Hit/miss metrics                     │ │
│  │  - Invalidation strategies              │ │
│  └──────────────────────────────────────────┘ │
└──────┬─────────────────────────────────────────┘
       │ HTTP/REST
       │ Token Authentication
       ▼
┌─────────────┐
│   NetBox    │
│     API     │
└─────────────┘
```

### Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Language** | Rust (stable) | Performance, memory safety, concurrency |
| **Runtime** | Tokio | Async runtime for high-performance I/O |
| **Web Framework** | poem + poem-openapi | REST API with automatic OpenAPI spec generation |
| **HTTP Client** | reqwest | Async HTTP client for NetBox integration |
| **Serialization** | serde + serde_json | Efficient JSON handling |
| **Logging** | tracing + tracing-subscriber | Structured logging with JSON support |
| **Error Handling** | thiserror + anyhow | Comprehensive error management |
| **Testing** | wiremock | HTTP mocking for integration tests |

## 🚀 Features

### 1. Core Functionality

#### API Endpoints

- **GET /health** - Enhanced health check with NetBox connectivity and circuit breaker state
- **GET /metrics** - Comprehensive metrics endpoint (requests, retries, circuit breaker, cache)
- **POST /orders/site** - Create site orders with full pipeline processing
- **GET /orders/:order_id/status** - Get order workflow status
- **GET /tenants/:tenant_id/sites** - Get sites for a tenant (tenant-scoped)

#### Order Processing Pipeline

1. **Validation** - Business rules validation (name, description, address)
2. **Workflow Creation** - Order ID generation and state tracking
3. **Transformation** - Order → NetBox resource mapping
4. **Enrichment** - Add computed fields, tags, metadata
5. **NetBox Creation** - Resilient API call with retry and circuit breaker
6. **Post-Enrichment** - Enrich created resource with additional data
7. **Workflow Completion** - Update state and link NetBox resource ID

### 2. Advanced Tenant Separation

- **Tenant Identification** - Header-based (`X-Tenant-Id`)
- **Tenant Mapping** - Application tenant ID → NetBox tenant ID mapping
- **Access Control** - Resource access verification per tenant
- **Resource Visibility** - Tenant-scoped filtering of NetBox resources
- **Isolation Enforcement** - Strict separation at all layers

### 3. Business Rules Engine

- **Order Validation** - Configurable validation rules
- **Transformation Rules** - Order → NetBox resource mapping
- **Workflow Management** - State machine for order lifecycle
- **State Tracking** - Pending → Validated → Processing → Completed/Failed

### 4. Object Enrichment

- **Computed Fields** - Derived fields based on business logic
- **Multi-Source Merging** - Geographic, contact, business metadata
- **Tag Management** - Business logic-based tagging
- **Metadata Addition** - Custom fields and annotations

### 5. Virtual Object Mapping

- **Abstraction Layer** - Unified interface for virtual and physical resources
- **Virtual Resources** - Resources that don't exist in NetBox
- **Mapping Management** - Virtual ↔ Physical relationships (1:1, 1:N, N:1, N:N)
- **Tenant-Scoped Mappings** - Mappings isolated per tenant

### 6. Error Handling & Resilience

#### Retry Logic
- Exponential backoff with jitter
- Configurable max attempts and delays
- Retryable error detection

#### Circuit Breaker
- Three-state pattern (Closed, Open, HalfOpen)
- Configurable failure thresholds
- Automatic recovery

#### Graceful Degradation
- TTL-based cache for fallback
- Multiple degradation strategies
- Service availability detection

#### Metrics
- Request counts (total, successful, failed)
- Success/failure rates
- Average response times
- Retry statistics
- Circuit breaker rejections

### 7. Caching Layer

- **In-Memory Cache** - TTL-based caching for NetBox resources
- **Cache Metrics** - Hit/miss rates, eviction tracking
- **Invalidation Strategies** - Write-through, write-back, type-based
- **Size Limits** - Configurable max size with FIFO eviction
- **Automatic Expiration** - TTL-based cleanup

### 8. Observability

- **Enhanced Health Check** - Service status, NetBox connectivity, circuit breaker state
- **Metrics Endpoint** - Comprehensive performance metrics
- **Structured Logging** - JSON-formatted logs with request IDs
- **Request Tracing** - Correlation IDs for distributed tracing (infrastructure ready)

### 9. Extensibility/Plugin Pattern

- **OrderProcessor Trait** - Extensible interface for order processing
- **OrderTypeRegistry** - Centralized processor management
- **Configuration-Driven** - Order type mappings from configuration
- **Easy Extension** - Add new order types without modifying core code
- **Type-Safe Enums** - Compile-time safety for order types

## 📁 Project Structure

```
netgate/
├── src/
│   ├── main.rs                    # Application entry point
│   ├── lib.rs                     # Library exports
│   ├── config.rs                  # Configuration management
│   ├── logging.rs                 # Logging initialization
│   ├── error.rs                   # Error types
│   │
│   ├── api/                       # API Layer
│   │   ├── health.rs              # Enhanced health check
│   │   ├── metrics.rs             # Metrics endpoint
│   │   ├── orders.rs              # Order endpoints
│   │   └── tenants.rs             # Tenant endpoints
│   │
│   ├── business/                  # Business Logic Layer
│   │   ├── validation.rs          # Order validation rules
│   │   ├── transformation.rs      # Order → NetBox transformation
│   │   ├── enrichment.rs          # Object enrichment
│   │   ├── workflow.rs            # Order workflow/state management
│   │   ├── order_service.rs       # Order orchestration service
│   │   ├── extensible_order_service.rs  # Plugin-based service
│   │   ├── plugin.rs              # Plugin infrastructure
│   │   └── processors.rs          # Order processor implementations
│   │
│   ├── domain/                    # Domain Models
│   │   ├── order.rs               # Order domain models
│   │   └── tenant.rs              # Tenant domain models
│   │
│   ├── netbox/                    # NetBox Integration
│   │   ├── client.rs              # NetBox HTTP client
│   │   ├── resilient_client.rs   # Resilient wrapper (retry, circuit breaker)
│   │   ├── cached_client.rs       # Cached wrapper
│   │   ├── tenant_client.rs       # Tenant-aware client
│   │   ├── models.rs              # NetBox data models
│   │   └── error.rs               # NetBox-specific errors
│   │
│   ├── resilience/                # Resilience Patterns
│   │   ├── retry.rs               # Retry logic with backoff
│   │   ├── circuit_breaker.rs     # Circuit breaker pattern
│   │   ├── metrics.rs             # API metrics tracking
│   │   └── degradation.rs         # Graceful degradation
│   │
│   ├── cache/                     # Caching Layer
│   │   ├── store.rs               # Cache implementation
│   │   ├── metrics.rs             # Cache metrics
│   │   └── strategy.rs            # Invalidation strategies
│   │
│   ├── security/                  # Security Layer
│   │   └── tenant.rs              # Tenant separation and access control
│   │
│   ├── observability/             # Observability
│   │   ├── middleware.rs         # Request tracing middleware
│   │   └── tracing.rs             # Structured logging setup
│   │
│   └── virtual/                   # Virtual Object Mapping
│       ├── mapping.rs             # Virtual/physical mapping
│       ├── resource.rs            # Resource abstraction
│       └── service.rs             # Virtual resource service
│
├── tests/
│   └── integration_test.rs        # Integration tests
│
├── demo/
│   └── frontend_emulator.sh       # Frontend simulation script
│
├── Cargo.toml                     # Dependencies
├── Makefile                       # Build automation
└── README.md                      # This file
```

## 🛠️ Getting Started

### Prerequisites

- **Rust** stable (1.70+)
- **Cargo** (comes with Rust)
- **NetBox** instance (optional, for full integration testing)
- **bc** (for demo script statistics)

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd netgate

# Build the project
cargo build --release

# Run tests
make test
```

### Configuration

Set environment variables:

```bash
export PORT=8080
export NETBOX_URL=http://localhost:8000
export NETBOX_TOKEN=your-netbox-token
```

Or use a `.env` file (not included, create as needed).

### Running the Server

```bash
# Development mode
cargo run

# Or using Makefile
make run

# Production mode
cargo build --release
./target/release/netgate
```

The server will start on `http://localhost:8080` (or configured port).

### Running the Frontend Emulator

The demo script simulates a frontend application making API calls:

**Step 1: Start the NetGate server**
```bash
# Terminal 1: Start server
cargo run
```

**Step 2: Run the emulator**
```bash
# Terminal 2: Run emulator (in a new terminal)
# Make script executable (first time only)
chmod +x demo/frontend_emulator.sh

# Run with default settings (3 tenants, 5 orders each)
./demo/frontend_emulator.sh
```

**Customization options:**
```bash
# Customize simulation parameters
TENANT_COUNT=5 ORDERS_PER_TENANT=10 ./demo/frontend_emulator.sh

# Use different NetGate URL
NETGATE_URL=http://localhost:9090 ./demo/frontend_emulator.sh

# Adjust delay between requests (in seconds)
DELAY_BETWEEN_REQUESTS=1.0 ./demo/frontend_emulator.sh

# Combine all options
TENANT_COUNT=3 ORDERS_PER_TENANT=5 DELAY_BETWEEN_REQUESTS=0.5 NETGATE_URL=http://localhost:8080 ./demo/frontend_emulator.sh
```

**What the script does:**
- ✅ Checks system health
- ✅ Creates orders for multiple tenants
- ✅ Checks order statuses
- ✅ Fetches tenant sites
- ✅ Retrieves system metrics
- ✅ Displays comprehensive statistics (success rate, response times, request counts)

## 📊 API Documentation

### Interactive Documentation

Once the server is running:

- **Swagger UI**: http://localhost:8080/docs
- **OpenAPI Spec**: http://localhost:8080/spec

### Example API Calls

#### Health Check

```bash
curl http://localhost:8080/health
```

Response includes:
- Service status (healthy/degraded)
- NetBox connectivity status
- Circuit breaker state
- Response times

#### Metrics

```bash
curl http://localhost:8080/metrics
```

Returns:
- Request counts and rates
- Response times
- Retry statistics
- Circuit breaker metrics
- Cache metrics (if enabled)

#### Create Site Order

```bash
curl -X POST http://localhost:8080/orders/site \
  -H "Content-Type: application/json" \
  -H "X-Tenant-Id: tenant1" \
  -d '{
    "name": "Production Site",
    "description": "Main production facility",
    "address": "123 Data Center Blvd"
  }'
```

Response:
```json
{
  "order_id": "uuid-here",
  "tenant_id": "tenant1",
  "netbox_site_id": 123,
  "state": "Completed",
  "site_name": "Production Site"
}
```

#### Get Order Status

```bash
curl http://localhost:8080/orders/{order_id}/status \
  -H "X-Tenant-Id: tenant1"
```

#### Get Tenant Sites

```bash
curl http://localhost:8080/tenants/tenant1/sites \
  -H "X-Tenant-Id: tenant1"
```

## 🧪 Testing

### Unit Tests

```bash
# Run all unit tests
cargo test --lib

# Run with output
cargo test --lib -- --nocapture

# Run specific test module
cargo test --lib business::validation
```

**Test Coverage**: 265+ unit tests covering:
- Domain logic
- Business rules (validation, transformation, enrichment)
- NetBox integration (with mocks)
- Resilience patterns (retry, circuit breaker)
- Caching layer
- Plugin system
- Security and tenant separation
- Virtual object mapping

### Integration Tests

```bash
# Start server in one terminal
cargo run

# Run integration tests in another terminal
cargo test --test integration_test -- --ignored
```

Integration tests cover:
- End-to-end order processing
- Tenant isolation
- Error handling
- Order status tracking
- API response validation

### Test Statistics

- **Total Tests**: 265+
- **Unit Tests**: 250+
- **Integration Tests**: 15+
- **Test Modules**: 20+
- **Coverage**: Comprehensive across all modules

## 🏛️ Architecture Deep Dive

### Clean Architecture Principles

NetGate follows clean architecture principles:

1. **Separation of Concerns** - Clear boundaries between layers
2. **Dependency Inversion** - High-level modules don't depend on low-level modules
3. **Single Responsibility** - Each module has one clear purpose
4. **Open/Closed Principle** - Open for extension, closed for modification (plugin pattern)

### Design Patterns

- **Strategy Pattern** - OrderProcessor trait for different order types
- **Registry Pattern** - OrderTypeRegistry for processor management
- **Circuit Breaker Pattern** - Resilience against failing services
- **Retry Pattern** - Exponential backoff for transient failures
- **Cache-Aside Pattern** - Caching with explicit invalidation
- **Builder Pattern** - ExtensibleOrderServiceBuilder for service construction

### Concurrency & Performance

- **Async/Await** - Non-blocking I/O throughout
- **Arc + RwLock** - Thread-safe shared state
- **Tokio Runtime** - High-performance async runtime
- **Connection Pooling** - Reqwest client connection reuse
- **Caching** - Reduces NetBox API calls

## 🔒 Security Features

- **Tenant Isolation** - Strict separation enforced at all layers
- **Access Control** - Resource access verification
- **Header-Based Authentication** - `X-Tenant-Id` header validation
- **Path Validation** - Tenant ID in path must match header
- **Error Message Sanitization** - No sensitive data leakage

## 📈 Performance Characteristics

- **Low Latency** - Async I/O with connection pooling
- **High Throughput** - Non-blocking request handling
- **Resource Efficient** - Rust's zero-cost abstractions
- **Caching** - Reduces external API calls
- **Circuit Breaker** - Prevents cascading failures

## 🔧 Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | Server port |
| `NETBOX_URL` | `http://localhost:8000` | NetBox API URL |
| `NETBOX_TOKEN` | (empty) | NetBox API token (optional - server can run without it for demo) |
| `RUST_LOG` | `info` | Logging level |

**Note**: The server can start without `NETBOX_TOKEN` for demonstration purposes. Without a token:
- Health and metrics endpoints will work (without NetBox connectivity info)
- Order creation endpoints will fail when trying to create resources in NetBox
- Other endpoints will function normally

To enable full NetBox integration, set `NETBOX_TOKEN`:
```bash
export NETBOX_TOKEN=your-netbox-token
cargo run
```

### Cache Configuration

Cache settings can be configured programmatically:

```rust
use crate::cache::CacheConfig;
use std::time::Duration;

let config = CacheConfig::new(Duration::from_secs(300))
    .with_max_size(1000)
    .with_invalidation_strategy(InvalidationStrategy::WriteBack)
    .with_metrics(true);
```

### Resilience Configuration

```rust
use crate::resilience::{RetryConfig, CircuitBreakerConfig};

let retry_config = RetryConfig {
    max_attempts: 3,
    initial_delay_ms: 100,
    max_delay_ms: 5000,
    backoff_multiplier: 2.0,
    use_jitter: true,
};

let cb_config = CircuitBreakerConfig {
    failure_threshold: 5,
    success_threshold: 2,
    timeout_duration: Duration::from_secs(60),
    window_duration: Duration::from_secs(60),
};
```

## 🚀 Production Readiness

### Features

✅ **Comprehensive Error Handling** - Structured errors with proper HTTP status codes  
✅ **Resilience Patterns** - Retry, circuit breaker, graceful degradation  
✅ **Observability** - Metrics, health checks, structured logging  
✅ **Caching** - Performance optimization with TTL and invalidation  
✅ **Testing** - 265+ tests with high coverage  
✅ **Documentation** - OpenAPI spec, code comments, README  
✅ **Extensibility** - Plugin pattern for easy extension  
✅ **Security** - Tenant isolation and access control  

### Deployment Considerations

- **Health Checks** - Use `/health` endpoint for load balancer
- **Metrics** - Monitor `/metrics` endpoint
- **Logging** - Structured JSON logs for log aggregation
- **Configuration** - Environment-based configuration
- **Graceful Shutdown** - Tokio signal handling (can be added)

## 📝 Code Quality

- **Rust Best Practices** - Idiomatic Rust code
- **Error Handling** - Comprehensive error types
- **Type Safety** - Strong typing throughout
- **Documentation** - Code comments and docstrings
- **Testing** - High test coverage
- **Linting** - Cargo clippy clean (warnings only for unused code in tests)

## 🔮 Future Enhancements

Potential additions (not yet implemented):

- [ ] Database persistence (PostgreSQL/SQLite)
- [ ] Redis caching backend
- [ ] Async job processing for long operations
- [ ] Webhook support for NetBox events
- [ ] Batch operations
- [ ] Rate limiting
- [ ] Authentication/authorization (JWT, OAuth)
- [ ] GraphQL API
- [ ] gRPC support
- [ ] Kubernetes deployment manifests
- [ ] Docker containerization
- [ ] CI/CD pipeline configuration

## 📚 Key Implementation Highlights

### 1. End-to-End Integration

The order processing flow demonstrates complete system integration:
- Order validation → Transformation → Enrichment → NetBox creation
- Workflow state management throughout
- Error handling at every step
- Comprehensive test coverage

### 2. Resilience Patterns

Production-grade resilience:
- Automatic retry with exponential backoff
- Circuit breaker to prevent cascading failures
- Graceful degradation with cached fallbacks
- Comprehensive metrics for monitoring

### 3. Extensibility

Plugin architecture allows easy extension:
- Add new order types without modifying core code
- Configuration-driven processor registration
- Type-safe enum-based design
- Clean separation of concerns

### 4. Observability

Production-ready monitoring:
- Detailed metrics endpoint
- Enhanced health checks
- Structured logging infrastructure
- Request tracing support

## 🤝 Contributing

This is a demonstration project showcasing:
- Rust async programming
- Clean architecture principles
- Production-ready patterns
- Comprehensive testing
- Extensible design

## 📄 License

This is a demo project for portfolio/learning purposes.

## 🙏 Acknowledgments

Built with:
- [Tokio](https://tokio.rs/) - Async runtime
- [Poem](https://github.com/poem-web/poem) - Web framework
- [Reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [Serde](https://serde.rs/) - Serialization
- [Tracing](https://github.com/tokio-rs/tracing) - Logging

---

**NetGate** - Bridging the gap between order portals and NetBox with enterprise-grade middleware.
