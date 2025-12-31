//! Liquidity provision utilities.

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use crate::{Result, ToolkitError};
use super::{DeFiConfig, PoolInfo};

#[derive(Debug, Clone)]
pub struct LiquidityQuote {
    pub pool: Pubkey,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub lp_tokens_received: u64,
    pub share_of_pool_pct: f64,
    pub price_impact_pct: f64,
    pub min_lp_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct RemoveLiquidityQuote {
    pub pool: Pubkey,
    pub lp_tokens_burned: u64,
    pub token_a_received: u64,
    pub token_b_received: u64,
    pub share_removed_pct: f64,
}

#[derive(Debug, Clone)]
pub struct LiquidityPosition {
    pub pool: Pubkey,
    pub lp_balance: u64,
    pub share_pct: f64,
    pub token_a_value: u64,
    pub token_b_value: u64,
    pub total_value_usd: Option<f64>,
    pub unclaimed_fees: Option<UnclaimedFees>,
}

#[derive(Debug, Clone)]
pub struct UnclaimedFees {
    pub token_a: u64,
    pub token_b: u64,
    pub value_usd: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ConcentratedPosition {
    pub pool: Pubkey,
    pub lower_tick: i32,
    pub upper_tick: i32,
    pub liquidity: u128,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub in_range: bool,
    pub unclaimed_fees: UnclaimedFees,
}

pub struct LiquidityProvider<'a> {
    client: &'a RpcClient,
    config: DeFiConfig,
}

impl<'a> LiquidityProvider<'a> {
    pub fn new(client: &'a RpcClient, config: DeFiConfig) -> Self {
        Self { client, config }
    }

    pub async fn quote(&self, pool: &Pubkey, amount_a: u64) -> Result<LiquidityQuote> {
        let pool_manager = super::PoolManager::new(self.client);
        let pool_info = pool_manager.get_info(pool).await?;

        let (amount_b, lp_tokens, share_pct) = self.calculate_amounts(
            &pool_info,
            amount_a,
        )?;

        let min_lp_tokens = lp_tokens * (10000 - self.config.default_slippage_bps as u64) / 10000;

        Ok(LiquidityQuote {
            pool: *pool,
            token_a_amount: amount_a,
            token_b_amount: amount_b,
            lp_tokens_received: lp_tokens,
            share_of_pool_pct: share_pct,
            price_impact_pct: 0.0, // Minimal for balanced adds
            min_lp_tokens,
        })
    }

    pub async fn quote_remove(
        &self,
        pool: &Pubkey,
        lp_amount: u64,
    ) -> Result<RemoveLiquidityQuote> {
        let pool_manager = super::PoolManager::new(self.client);
        let pool_info = pool_manager.get_info(pool).await?;

        let share_pct = if pool_info.lp_supply > 0 {
            (lp_amount as f64 / pool_info.lp_supply as f64) * 100.0
        } else {
            0.0
        };

        let token_a_received = (pool_info.reserve_a as f64 * share_pct / 100.0) as u64;
        let token_b_received = (pool_info.reserve_b as f64 * share_pct / 100.0) as u64;

        Ok(RemoveLiquidityQuote {
            pool: *pool,
            lp_tokens_burned: lp_amount,
            token_a_received,
            token_b_received,
            share_removed_pct: share_pct,
        })
    }

    pub async fn get_position(
        &self,
        wallet: &Pubkey,
        pool: &Pubkey,
    ) -> Result<Option<LiquidityPosition>> {
        let pool_manager = super::PoolManager::new(self.client);
        let pool_info = pool_manager.get_info(pool).await?;

        // Find LP token account for this wallet
        let lp_balance = self.get_lp_balance(wallet, &pool_info.lp_mint).await?;

        if lp_balance == 0 {
            return Ok(None);
        }

        let share_pct = if pool_info.lp_supply > 0 {
            (lp_balance as f64 / pool_info.lp_supply as f64) * 100.0
        } else {
            0.0
        };

        let token_a_value = (pool_info.reserve_a as f64 * share_pct / 100.0) as u64;
        let token_b_value = (pool_info.reserve_b as f64 * share_pct / 100.0) as u64;

        Ok(Some(LiquidityPosition {
            pool: *pool,
            lp_balance,
            share_pct,
            token_a_value,
            token_b_value,
            total_value_usd: None, // Would need price data
            unclaimed_fees: None,  // Protocol specific
        }))
    }

