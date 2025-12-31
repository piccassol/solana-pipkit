//! LP token parsing and analysis utilities.

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use crate::{Result, ToolkitError};
use super::Dex;

#[derive(Debug, Clone)]
pub struct LpTokenInfo {
    pub mint: Pubkey,
    pub dex: Dex,
    pub pool: Pubkey,
    pub token_a: UnderlyingToken,
    pub token_b: UnderlyingToken,
    pub total_supply: u64,
    pub decimals: u8,
}

#[derive(Debug, Clone)]
pub struct UnderlyingToken {
    pub mint: Pubkey,
    pub symbol: Option<String>,
    pub decimals: u8,
    pub reserve: u64,
    pub price_usd: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct LpValue {
    pub lp_amount: u64,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub total_value_usd: Option<f64>,
    pub token_a_value_usd: Option<f64>,
    pub token_b_value_usd: Option<f64>,
}

pub struct LpTokenParser<'a> {
    client: &'a RpcClient,
}

impl<'a> LpTokenParser<'a> {
    pub fn new(client: &'a RpcClient) -> Self {
        Self { client }
    }

    pub async fn parse(&self, lp_mint: &Pubkey) -> Result<LpTokenInfo> {
        let mint_account = self.client
            .get_account(lp_mint)
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        if mint_account.data.len() < 82 {
            return Err(ToolkitError::ParseError("Invalid mint account".to_string()));
        }

        let total_supply = u64::from_le_bytes(
            mint_account.data[36..44].try_into()
                .map_err(|_| ToolkitError::ParseError("Invalid supply".to_string()))?
        );

        let decimals = mint_account.data[44];

        let (dex, pool) = self.find_pool_for_lp(lp_mint).await?;

        let pool_manager = super::PoolManager::new(self.client);
        let pool_info = pool_manager.get_info(&pool).await?;

        Ok(LpTokenInfo {
            mint: *lp_mint,
            dex,
            pool,
            token_a: UnderlyingToken {
                mint: pool_info.token_a.mint,
                symbol: pool_info.token_a.symbol,
                decimals: pool_info.token_a.decimals,
                reserve: pool_info.reserve_a,
                price_usd: None,
            },
            token_b: UnderlyingToken {
                mint: pool_info.token_b.mint,
                symbol: pool_info.token_b.symbol,
                decimals: pool_info.token_b.decimals,
                reserve: pool_info.reserve_b,
                price_usd: None,
            },
            total_supply,
            decimals,
        })
    }

    pub async fn calculate_value(&self, lp_mint: &Pubkey, lp_amount: u64) -> Result<LpValue> {
        let info = self.parse(lp_mint).await?;

        if info.total_supply == 0 {
            return Ok(LpValue {
                lp_amount,
                token_a_amount: 0,
                token_b_amount: 0,
                total_value_usd: None,
                token_a_value_usd: None,
                token_b_value_usd: None,
            });
        }

        let share = lp_amount as f64 / info.total_supply as f64;
        let token_a_amount = (info.token_a.reserve as f64 * share) as u64;
        let token_b_amount = (info.token_b.reserve as f64 * share) as u64;

        let token_a_value_usd = info.token_a.price_usd.map(|p| {
            (token_a_amount as f64 / 10_f64.powi(info.token_a.decimals as i32)) * p
        });

        let token_b_value_usd = info.token_b.price_usd.map(|p| {
            (token_b_amount as f64 / 10_f64.powi(info.token_b.decimals as i32)) * p
        });

        let total_value_usd = match (token_a_value_usd, token_b_value_usd) {
            (Some(a), Some(b)) => Some(a + b),
            _ => None,
        };

        Ok(LpValue {
            lp_amount,
            token_a_amount,
            token_b_amount,
            total_value_usd,
            token_a_value_usd,
            token_b_value_usd,
        })
    }

    async fn find_pool_for_lp(&self, lp_mint: &Pubkey) -> Result<(Dex, Pubkey)> {
        if let Some(pool) = self.check_raydium_pool(lp_mint).await? {
            return Ok((Dex::RaydiumAmm, pool));
        }

        if let Some(pool) = self.check_orca_pool(lp_mint).await? {
            return Ok((Dex::OrcaWhirlpool, pool));
        }

        Err(ToolkitError::NotFound(format!("No pool found for LP mint {}", lp_mint)))
    }

    async fn check_raydium_pool(&self, _lp_mint: &Pubkey) -> Result<Option<Pubkey>> {
        // Would query Raydium's pool registry
        Ok(None)
    }

    async fn check_orca_pool(&self, _lp_mint: &Pubkey) -> Result<Option<Pubkey>> {
        Ok(None)
    }
}

/// Calculate how much of each underlying token an LP position represents.
pub fn calculate_underlying_amounts(
    lp_amount: u64,
    total_lp_supply: u64,
    reserve_a: u64,
    reserve_b: u64,
) -> (u64, u64) {
    if total_lp_supply == 0 {
        return (0, 0);
    }

    let share = lp_amount as u128;
    let supply = total_lp_supply as u128;

    let amount_a = ((reserve_a as u128 * share) / supply) as u64;
    let amount_b = ((reserve_b as u128 * share) / supply) as u64;

    (amount_a, amount_b)
}

/// Calculate LP token value in USD.
#[allow(clippy::too_many_arguments)]
pub fn calculate_lp_value_usd(
    lp_amount: u64,
    total_lp_supply: u64,
    reserve_a: u64,
    reserve_b: u64,
    price_a_usd: f64,
    price_b_usd: f64,
    decimals_a: u8,
    decimals_b: u8,
) -> f64 {
    let (amount_a, amount_b) = calculate_underlying_amounts(
        lp_amount,
        total_lp_supply,
        reserve_a,
        reserve_b,
    );

    let value_a = (amount_a as f64 / 10_f64.powi(decimals_a as i32)) * price_a_usd;
    let value_b = (amount_b as f64 / 10_f64.powi(decimals_b as i32)) * price_b_usd;

    value_a + value_b
}

/// Calculate price per LP token in USD.
pub fn calculate_lp_token_price(
    total_lp_supply: u64,
    lp_decimals: u8,
    tvl_usd: f64,
) -> f64 {
    if total_lp_supply == 0 {
        return 0.0;
    }
    let supply_adjusted = total_lp_supply as f64 / 10_f64.powi(lp_decimals as i32);
    tvl_usd / supply_adjusted
}

/// Check if LP token is from a known DEX.
pub fn identify_lp_token_dex(mint_authority: &Option<Pubkey>) -> Option<Dex> {
    match mint_authority {
        Some(auth) => {
            if auth == &*super::programs::RAYDIUM_AMM_V4 {
                Some(Dex::RaydiumAmm)
            } else if auth == &*super::programs::ORCA_WHIRLPOOL {
                Some(Dex::OrcaWhirlpool)
            } else {
                None
            }
        }
        None => None,
    }
}
