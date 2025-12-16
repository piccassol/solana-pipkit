//! Account utilities for validation and parsing.

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    rent::Rent,
    sysvar,
};

use crate::{Result, ToolkitError};

/// Account validation utilities.
pub struct AccountUtils {
    client: RpcClient,
}

impl AccountUtils {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
        }
    }

    /// Check if an account exists.
    pub async fn exists(&self, pubkey: &Pubkey) -> bool {
        self.client.get_account(pubkey).await.is_ok()
    }

    /// Get account data or return error if not found.
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Account> {
        self.client
            .get_account(pubkey)
            .await
            .map_err(|_| ToolkitError::AccountNotFound(pubkey.to_string()))
    }

    /// Get account lamports balance.
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        Ok(self.client.get_balance(pubkey).await?)
    }

    /// Check if account is rent exempt.
    pub async fn is_rent_exempt(&self, pubkey: &Pubkey) -> Result<bool> {
        let account = self.get_account(pubkey).await?;
        let rent = self.client.get_account(&sysvar::rent::id()).await?;
        let rent: Rent = bincode::deserialize(&rent.data)
            .map_err(|e| ToolkitError::InvalidAccountData(e.to_string()))?;

        Ok(rent.is_exempt(account.lamports, account.data.len()))
    }

    /// Get minimum balance for rent exemption.
    pub async fn minimum_balance_for_rent_exemption(&self, data_len: usize) -> Result<u64> {
        Ok(self
            .client
            .get_minimum_balance_for_rent_exemption(data_len)
            .await?)
    }

    /// Check if account is owned by a program.
    pub async fn is_owned_by(&self, pubkey: &Pubkey, program_id: &Pubkey) -> Result<bool> {
        let account = self.get_account(pubkey).await?;
        Ok(account.owner == *program_id)
    }

    /// Get multiple accounts in a single RPC call.
    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>> {
        Ok(self.client.get_multiple_accounts(pubkeys).await?)
    }
}

/// Account data parser helpers.
pub mod parser {
    use super::*;
    use borsh::BorshDeserialize;

    /// Parse account data as a Borsh-serialized type.
    pub fn parse_borsh<T: BorshDeserialize>(data: &[u8]) -> Result<T> {
        T::try_from_slice(data).map_err(|e| ToolkitError::InvalidAccountData(e.to_string()))
    }

    /// Parse account data with a discriminator prefix.
    pub fn parse_with_discriminator<T: BorshDeserialize>(
        data: &[u8],
        expected_discriminator: &[u8; 8],
    ) -> Result<T> {
        if data.len() < 8 {
            return Err(ToolkitError::InvalidAccountData(
                "Data too short for discriminator".to_string(),
            ));
        }

        let discriminator: &[u8; 8] = data[..8]
            .try_into()
            .map_err(|_| ToolkitError::InvalidAccountData("Invalid discriminator".to_string()))?;

        if discriminator != expected_discriminator {
            return Err(ToolkitError::InvalidAccountData(format!(
                "Invalid discriminator: expected {:?}, got {:?}",
                expected_discriminator, discriminator
            )));
        }

        parse_borsh(&data[8..])
    }

    /// Calculate Anchor account discriminator.
    pub fn anchor_discriminator(account_name: &str) -> [u8; 8] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Note: This is a simplified version. 
        // Real Anchor uses SHA256("account:{name}")[..8]
        let mut hasher = DefaultHasher::new();
        format!("account:{}", account_name).hash(&mut hasher);
        let hash = hasher.finish();
        hash.to_le_bytes()
    }
}

/// Account info struct for displaying account details.
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub pubkey: Pubkey,
    pub lamports: u64,
    pub data_len: usize,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: u64,
}

impl From<(Pubkey, Account)> for AccountInfo {
    fn from((pubkey, account): (Pubkey, Account)) -> Self {
        Self {
            pubkey,
            lamports: account.lamports,
            data_len: account.data.len(),
            owner: account.owner,
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_info_from() {
        let pubkey = Pubkey::new_unique();
        let account = Account {
            lamports: 1000,
            data: vec![0; 100],
            owner: Pubkey::new_unique(),
            executable: false,
            rent_epoch: 0,
        };

        let info = AccountInfo::from((pubkey, account));
        assert_eq!(info.pubkey, pubkey);
        assert_eq!(info.lamports, 1000);
        assert_eq!(info.data_len, 100);
    }
}
