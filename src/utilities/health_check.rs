use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow};
use colored::Colorize;
use reqwest::Client;
use serde_json::json;
use tokio::sync::RwLock;
use tokio::time::{sleep, interval};

use crate::library::{
    config::TransactionLandingMode,
    logger::Logger,
};

/// Health status of a service
#[derive(Clone, Debug, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// Health check result for a service
#[derive(Clone, Debug)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub response_time: Duration,
    pub last_checked: Instant,
    pub error_message: Option<String>,
}

/// Health check manager for transaction landing services
pub struct HealthCheckManager {
    services: Arc<RwLock<HashMap<String, HealthCheckResult>>>,
    client: Client,
    logger: Logger,
}

impl HealthCheckManager {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            logger: Logger::new("[HEALTH-CHECK] => ".yellow().bold().to_string()),
        }
    }

    /// Start the health check service
    pub async fn start(&self) -> Result<()> {
        self.logger.log("Starting health check service...".green().to_string());
        
        // Initialize all services as unknown
        let mut services = self.services.write().await;
        services.insert("zeroslot".to_string(), HealthCheckResult {
            status: HealthStatus::Unknown,
            response_time: Duration::from_secs(0),
            last_checked: Instant::now(),
            error_message: None,
        });
        services.insert("nozomi".to_string(), HealthCheckResult {
            status: HealthStatus::Unknown,
            response_time: Duration::from_secs(0),
            last_checked: Instant::now(),
            error_message: None,
        });
        drop(services);

        // Start periodic health checks
        self.start_periodic_checks().await;
        
        Ok(())
    }

    /// Start periodic health checks for all services
    async fn start_periodic_checks(&self) {
        let services = self.services.clone();
        let client = self.client.clone();
        let logger = self.logger.clone();

        // Health check interval
        let mut health_interval = interval(Duration::from_secs(30));
        
        // ZeroSlot keepalive interval (every 60 seconds to stay within 65-second timeout)
        let mut zeroslot_keepalive_interval = interval(Duration::from_secs(60));

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = health_interval.tick() => {
                        Self::check_all_services(&services, &client, &logger).await;
                    }
                    _ = zeroslot_keepalive_interval.tick() => {
                        Self::zeroslot_keepalive(&client, &logger).await;
                    }
                }
            }
        });
    }

    /// Check health of all services
    async fn check_all_services(
        services: &Arc<RwLock<HashMap<String, HealthCheckResult>>>,
        client: &Client,
        logger: &Logger,
    ) {
        let mut tasks = Vec::new();
        
        // Check ZeroSlot
        tasks.push(tokio::spawn(Self::check_zeroslot_health(client.clone(), logger.clone())));
        
        // Check Nozomi
        tasks.push(tokio::spawn(Self::check_nozomi_health(client.clone(), logger.clone())));

        // Wait for all health checks to complete
        let results = futures::future::join_all(tasks).await;
        
        // Update service statuses
        let mut services_map = services.write().await;
        let service_names = ["zeroslot", "nozomi"];
        
        for (i, result) in results.iter().enumerate() {
            if let Ok(health_result) = result {
                if let Some(service_name) = service_names.get(i) {
                    services_map.insert(service_name.to_string(), health_result.clone());
                }
            }
        }
    }

    /// Check ZeroSlot health
    async fn check_zeroslot_health(client: Client, logger: Logger) -> HealthCheckResult {
        let start_time = Instant::now();
        
        // Get ZeroSlot URL from environment
        let zeroslot_url = std::env::var("ZERO_SLOT_URL")
            .unwrap_or_else(|_| "https://api.zeroslot.io".to_string());
        
        let health_check_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getHealth"
        });
        
        match client
            .post(&format!("{}/rpc", zeroslot_url))
            .json(&health_check_body)
            .send()
            .await
        {
            Ok(response) => {
                let response_time = start_time.elapsed();
                if response.status().is_success() {
                    HealthCheckResult {
                        status: HealthStatus::Healthy,
                        response_time,
                        last_checked: Instant::now(),
                        error_message: None,
                    }
                } else {
                    HealthCheckResult {
                        status: HealthStatus::Unhealthy,
                        response_time,
                        last_checked: Instant::now(),
                        error_message: Some(format!("HTTP {}", response.status())),
                    }
                }
            }
            Err(e) => {
                logger.log(format!("ZeroSlot health check failed: {}", e).red().to_string());
                HealthCheckResult {
                    status: HealthStatus::Unhealthy,
                    response_time: start_time.elapsed(),
                    last_checked: Instant::now(),
                    error_message: Some(e.to_string()),
                }
            }
        }
    }

    /// ZeroSlot keepalive - send periodic requests to maintain connection
    async fn zeroslot_keepalive(client: &Client, logger: &Logger) {
        let zeroslot_url = std::env::var("ZERO_SLOT_URL")
            .unwrap_or_else(|_| "https://api.zeroslot.io".to_string());
        
        let keepalive_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getVersion"
        });
        
        if let Err(e) = client
            .post(&format!("{}/rpc", zeroslot_url))
            .json(&keepalive_body)
            .send()
            .await
        {
            logger.log(format!("ZeroSlot keepalive failed: {}", e).yellow().to_string());
        } else {
            logger.log("ZeroSlot keepalive sent".green().to_string());
        }
    }

    /// Check Nozomi health
    async fn check_nozomi_health(client: Client, logger: Logger) -> HealthCheckResult {
        let start_time = Instant::now();
        
        // Get Nozomi URL from environment
        let nozomi_url = std::env::var("NOZOMI_URL")
            .unwrap_or_else(|_| "https://nozomi.rpc.endpoint".to_string());
        
        let health_check_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getHealth"
        });
        
        match client
            .post(&nozomi_url)
            .json(&health_check_body)
            .send()
            .await
        {
            Ok(response) => {
                let response_time = start_time.elapsed();
                if response.status().is_success() {
                    HealthCheckResult {
                        status: HealthStatus::Healthy,
                        response_time,
                        last_checked: Instant::now(),
                        error_message: None,
                    }
                } else {
                    HealthCheckResult {
                        status: HealthStatus::Unhealthy,
                        response_time,
                        last_checked: Instant::now(),
                        error_message: Some(format!("HTTP {}", response.status())),
                    }
                }
            }
            Err(e) => {
                logger.log(format!("Nozomi health check failed: {}", e).red().to_string());
                HealthCheckResult {
                    status: HealthStatus::Unhealthy,
                    response_time: start_time.elapsed(),
                    last_checked: Instant::now(),
                    error_message: Some(e.to_string()),
                }
            }
        }
    }

    /// Get health status of a specific service
    pub async fn get_service_health(&self, service_name: &str) -> Option<HealthCheckResult> {
        let services = self.services.read().await;
        services.get(service_name).cloned()
    }

    /// Get the healthiest service for a given transaction landing mode
    pub async fn get_healthiest_service(&self, mode: &TransactionLandingMode) -> Option<String> {
        let services = self.services.read().await;
        
        let candidates = match mode {
            TransactionLandingMode::Zeroslot => vec!["zeroslot"],
            TransactionLandingMode::Nozomi => vec!["nozomi"],
        };
        
        // Find the healthiest service with the fastest response time
        let mut best_service = None;
        let mut best_response_time = Duration::from_secs(u64::MAX);
        
        for candidate in candidates {
            if let Some(health) = services.get(candidate) {
                if health.status == HealthStatus::Healthy && health.response_time < best_response_time {
                    best_response_time = health.response_time;
                    best_service = Some(candidate.to_string());
                }
            }
        }
        
        best_service
    }

    /// Get all service health statuses
    pub async fn get_all_service_health(&self) -> HashMap<String, HealthCheckResult> {
        let services = self.services.read().await;
        services.clone()
    }

    /// Check if a specific service is healthy
    pub async fn is_service_healthy(&self, service_name: &str) -> bool {
        let services = self.services.read().await;
        services.get(service_name)
            .map(|health| health.status == HealthStatus::Healthy)
            .unwrap_or(false)
    }
}

/// Global health check manager instance
lazy_static::lazy_static! {
    pub static ref HEALTH_CHECK_MANAGER: HealthCheckManager = HealthCheckManager::new();
}

/// Initialize and start the health check manager
pub async fn initialize_health_check_manager() -> Result<()> {
    HEALTH_CHECK_MANAGER.start().await
} 