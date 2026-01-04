//! CryptoBot API integration for payment processing
//! API docs: https://help.send.tg/en/articles/10279948-crypto-pay-api
//!
//! This module provides a client for the CryptoBot API to handle
//! cryptocurrency payments. The integration requires:
//! 1. Setting up CRYPTOBOT_API_TOKEN environment variable
//! 2. Configuring webhook endpoint to receive payment notifications
//! 3. Using the CryptoBot service in the app state

#![allow(dead_code)]

use std::collections::HashMap;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

// Re-import json crate with proper alias
use json as serde_json;

/// CryptoBot API base URLs
pub const MAINNET_URL: &str = "https://pay.crypt.bot/api/";
pub const TESTNET_URL: &str = "https://testnet-pay.crypt.bot/api/";

/// Invoice status enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvoiceStatus {
  Active,
  Paid,
  Expired,
}

/// Invoice response from CryptoBot API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
  pub invoice_id: i64,
  pub hash: String,
  pub currency_type: String,
  pub asset: Option<String>,
  pub fiat: Option<String>,
  pub amount: String,
  pub paid_asset: Option<String>,
  pub paid_amount: Option<String>,
  pub paid_fiat_rate: Option<String>,
  pub accepted_assets: Option<Vec<String>>,
  pub fee_asset: Option<String>,
  pub fee_amount: Option<String>,
  pub fee: Option<String>,
  pub pay_url: String,
  pub bot_invoice_url: String,
  pub mini_app_invoice_url: Option<String>,
  pub web_app_invoice_url: Option<String>,
  pub description: Option<String>,
  pub status: InvoiceStatus,
  pub created_at: String,
  pub paid_usd_rate: Option<String>,
  pub usd_rate: Option<String>,
  pub allow_comments: bool,
  pub allow_anonymous: bool,
  pub expiration_date: Option<String>,
  pub paid_at: Option<String>,
  pub paid_anonymously: Option<bool>,
  pub comment: Option<String>,
  pub hidden_message: Option<String>,
  pub payload: Option<String>,
  pub paid_btn_name: Option<String>,
  pub paid_btn_url: Option<String>,
}

/// API response wrapper
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
  pub ok: bool,
  pub result: Option<T>,
  pub error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
  pub code: i32,
  pub name: String,
}

/// App info from getMe
#[derive(Debug, Deserialize)]
pub struct AppInfo {
  pub app_id: i64,
  pub name: String,
  pub payment_processing_bot_username: String,
}

/// Balance info
#[derive(Debug, Deserialize)]
pub struct BalanceItem {
  pub currency_code: String,
  pub available: String,
  pub onhold: String,
}

/// Webhook update from CryptoBot
#[derive(Debug, Deserialize)]
pub struct WebhookUpdate {
  pub update_id: i64,
  pub update_type: String,
  pub request_date: String,
  pub payload: Invoice,
}

/// Parameters for creating an invoice
#[derive(Debug, Clone, Serialize)]
pub struct CreateInvoiceParams {
  /// Currency code (USDT, TON, BTC, etc.) or fiat currency
  pub asset: Option<String>,
  /// Amount of the invoice
  pub amount: String,
  /// Description of the invoice
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  /// Text shown after payment (up to 2048 chars)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub hidden_message: Option<String>,
  /// Data to associate with the invoice (up to 4kb)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub payload: Option<String>,
  /// Expiration time in seconds (1-2678400)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub expires_in: Option<i32>,
  /// Allow user to pay with different crypto
  #[serde(skip_serializing_if = "Option::is_none")]
  pub accepted_assets: Option<Vec<String>>,
  /// Allow user to add a comment
  #[serde(skip_serializing_if = "Option::is_none")]
  pub allow_comments: Option<bool>,
  /// Allow anonymous payment
  #[serde(skip_serializing_if = "Option::is_none")]
  pub allow_anonymous: Option<bool>,
}

/// CryptoBot client for payment processing
#[derive(Clone)]
pub struct CryptoBot {
  client: Client,
  base_url: String,
  api_token: String,
}

impl CryptoBot {
  /// Create a new CryptoBot client
  pub fn new(api_token: String, use_testnet: bool) -> Self {
    let base_url = if use_testnet {
      TESTNET_URL.to_string()
    } else {
      MAINNET_URL.to_string()
    };

    Self { client: Client::new(), base_url, api_token }
  }

