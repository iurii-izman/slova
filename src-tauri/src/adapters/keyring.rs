// ============================================================================
// OS Keyring Adapter
// ============================================================================
// Secure storage for API credentials using OS-native keychains:
// - Windows: Credential Manager via DPAPI
// - macOS: Keychain
// - Linux: Secret Service or pass

use crate::types::AppErrorView;

const SERVICE_NAME: &str = "VideoTranscriber";
const GROQ_KEY: &str = "groq_api_key";

/// Wrapper around OS keyring for secure credential storage
pub struct KeyringAdapter;

impl KeyringAdapter {
    /// Store API key in OS keychain
    /// service: "VideoTranscriber"
    /// username: "groq_api_key"
    pub fn save_api_key(key: &str) -> Result<(), AppErrorView> {
        if key.is_empty() {
            return Err(AppErrorView::new(
                "INVALID_INPUT",
                "API key cannot be empty",
            ));
        }

        let entry = keyring::Entry::new(SERVICE_NAME, GROQ_KEY)
            .map_err(|e| AppErrorView::internal_error(format!("Keyring setup failed: {}", e)))?;

        entry
            .set_password(key)
            .map_err(|e| AppErrorView::internal_error(format!("Failed to save API key: {}", e)))
    }

    /// Retrieve API key from OS keychain
    pub fn get_api_key() -> Result<Option<String>, AppErrorView> {
        let entry = keyring::Entry::new(SERVICE_NAME, GROQ_KEY)
            .map_err(|e| AppErrorView::internal_error(format!("Keyring setup failed: {}", e)))?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AppErrorView::internal_error(format!(
                "Failed to retrieve API key: {}",
                e
            ))),
        }
    }

    /// Check if API key is stored
    pub fn has_api_key() -> Result<bool, AppErrorView> {
        match Self::get_api_key() {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Delete API key from OS keychain
    pub fn delete_api_key() -> Result<(), AppErrorView> {
        let entry = keyring::Entry::new(SERVICE_NAME, GROQ_KEY)
            .map_err(|e| AppErrorView::internal_error(format!("Keyring setup failed: {}", e)))?;

        entry
            .delete_password()
            .map_err(|e| AppErrorView::internal_error(format!("Failed to delete API key: {}", e)))
    }
}
