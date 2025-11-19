# Frontend Emulator Demo

This script simulates a frontend application making API calls to NetGate, demonstrating the full system behavior and generating comprehensive statistics.

## Usage

### Basic Usage

```bash
# Make sure NetGate server is running
cargo run

# In another terminal, run the emulator
./demo/frontend_emulator.sh
```

### Customization

```bash
# Customize number of tenants and orders
TENANT_COUNT=5 ORDERS_PER_TENANT=10 ./demo/frontend_emulator.sh

# Adjust delay between requests (in seconds)
DELAY_BETWEEN_REQUESTS=1.0 ./demo/frontend_emulator.sh

# Use different NetGate URL
NETGATE_URL=http://localhost:9090 ./demo/frontend_emulator.sh

# Combine all options
TENANT_COUNT=3 ORDERS_PER_TENANT=5 DELAY_BETWEEN_REQUESTS=0.5 NETGATE_URL=http://localhost:8080 ./demo/frontend_emulator.sh
```

## What It Does

1. **Health Check** - Verifies NetGate is running and checks system health
2. **Order Creation** - Creates site orders for multiple tenants
3. **Status Checking** - Checks the status of created orders
4. **Site Retrieval** - Fetches sites for each tenant
5. **Metrics Collection** - Retrieves system metrics
6. **Statistics** - Displays comprehensive statistics

## Output

The script provides:
- ✅/✗ indicators for each request (success/failure)
- HTTP status codes
- Response times
- Final statistics:
  - Total requests
  - Success/failure counts
  - Success rate percentage
  - Average response time

## Requirements

- `curl` - For making HTTP requests
- `bc` - For mathematical calculations (statistics)
- NetGate server running on configured port

## Example Output

```
========================================
NetGate Frontend Emulator
========================================
Base URL: http://localhost:8080
Tenants: 3
Orders per tenant: 5

========================================
Checking NetGate Health
========================================
✓ Health Check (HTTP 200, 0.023s)
{"status":"healthy","service":"NetGate",...}

========================================
Creating Site Orders
========================================
Processing tenant: tenant-1
✓ Create Site Order: Site-1-1 for tenant tenant-1 (HTTP 201, 0.145s)
✓ Create Site Order: Site-1-2 for tenant tenant-1 (HTTP 201, 0.132s)
...

========================================
Simulation Statistics
========================================
Total Requests: 25
Successful: 25
Failed: 0
Success Rate: 100.00%
Average Response Time: 0.089s
```

