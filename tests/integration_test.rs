// Integration tests using reqwest to make actual HTTP calls
// Note: These tests require the server to be running on port 8080
// For a more complete setup, you could use a test server that starts/stops automatically

use serde_json::json;

const BASE_URL: &str = "http://localhost:8080";

#[tokio::test]
#[ignore] // Ignore by default - run with: cargo test -- --ignored
async fn test_health_endpoint() {
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/health", BASE_URL)).send().await.unwrap();
    
    // Health endpoint should return 200 or 503 depending on NetBox connectivity
    assert!(resp.status() == 200 || resp.status() == 503);
    
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["service"], "NetGate");
    assert_eq!(body["version"], "1.0.0");
    assert!(body["status"].is_string());
    assert!(body["timestamp"].is_string());
    
    // NetBox health should be present (may be connected or disconnected)
    if let Some(netbox) = body.get("netbox") {
        assert!(netbox["connected"].is_boolean());
    }
    
    // Circuit breaker health should be present
    if let Some(cb) = body.get("circuit_breaker") {
        assert!(cb["state"].is_string());
        assert!(cb["failure_count"].is_number());
    }
}

#[tokio::test]
#[ignore]
async fn test_metrics_endpoint() {
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/metrics", BASE_URL)).send().await.unwrap();
    
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    
    assert!(body["timestamp"].is_string());
    
    // NetBox metrics should be present
    if let Some(netbox) = body.get("netbox") {
        assert!(netbox["total_requests"].is_number());
        assert!(netbox["successful_requests"].is_number());
        assert!(netbox["failed_requests"].is_number());
        assert!(netbox["success_rate"].is_number());
        assert!(netbox["failure_rate"].is_number());
        assert!(netbox["average_response_time_ms"].is_number());
        assert!(netbox["total_retries"].is_number());
        assert!(netbox["circuit_breaker_rejections"].is_number());
        assert!(netbox["circuit_breaker_state"].is_string());
    }
}

#[tokio::test]
#[ignore]
async fn test_create_site_success() {
    let client = reqwest::Client::new();
    let order = json!({
        "name": "Test Site",
        "description": "Test Description",
        "address": "123 Test St"
    });

    let resp = client
        .post(format!("{}/orders/site", BASE_URL))
        .header("X-Tenant-Id", "tenant1")
        .json(&order)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["site_name"], "Test Site");
    assert_eq!(body["tenant_id"], "tenant1");
    assert!(body["order_id"].is_string());
}

#[tokio::test]
#[ignore]
async fn test_create_site_missing_header() {
    let client = reqwest::Client::new();
    let order = json!({
        "name": "Test Site"
    });

    let resp = client
        .post(format!("{}/orders/site", BASE_URL))
        .json(&order)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
#[ignore]
async fn test_get_sites_success() {
    let client = reqwest::Client::new();
    let tenant_id = "tenant2";

    // Note: The new implementation creates sites in NetBox, not in-memory store
    // These tests may need to be updated based on actual behavior
    // For now, we'll test the order creation endpoint

    // Get sites
    let resp = client
        .get(format!("{}/tenants/{}/sites", BASE_URL, tenant_id))
        .header("X-Tenant-Id", tenant_id)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.is_array());
    let sites = body.as_array().unwrap();
    assert_eq!(sites.len(), 2);
    assert_eq!(sites[0]["name"], "Site 1");
    assert_eq!(sites[1]["name"], "Site 2");
}