    fn calculate_amounts(
        &self,
        pool: &PoolInfo,
        amount_a: u64,
    ) -> Result<(u64, u64, f64)> {
        if pool.reserve_a == 0 || pool.reserve_b == 0 {
            return Err(ToolkitError::InvalidInput("Pool has no liquidity".to_string()));
        }

        // For constant product AMM: amount_b = amount_a * reserve_b / reserve_a
        let amount_b = (amount_a as u128 * pool.reserve_b as u128 / pool.reserve_a as u128) as u64;

        // LP tokens received (simplified - actual calculation varies by DEX)
        let lp_tokens = if pool.lp_supply == 0 {
            // First liquidity provider
            ((amount_a as u128 * amount_b as u128).integer_sqrt()) as u64
        } else {
            // Proportional to contribution
            let share_a = amount_a as u128 * pool.lp_supply as u128 / pool.reserve_a as u128;
            let share_b = amount_b as u128 * pool.lp_supply as u128 / pool.reserve_b as u128;
            share_a.min(share_b) as u64
        };

        let share_pct = if pool.lp_supply > 0 {
            (lp_tokens as f64 / (pool.lp_supply + lp_tokens) as f64) * 100.0
        } else {
            100.0
        };

        Ok((amount_b, lp_tokens, share_pct))
    }

    async fn get_lp_balance(&self, wallet: &Pubkey, lp_mint: &Pubkey) -> Result<u64> {
        let ata = spl_associated_token_account::get_associated_token_address(wallet, lp_mint);
        
        match self.client.get_account(&ata) {
            Ok(account) => {
                if account.data.len() >= 72 {
                    let amount = u64::from_le_bytes(
                        account.data[64..72].try_into()
                            .map_err(|_| ToolkitError::ParseError("Invalid balance".to_string()))?
                    );
                    Ok(amount)
                } else {
                    Ok(0)
                }
            }
            Err(_) => Ok(0),
        }
    }
}

/// Calculate optimal liquidity ratio for imbalanced adds.
pub fn calculate_optimal_ratio(reserve_a: u64, reserve_b: u64) -> f64 {
    reserve_b as f64 / reserve_a as f64
}

/// Calculate impermanent loss percentage.
pub fn calculate_impermanent_loss(initial_price_ratio: f64, current_price_ratio: f64) -> f64 {
    let price_change = current_price_ratio / initial_price_ratio;
    let sqrt_price = price_change.sqrt();
    let il = (2.0 * sqrt_price / (1.0 + price_change)) - 1.0;
    il.abs() * 100.0
}

/// Estimate APY from fee revenue.
pub fn estimate_fee_apy(
    daily_volume: f64,
    fee_rate_bps: u16,
    tvl: f64,
) -> f64 {
    if tvl == 0.0 {
        return 0.0;
    }
    let daily_fees = daily_volume * (fee_rate_bps as f64 / 10000.0);
    let daily_return = daily_fees / tvl;
    ((1.0 + daily_return).powi(365) - 1.0) * 100.0
}

/// Check if position is in range for concentrated liquidity.
pub fn is_position_in_range(current_tick: i32, lower_tick: i32, upper_tick: i32) -> bool {
    current_tick >= lower_tick && current_tick < upper_tick
}

trait IntegerSquareRoot {
    fn integer_sqrt(self) -> Self;
}

impl IntegerSquareRoot for u128 {
    fn integer_sqrt(self) -> Self {
        if self == 0 {
            return 0;
        }
        let mut x = self;
        let mut y = x.div_ceil(2);
        while y < x {
            x = y;
            y = (x + self / x) / 2;
        }
        x
    }
}
