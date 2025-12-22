//! SPL Token utilities for common operations.
//!
//! Provides helpers for token minting, burning, transfers, and account management.

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token::{
    instruction as token_instruction,
    solana_program::program_pack::Pack,
    state::{Account as TokenAccount, Mint},
};

use crate::{pda::find_associated_token_address, Result, ToolkitError};

/// Token client for SPL token operations.
pub struct TokenClient {
    client: RpcClient,
    payer: Keypair,
}

impl TokenClient {
    pub fn new(rpc_url: &str, payer: Keypair) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            payer,
        }
    }

    /// Burn tokens from a token account.
    pub async fn burn(
        &self,
        mint: &Pubkey,
        token_account: &Pubkey,
        amount: u64,
    ) -> Result<()> {
        let instruction = token_instruction::burn(
            &spl_token::id(),
            token_account,
            mint,
            &self.payer.pubkey(),
            &[],
            amount,
        )?;

        self.send_transaction(vec![instruction]).await
    }

    /// Burn tokens and close the account if empty.
    pub async fn burn_and_close(
        &self,
        mint: &Pubkey,
        token_account: &Pubkey,
        amount: u64,
    ) -> Result<u64> {
        let mut instructions = vec![];

        // Burn instruction
        instructions.push(token_instruction::burn(
            &spl_token::id(),
            token_account,
            mint,
            &self.payer.pubkey(),
            &[],
            amount,
        )?);

        // Close account instruction
        instructions.push(token_instruction::close_account(
            &spl_token::id(),
            token_account,
            &self.payer.pubkey(),
            &self.payer.pubkey(),
            &[],
        )?);

        // Get account balance before closing
        let account = self.client.get_account(token_account).await?;
        let lamports = account.lamports;

        self.send_transaction(instructions).await?;

        Ok(lamports)
    }

    /// Transfer tokens between accounts.
    pub async fn transfer(
        &self,
        _mint: &Pubkey,
        source: &Pubkey,
        destination: &Pubkey,
        amount: u64,
    ) -> Result<()> {
        let instruction = token_instruction::transfer(
            &spl_token::id(),
            source,
            destination,
            &self.payer.pubkey(),
            &[],
            amount,
        )?;

        self.send_transaction(vec![instruction]).await
    }

    /// Create an associated token account.
    pub async fn create_associated_token_account(
        &self,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Pubkey> {
        let (ata, _) = find_associated_token_address(wallet, mint);

        let instruction =
            spl_associated_token_account::instruction::create_associated_token_account(
                &self.payer.pubkey(),
                wallet,
                mint,
                &spl_token::id(),
            );

        self.send_transaction(vec![instruction]).await?;

        Ok(ata)
    }

    /// Get or create associated token account.
    pub async fn get_or_create_ata(
        &self,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Pubkey> {
        let (ata, _) = find_associated_token_address(wallet, mint);

        // Check if account exists
        match self.client.get_account(&ata).await {
            Ok(_) => Ok(ata),
            Err(_) => self.create_associated_token_account(wallet, mint).await,
        }
    }

    /// Close a token account and recover rent.
    pub async fn close_account(&self, token_account: &Pubkey) -> Result<u64> {
        let account = self.client.get_account(token_account).await?;
        let lamports = account.lamports;

        let instruction = token_instruction::close_account(
            &spl_token::id(),
            token_account,
            &self.payer.pubkey(),
            &self.payer.pubkey(),
            &[],
        )?;

        self.send_transaction(vec![instruction]).await?;

        Ok(lamports)
    }

    /// Get token account balance.
    pub async fn get_balance(&self, token_account: &Pubkey) -> Result<u64> {
        let account = self.client.get_account(token_account).await?;
        let token_account = TokenAccount::unpack(&account.data)
            .map_err(|e| ToolkitError::InvalidAccountData(e.to_string()))?;
        Ok(token_account.amount)
    }

    /// Get mint info.
    pub async fn get_mint_info(&self, mint: &Pubkey) -> Result<MintInfo> {
        let account = self.client.get_account(mint).await?;
        let mint_data = Mint::unpack(&account.data)
            .map_err(|e| ToolkitError::InvalidAccountData(e.to_string()))?;

        Ok(MintInfo {
            supply: mint_data.supply,
            decimals: mint_data.decimals,
            is_initialized: mint_data.is_initialized,
            mint_authority: mint_data.mint_authority.into(),
            freeze_authority: mint_data.freeze_authority.into(),
        })
    }

    async fn send_transaction(&self, instructions: Vec<Instruction>) -> Result<()> {
        let recent_blockhash = self.client.get_latest_blockhash().await?;
        let message = Message::new(&instructions, Some(&self.payer.pubkey()));
        let transaction = Transaction::new(&[&self.payer], message, recent_blockhash);

        self.client
            .send_and_confirm_transaction(&transaction)
            .await?;

        Ok(())
    }
}

/// Mint information.
#[derive(Debug, Clone)]
pub struct MintInfo {
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub mint_authority: Option<Pubkey>,
    pub freeze_authority: Option<Pubkey>,
}

/// Standalone burn function.
pub async fn burn_tokens(
    rpc_url: &str,
    payer: Keypair,
    mint: &Pubkey,
    token_account: &Pubkey,
    amount: u64,
) -> Result<()> {
    let client = TokenClient::new(rpc_url, payer);
    client.burn(mint, token_account, amount).await
}

/// Standalone close account function.
pub async fn close_token_account(
    rpc_url: &str,
    payer: Keypair,
    token_account: &Pubkey,
) -> Result<u64> {
    let client = TokenClient::new(rpc_url, payer);
    client.close_account(token_account).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mint_info_debug() {
        let info = MintInfo {
            supply: 1_000_000,
            decimals: 9,
            is_initialized: true,
            mint_authority: None,
            freeze_authority: None,
        };
        assert_eq!(info.decimals, 9);
    }
}
