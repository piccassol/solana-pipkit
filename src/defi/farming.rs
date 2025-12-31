//! Yield farming position management.

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use crate::{Result, ToolkitError};

/// Known farming program IDs
pub mod farm_programs {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    lazy_static::lazy_static! {
        pub static ref RAYDIUM_STAKING: Pubkey = 
            Pubkey::from_str("EhhTKczWMGQt46ynNeRX1WfeagwwJd7ufHvCDjRxjo5Q").unwrap();
        pub static ref ORCA_AQUAFARM: Pubkey = 
            Pubkey::from_str("82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ").unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct FarmPosition {
    pub farm: Pubkey,
    pub wallet: Pubkey,
    pub staked_amount: u64,
    pub pending_rewards: Vec<PendingReward>,
    pub stake_timestamp: i64,
    pub farm_info: FarmInfo,
}

#[derive(Debug, Clone)]
pub struct PendingReward {
    pub token_mint: Pubkey,
    pub symbol: Option<String>,
    pub amount: u64,
    pub value_usd: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct FarmInfo {
    pub address: Pubkey,
    pub staked_token: Pubkey,
    pub reward_tokens: Vec<Pubkey>,
    pub total_staked: u64,
    pub apr: Option<f64>,
    pub daily_rewards: Option<f64>,
    pub lock_period: Option<u64>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct FarmStats {
    pub total_value_locked_usd: f64,
    pub participant_count: u64,
    pub reward_rate_per_day: f64,
    pub avg_stake_duration_days: f64,
}

pub struct FarmManager<'a> {
    client: &'a RpcClient,
}

impl<'a> FarmManager<'a> {
    pub fn new(client: &'a RpcClient) -> Self {
        Self { client }
    }

    pub async fn get_position(&self, wallet: &Pubkey, farm: &Pubkey) -> Result<FarmPosition> {
        // Get farm account to determine the program/type
        let farm_account = self.client
            .get_account(farm)
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        let farm_info = self.parse_farm_info(farm, &farm_account.data)?;

        // Find user's stake account
        let staked_amount = self.get_user_stake(wallet, farm).await?;
        let pending_rewards = self.get_pending_rewards(wallet, farm).await?;

        Ok(FarmPosition {
            farm: *farm,
            wallet: *wallet,
            staked_amount,
            pending_rewards,
            stake_timestamp: 0, // Would need to track from transaction history
            farm_info,
        })
    }

    pub async fn get_farm_info(&self, farm: &Pubkey) -> Result<FarmInfo> {
        let account = self.client
            .get_account(farm)
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        self.parse_farm_info(farm, &account.data)
    }

    pub async fn list_user_farms(&self, _wallet: &Pubkey) -> Result<Vec<FarmPosition>> {
        // In production, would query known farm programs for user accounts
        // For now, return empty
        Ok(Vec::new())
    }

    async fn get_user_stake(&self, wallet: &Pubkey, farm: &Pubkey) -> Result<u64> {
        // Get farm account to determine the program
        let farm_account = self.client
            .get_account(farm)
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        // Derive user stake account PDA based on the farm's owning program
        let program_id = &farm_account.owner;

        // Different programs use different PDA seeds
        let stake_pda = if program_id == &*farm_programs::RAYDIUM_STAKING {
            // Raydium: [farm, wallet, "stake_info"]
            let (pda, _) = Pubkey::find_program_address(
                &[farm.as_ref(), wallet.as_ref(), b"stake_info"],
                program_id,
            );
            pda
        } else if program_id == &*farm_programs::ORCA_AQUAFARM {
            // Orca: [farm, wallet, "user"]
            let (pda, _) = Pubkey::find_program_address(
                &[farm.as_ref(), wallet.as_ref(), b"user"],
                program_id,
            );
            pda
        } else {
            // Unknown program - try generic seeds
            let (pda, _) = Pubkey::find_program_address(
                &[farm.as_ref(), wallet.as_ref()],
                program_id,
            );
            pda
        };

        match self.client.get_account(&stake_pda) {
            Ok(account) => {
                if account.data.len() >= 8 {
                    let amount = u64::from_le_bytes(
                        account.data[0..8].try_into()
                            .map_err(|_| ToolkitError::ParseError("Invalid stake amount".to_string()))?
                    );
                    Ok(amount)
                } else {
                    Ok(0)
                }
            }
            Err(_) => Ok(0),
        }
    }

    async fn get_pending_rewards(&self, _wallet: &Pubkey, _farm: &Pubkey) -> Result<Vec<PendingReward>> {
        // Would calculate based on stake duration and reward rate
        // Requires knowing farm's reward schedule
        Ok(Vec::new())
    }

    fn parse_farm_info(&self, farm: &Pubkey, data: &[u8]) -> Result<FarmInfo> {
        // Simplified - actual parsing depends on specific farm program
        if data.len() < 100 {
            return Err(ToolkitError::ParseError("Invalid farm data".to_string()));
        }

        // Extract staked token mint (offset varies by program)
        let staked_token = Pubkey::try_from(&data[8..40])
            .map_err(|_| ToolkitError::ParseError("Invalid staked token".to_string()))?;

        Ok(FarmInfo {
            address: *farm,
            staked_token,
            reward_tokens: Vec::new(),
            total_staked: 0,
            apr: None,
            daily_rewards: None,
            lock_period: None,
            is_active: true,
        })
    }
}

/// Calculate APR from reward rate and TVL.
pub fn calculate_apr(
    daily_reward_value_usd: f64,
    total_staked_value_usd: f64,
) -> f64 {
    if total_staked_value_usd == 0.0 {
        return 0.0;
    }
    (daily_reward_value_usd / total_staked_value_usd) * 365.0 * 100.0
}

/// Calculate APY from APR with compounding frequency.
pub fn apr_to_apy(apr: f64, compounds_per_year: u32) -> f64 {
    let r = apr / 100.0;
    let n = compounds_per_year as f64;
    ((1.0 + r / n).powf(n) - 1.0) * 100.0
}

/// Calculate pending rewards based on time staked.
pub fn calculate_pending_rewards(
    staked_amount: u64,
    reward_rate_per_second: f64,
    seconds_staked: u64,
    total_staked: u64,
) -> u64 {
    if total_staked == 0 {
        return 0;
    }
    let share = staked_amount as f64 / total_staked as f64;
    (reward_rate_per_second * seconds_staked as f64 * share) as u64
}

/// Estimate break-even time for staking given gas costs.
pub fn calculate_break_even_time(
    stake_gas_cost_usd: f64,
    unstake_gas_cost_usd: f64,
    claim_gas_cost_usd: f64,
    daily_reward_usd: f64,
) -> Option<f64> {
    if daily_reward_usd <= 0.0 {
        return None;
    }
    let total_gas = stake_gas_cost_usd + unstake_gas_cost_usd + claim_gas_cost_usd;
    Some(total_gas / daily_reward_usd)
}

/// Check if farming is profitable after IL and gas.
pub fn is_farming_profitable(
    expected_apr: f64,
    expected_il_pct: f64,
    gas_cost_pct: f64,
) -> bool {
    expected_apr > expected_il_pct + gas_cost_pct
}
