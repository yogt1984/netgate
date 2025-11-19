#!/bin/bash

# NetGate Frontend Emulator
# Simulates a frontend application making API calls to NetGate
# Generates statistics and demonstrates the full system behavior

set -e

# Configuration
BASE_URL="${NETGATE_URL:-http://localhost:8080}"
TENANT_COUNT="${TENANT_COUNT:-3}"
ORDERS_PER_TENANT="${ORDERS_PER_TENANT:-5}"
DELAY_BETWEEN_REQUESTS="${DELAY_BETWEEN_REQUESTS:-0.5}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Statistics
TOTAL_REQUESTS=0
SUCCESSFUL_REQUESTS=0
FAILED_REQUESTS=0
TOTAL_RESPONSE_TIME=0
ORDER_IDS=()

# Function to print header
print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Function to make API call and measure time
api_call() {
    local method=$1
    local endpoint=$2
    local tenant_id=$3
    local data=$4
    local description=$5
    
    local start_time=$(date +%s.%N)
    local response
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -w "\n%{http_code}" \
            -H "X-Tenant-Id: $tenant_id" \
            "$BASE_URL$endpoint" 2>/dev/null || echo -e "\n000")
    else
        response=$(curl -s -w "\n%{http_code}" \
            -X "$method" \
            -H "Content-Type: application/json" \
            -H "X-Tenant-Id: $tenant_id" \
            -d "$data" \
            "$BASE_URL$endpoint" 2>/dev/null || echo -e "\n000")
    fi
    
    local end_time=$(date +%s.%N)
    local response_time=$(echo "$end_time - $start_time" | bc)
    local http_code=$(echo "$response" | tail -n1)
    local body=$(echo "$response" | sed '$d')
    
    TOTAL_REQUESTS=$((TOTAL_REQUESTS + 1))
    TOTAL_RESPONSE_TIME=$(echo "$TOTAL_RESPONSE_TIME + $response_time" | bc)
    
    if [ "$http_code" -ge 200 ] && [ "$http_code" -lt 300 ]; then
        SUCCESSFUL_REQUESTS=$((SUCCESSFUL_REQUESTS + 1))
        echo -e "${GREEN}✓${NC} $description (HTTP $http_code, ${response_time}s)"
        echo "$body"
        return 0
    else
        FAILED_REQUESTS=$((FAILED_REQUESTS + 1))
        echo -e "${RED}✗${NC} $description (HTTP $http_code, ${response_time}s)"
        echo "$body" | head -c 200
        echo ""
        return 1
    fi
}

# Function to check health
check_health() {
    print_header "Checking NetGate Health"
    api_call "GET" "/health" "system" "" "Health Check"
    echo ""
}

# Function to get metrics
get_metrics() {
    print_header "Fetching System Metrics"
    api_call "GET" "/metrics" "system" "" "Metrics"
    echo ""
}

# Function to create site order
create_site_order() {
    local tenant_id=$1
    local site_name=$2
    local description=$3
    local address=$4
    
    local order_data=$(cat <<EOF
{
    "name": "$site_name",
    "description": "$description",
    "address": "$address"
}
EOF
)
    
    local result=$(api_call "POST" "/orders/site" "$tenant_id" "$order_data" \
        "Create Site Order: $site_name for tenant $tenant_id")
    
    if [ $? -eq 0 ]; then
        # Extract order_id from response (assuming JSON response)
        local order_id=$(echo "$result" | grep -o '"order_id":"[^"]*"' | cut -d'"' -f4 || echo "")
        if [ -n "$order_id" ]; then
            ORDER_IDS+=("$tenant_id:$order_id")
        fi
    fi
    
    sleep "$DELAY_BETWEEN_REQUESTS"
}

# Function to check order status
check_order_status() {
    local tenant_id=$1
    local order_id=$2
    
    api_call "GET" "/orders/$order_id/status" "$tenant_id" "" \
        "Check Order Status: $order_id" > /dev/null
    
    sleep "$DELAY_BETWEEN_REQUESTS"
}

# Function to get tenant sites
get_tenant_sites() {
    local tenant_id=$1
    
    api_call "GET" "/tenants/$tenant_id/sites" "$tenant_id" "" \
        "Get Sites for Tenant: $tenant_id" > /dev/null
    
    sleep "$DELAY_BETWEEN_REQUESTS"
}

# Main simulation
main() {
    print_header "NetGate Frontend Emulator"
    echo -e "Base URL: ${YELLOW}$BASE_URL${NC}"
    echo -e "Tenants: ${YELLOW}$TENANT_COUNT${NC}"
    echo -e "Orders per tenant: ${YELLOW}$ORDERS_PER_TENANT${NC}"
    echo ""
    
    # Check if server is running
    if ! curl -s "$BASE_URL/health" > /dev/null 2>&1; then
        echo -e "${RED}Error: Cannot connect to NetGate at $BASE_URL${NC}"
        echo "Please ensure the server is running: cargo run"
        exit 1
    fi
    
    # Initial health check
    check_health
    
    # Create orders for each tenant
    print_header "Creating Site Orders"
    for tenant_num in $(seq 1 $TENANT_COUNT); do
        tenant_id="tenant-$tenant_num"
        echo -e "${BLUE}Processing tenant: $tenant_id${NC}"
        
        for order_num in $(seq 1 $ORDERS_PER_TENANT); do
            site_name="Site-$tenant_num-$order_num"
            description="Site created by frontend emulator for tenant $tenant_id"
            address="$((100 + order_num)) Main St, City $tenant_num"
            
            create_site_order "$tenant_id" "$site_name" "$description" "$address"
        done
        echo ""
    done
    
    # Check order statuses
    if [ ${#ORDER_IDS[@]} -gt 0 ]; then
        print_header "Checking Order Statuses"
        for order_ref in "${ORDER_IDS[@]}"; do
            tenant_id=$(echo "$order_ref" | cut -d':' -f1)
            order_id=$(echo "$order_ref" | cut -d':' -f2)
            check_order_status "$tenant_id" "$order_id"
        done
        echo ""
    fi
    
    # Get tenant sites
    print_header "Fetching Tenant Sites"
    for tenant_num in $(seq 1 $TENANT_COUNT); do
        tenant_id="tenant-$tenant_num"
        get_tenant_sites "$tenant_id"
    done
    echo ""
    
    # Final metrics
    get_metrics
    
    # Print statistics
    print_header "Simulation Statistics"
    echo -e "Total Requests: ${YELLOW}$TOTAL_REQUESTS${NC}"
    echo -e "Successful: ${GREEN}$SUCCESSFUL_REQUESTS${NC}"
    echo -e "Failed: ${RED}$FAILED_REQUESTS${NC}"
    
    if [ $TOTAL_REQUESTS -gt 0 ]; then
        local success_rate=$(echo "scale=2; $SUCCESSFUL_REQUESTS * 100 / $TOTAL_REQUESTS" | bc)
        echo -e "Success Rate: ${GREEN}${success_rate}%${NC}"
        
        local avg_response_time=$(echo "scale=3; $TOTAL_RESPONSE_TIME / $TOTAL_REQUESTS" | bc)
        echo -e "Average Response Time: ${YELLOW}${avg_response_time}s${NC}"
    fi
    
    echo ""
    echo -e "${GREEN}Simulation completed!${NC}"
}

# Run main function
main