  /// Make an API request
  async fn request<T: for<'de> Deserialize<'de>>(
    &self,
    method: &str,
    params: Option<HashMap<String, String>>,
  ) -> Result<T> {
    let url = format!("{}{}", self.base_url, method);

    let mut request =
      self.client.get(&url).header("Crypto-Pay-API-Token", &self.api_token);

    if let Some(p) = params {
      request = request.query(&p);
    }

    let response = request
      .send()
      .await
      .map_err(|e| Error::CryptoBot(format!("Request failed: {}", e)))?;

    let api_response: ApiResponse<T> = response.json().await.map_err(|e| {
      Error::CryptoBot(format!("Failed to parse response: {}", e))
    })?;

    if api_response.ok {
      api_response
        .result
        .ok_or_else(|| Error::CryptoBot("Empty result".to_string()))
    } else {
      let err = api_response.error.map_or_else(
        || "Unknown error".to_string(),
        |e| format!("{}: {}", e.name, e.code),
      );
      Err(Error::CryptoBot(err))
    }
  }

  /// Make a POST request with JSON body
  async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
    &self,
    method: &str,
    body: &B,
  ) -> Result<T> {
    let url = format!("{}{}", self.base_url, method);

    let response = self
      .client
      .post(&url)
      .header("Crypto-Pay-API-Token", &self.api_token)
      .json(body)
      .send()
      .await
      .map_err(|e| Error::CryptoBot(format!("Request failed: {}", e)))?;

    let api_response: ApiResponse<T> = response.json().await.map_err(|e| {
      Error::CryptoBot(format!("Failed to parse response: {}", e))
    })?;

    if api_response.ok {
      api_response
        .result
        .ok_or_else(|| Error::CryptoBot("Empty result".to_string()))
    } else {
      let err = api_response.error.map_or_else(
        || "Unknown error".to_string(),
        |e| format!("{}: {}", e.name, e.code),
      );
      Err(Error::CryptoBot(err))
    }
  }

  /// Test API connection and get app info
  pub async fn get_me(&self) -> Result<AppInfo> {
    self.request("getMe", None).await
  }

  /// Get app balance
  pub async fn get_balance(&self) -> Result<Vec<BalanceItem>> {
    self.request("getBalance", None).await
  }

  /// Create a payment invoice
  pub async fn create_invoice(
    &self,
    params: CreateInvoiceParams,
  ) -> Result<Invoice> {
    self.post("createInvoice", &params).await
  }

  /// Get invoices with optional filters
  pub async fn get_invoices(
    &self,
    invoice_ids: Option<Vec<i64>>,
    status: Option<InvoiceStatus>,
  ) -> Result<Vec<Invoice>> {
    let mut params = HashMap::new();

    if let Some(ids) = invoice_ids {
      params.insert(
        "invoice_ids".to_string(),
        ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","),
      );
    }

    if let Some(s) = status {
      let status_str = match s {
        InvoiceStatus::Active => "active",
        InvoiceStatus::Paid => "paid",
        InvoiceStatus::Expired => "expired",
      };
      params.insert("status".to_string(), status_str.to_string());
    }

    let params = if params.is_empty() { None } else { Some(params) };

    #[derive(Deserialize)]
    struct ItemsResponse {
      items: Vec<Invoice>,
    }

    let response: ItemsResponse = self.request("getInvoices", params).await?;
    Ok(response.items)
  }

  /// Get a single invoice by ID
  pub async fn get_invoice(&self, invoice_id: i64) -> Result<Invoice> {
    let invoices = self.get_invoices(Some(vec![invoice_id]), None).await?;
    invoices.into_iter().next().ok_or(Error::InvoiceNotFound)
  }

  /// Delete an invoice
  pub async fn delete_invoice(&self, invoice_id: i64) -> Result<bool> {
    let mut params = HashMap::new();
    params.insert("invoice_id".to_string(), invoice_id.to_string());
    self.request("deleteInvoice", Some(params)).await
  }

  /// Create an invoice for depositing USDT
  pub async fn create_deposit_invoice(
    &self,
    user_id: i64,
    amount_usdt: f64,
    referrer_id: Option<i64>,
  ) -> Result<Invoice> {
    let payload = serde_json::json!({
      "type": "deposit",
      "user_id": user_id,
      "referrer_id": referrer_id,
    })
    .to_string();

    let params = CreateInvoiceParams {
      asset: Some("USDT".to_string()),
      amount: format!("{:.2}", amount_usdt),
      description: Some(format!(
        "Deposit {} USDT for user {}",
        amount_usdt, user_id
      )),
      hidden_message: Some(
        "Thank you for your deposit! Your balance has been updated."
          .to_string(),
      ),
      payload: Some(payload),
      expires_in: Some(3600), // 1 hour
      accepted_assets: Some(vec![
        "USDT".to_string(),
        "TON".to_string(),
        "BTC".to_string(),
      ]),
      allow_comments: Some(true),
      allow_anonymous: Some(false),
    };

    self.create_invoice(params).await
  }

  /// Create an invoice for purchasing a license
  pub async fn create_license_invoice(
    &self,
    user_id: i64,
    license_type: &str,
    price_usdt: f64,
    referrer_id: Option<i64>,
    discount_percent: Option<i32>,
  ) -> Result<Invoice> {
    let discounted_price = if let Some(discount) = discount_percent {
      price_usdt * (100 - discount) as f64 / 100.0
    } else {
      price_usdt
    };

    let payload = serde_json::json!({
      "type": "license_purchase",
      "user_id": user_id,
      "license_type": license_type,
      "original_price": price_usdt,
      "discount_percent": discount_percent,
      "referrer_id": referrer_id,
    })
    .to_string();

    let description = if let Some(discount) = discount_percent {
      format!(
        "{} license for user {} ({}% discount)",
        license_type, user_id, discount
      )
    } else {
      format!("{} license for user {}", license_type, user_id)
    };

    let params = CreateInvoiceParams {
      asset: Some("USDT".to_string()),
      amount: format!("{:.2}", discounted_price),
      description: Some(description),
      hidden_message: Some(
        "Thank you for your purchase! Your license is now active.".to_string(),
      ),
      payload: Some(payload),
      expires_in: Some(3600), // 1 hour
      accepted_assets: Some(vec![
        "USDT".to_string(),
        "TON".to_string(),
        "BTC".to_string(),
      ]),
      allow_comments: Some(true),
      allow_anonymous: Some(false),
    };

    self.create_invoice(params).await
  }

  /// Parse webhook payload data
  pub fn parse_payload(payload: &str) -> Option<PaymentPayload> {
    serde_json::from_str(payload).ok()
  }

  /// Verify webhook signature
  pub fn verify_signature(
    api_token: &str,
    body: &[u8],
    signature: &str,
  ) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    // Create secret from SHA256 hash of API token
    let token_hash = {
      use sha2::Digest;
      let mut hasher = Sha256::new();
      hasher.update(api_token.as_bytes());
      hasher.finalize()
    };

    // Compute HMAC-SHA256 of body
    let mut mac = HmacSha256::new_from_slice(&token_hash)
      .expect("HMAC can take key of any size");
    mac.update(body);

    // Verify signature
    let expected = hex::encode(mac.finalize().into_bytes());
    expected == signature
  }
}

/// Parsed webhook payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPayload {
  #[serde(rename = "type")]
  pub payment_type: String,
  pub user_id: i64,
  pub license_type: Option<String>,
  pub original_price: Option<f64>,
  pub discount_percent: Option<i32>,
  pub referrer_id: Option<i64>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_deposit_payload() {
    let payload = r#"{"type":"deposit","user_id":12345,"referrer_id":null}"#;
    let parsed = CryptoBot::parse_payload(payload).unwrap();
    assert_eq!(parsed.payment_type, "deposit");
    assert_eq!(parsed.user_id, 12345);
    assert!(parsed.referrer_id.is_none());
  }

  #[test]
  fn test_parse_license_payload() {
    let payload = r#"{"type":"license_purchase","user_id":12345,"license_type":"month","original_price":10.0,"discount_percent":3,"referrer_id":67890}"#;
    let parsed = CryptoBot::parse_payload(payload).unwrap();
    assert_eq!(parsed.payment_type, "license_purchase");
    assert_eq!(parsed.user_id, 12345);
    assert_eq!(parsed.license_type.unwrap(), "month");
    assert_eq!(parsed.discount_percent.unwrap(), 3);
    assert_eq!(parsed.referrer_id.unwrap(), 67890);
  }
}