#[tokio::test]
#[ignore]
async fn test_get_sites_missing_header() {
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/tenants/tenant1/sites", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
#[ignore]
async fn test_get_sites_header_mismatch() {
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/tenants/tenant1/sites", BASE_URL))
        .header("X-Tenant-Id", "tenant2")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
#[ignore]
async fn test_tenant_isolation() {
    let client = reqwest::Client::new();

    // Create site for tenant1
    let order1 = json!({"name": "Tenant1 Site"});
    client
        .post(format!("{}/orders/site", BASE_URL))
        .header("X-Tenant-Id", "tenant1")
        .json(&order1)
        .send()
        .await
        .unwrap();

    // Create site for tenant2
    let order2 = json!({"name": "Tenant2 Site"});
    client
        .post(format!("{}/orders/site", BASE_URL))
        .header("X-Tenant-Id", "tenant2")
        .json(&order2)
        .send()
        .await
        .unwrap();

    // Get sites for tenant1 - should only see tenant1's site
    let resp1 = client
        .get(format!("{}/tenants/tenant1/sites", BASE_URL))
        .header("X-Tenant-Id", "tenant1")
        .send()
        .await
        .unwrap();

    assert_eq!(resp1.status(), 200);
    let body1: serde_json::Value = resp1.json().await.unwrap();
    let sites1 = body1.as_array().unwrap();
    assert_eq!(sites1.len(), 1);
    assert_eq!(sites1[0]["name"], "Tenant1 Site");
    assert_eq!(sites1[0]["tenant_id"], "tenant1");

    // Get sites for tenant2 - should only see tenant2's site
    let resp2 = client
        .get(format!("{}/tenants/tenant2/sites", BASE_URL))
        .header("X-Tenant-Id", "tenant2")
        .send()
        .await
        .unwrap();

    assert_eq!(resp2.status(), 200);
    let body2: serde_json::Value = resp2.json().await.unwrap();
    let sites2 = body2.as_array().unwrap();
    assert_eq!(sites2.len(), 1);
    assert_eq!(sites2[0]["name"], "Tenant2 Site");
    assert_eq!(sites2[0]["tenant_id"], "tenant2");
}

#[tokio::test]
#[ignore]
async fn test_get_sites_empty() {
    let client = reqwest::Client::new();
    let tenant_id = "empty_tenant";

    let resp = client
        .get(format!("{}/tenants/{}/sites", BASE_URL, tenant_id))
        .header("X-Tenant-Id", tenant_id)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.is_array());
    let sites = body.as_array().unwrap();
    assert_eq!(sites.len(), 0);
}

#[tokio::test]
#[ignore]
async fn test_create_site_order_end_to_end() {
    let client = reqwest::Client::new();
    let order = json!({
        "name": "End-to-End Test Site",
        "description": "Testing full pipeline",
        "address": "456 Integration St"
    });

    let resp = client
        .post(format!("{}/orders/site", BASE_URL))
        .header("X-Tenant-Id", "e2e-tenant")
        .json(&order)
        .send()
        .await
        .unwrap();

    // Should either succeed (if NetBox is available) or fail gracefully
    assert!(resp.status() == 201 || resp.status() == 500);
    
    if resp.status() == 201 {
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["order_id"].is_string());
        assert!(body["tenant_id"] == "e2e-tenant");
        assert!(body["site_name"] == "End-to-End Test Site");
        
        // Test order status endpoint
        let order_id = body["order_id"].as_str().unwrap();
        let status_resp = client
            .get(format!("{}/orders/{}/status", BASE_URL, order_id))
            .header("X-Tenant-Id", "e2e-tenant")
            .send()
            .await
            .unwrap();
        
        assert_eq!(status_resp.status(), 200);
        let status_body: serde_json::Value = status_resp.json().await.unwrap();
        assert_eq!(status_body["order_id"], order_id);
        assert!(status_body["state"].is_string());
    }
}

#[tokio::test]
#[ignore]
async fn test_order_status_not_found() {
    let client = reqwest::Client::new();
    
    let resp = client
        .get(format!("{}/orders/nonexistent-order-id/status", BASE_URL))
        .header("X-Tenant-Id", "tenant1")
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
#[ignore]
async fn test_order_status_unauthorized() {
    let client = reqwest::Client::new();
    
    // First create an order for tenant1
    let order = json!({"name": "Test Site"});
    let create_resp = client
        .post(format!("{}/orders/site", BASE_URL))
        .header("X-Tenant-Id", "tenant1")
        .json(&order)
        .send()
        .await
        .unwrap();
    
    if create_resp.status() == 201 {
        let body: serde_json::Value = create_resp.json().await.unwrap();
        let order_id = body["order_id"].as_str().unwrap();
        
        // Try to access with tenant2
        let status_resp = client
            .get(format!("{}/orders/{}/status", BASE_URL, order_id))
            .header("X-Tenant-Id", "tenant2")
            .send()
            .await
            .unwrap();
        
        assert_eq!(status_resp.status(), 401);
    }
}

#[tokio::test]
#[ignore]
async fn test_create_site_validation_error() {
    let client = reqwest::Client::new();
    
    // Create order with invalid data (empty name)
    let invalid_order = json!({
        "name": "",
        "description": "Invalid order"
    });
    
    let resp = client
        .post(format!("{}/orders/site", BASE_URL))
        .header("X-Tenant-Id", "tenant1")
        .json(&invalid_order)
        .send()
        .await
        .unwrap();
    
    // Should return validation error
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "Validation failed");
}

