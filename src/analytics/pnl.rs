//! Profit and Loss tracking for wallet positions.

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use crate::{Result, ToolkitError};

#[derive(Debug, Clone)]
pub struct PnlReport {
    pub wallet: Pubkey,
    pub total_realized_pnl: f64,
    pub total_unrealized_pnl: f64,
    pub positions: Vec<Position>,
    pub closed_positions: Vec<ClosedPosition>,
    pub summary: PnlSummary,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub token_mint: Pubkey,
    pub symbol: Option<String>,
    pub balance: u64,
    pub avg_cost_basis: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_pct: f64,
    pub first_buy: i64,
    pub total_cost: f64,
    pub current_value: f64,
}

#[derive(Debug, Clone)]
pub struct ClosedPosition {
    pub token_mint: Pubkey,
    pub symbol: Option<String>,
    pub total_bought: u64,
    pub total_sold: u64,
    pub avg_buy_price: f64,
    pub avg_sell_price: f64,
    pub realized_pnl: f64,
    pub realized_pnl_pct: f64,
    pub hold_duration_days: f64,
}

#[derive(Debug, Clone)]
pub struct PnlSummary {
    pub total_invested: f64,
    pub current_portfolio_value: f64,
    pub total_pnl: f64,
    pub total_pnl_pct: f64,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: f64,
    pub best_trade: Option<TradeResult>,
    pub worst_trade: Option<TradeResult>,
    pub avg_hold_time_days: f64,
}

#[derive(Debug, Clone)]
pub struct TradeResult {
    pub token_mint: Pubkey,
    pub pnl: f64,
    pub pnl_pct: f64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TradeEntry {
    timestamp: i64,
    is_buy: bool,
    amount: u64,
    price: f64,
    total_value: f64,
}

pub struct PnlTracker<'a> {
    client: &'a RpcClient,
    price_cache: HashMap<Pubkey, f64>,
}

impl<'a> PnlTracker<'a> {
    pub fn new(client: &'a RpcClient) -> Self {
        Self {
            client,
            price_cache: HashMap::new(),
        }
    }

    pub fn with_prices(client: &'a RpcClient, prices: HashMap<Pubkey, f64>) -> Self {
        Self {
            client,
            price_cache: prices,
        }
    }

    pub async fn calculate(&self, wallet: &Pubkey) -> Result<PnlReport> {
        // Get current token holdings
        let token_accounts = self.client
            .get_token_accounts_by_owner(
                wallet,
                solana_client::rpc_request::TokenAccountsFilter::ProgramId(spl_token::id()),
            )
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        let mut positions = Vec::new();
        let mut total_unrealized = 0.0;
        let mut total_cost = 0.0;
        let mut current_value = 0.0;

        for account in &token_accounts {
            let token_account_pubkey: Pubkey = account.pubkey.parse()
                .map_err(|_| ToolkitError::ParseError(format!("Invalid token account pubkey: {}", account.pubkey)))?;
            if let Some(position) = self.analyze_position(wallet, &token_account_pubkey).await? {
                total_unrealized += position.unrealized_pnl;
                total_cost += position.total_cost;
                current_value += position.current_value;
                positions.push(position);
            }
        }

        // For now, we don't have historical trade data to calculate realized PnL
        // This would require parsing transaction history and matching buys/sells
        let closed_positions = Vec::new();
        let total_realized = 0.0;

        let summary = self.calculate_summary(
            &positions,
            &closed_positions,
            total_cost,
            current_value,
        );

        Ok(PnlReport {
            wallet: *wallet,
            total_realized_pnl: total_realized,
            total_unrealized_pnl: total_unrealized,
            positions,
            closed_positions,
            summary,
        })
    }

    async fn analyze_position(
        &self,
        _wallet: &Pubkey,
        token_account: &Pubkey,
    ) -> Result<Option<Position>> {
        let account_info = self.client
            .get_account(token_account)
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        if account_info.data.len() < 72 {
            return Ok(None);
        }

        let mint = Pubkey::try_from(&account_info.data[0..32])
            .map_err(|_| ToolkitError::ParseError("Invalid mint".to_string()))?;
        
        let balance = u64::from_le_bytes(
            account_info.data[64..72].try_into()
                .map_err(|_| ToolkitError::ParseError("Invalid balance".to_string()))?
        );

        if balance == 0 {
            return Ok(None);
        }

        // Get current price (from cache or would need external API)
        let current_price = self.price_cache.get(&mint).copied().unwrap_or(0.0);

        // Without historical data, we estimate cost basis at current price
        // Real implementation would track actual purchases
        let avg_cost_basis = current_price;
        let total_cost = avg_cost_basis * balance as f64;
        let current_value = current_price * balance as f64;
        let unrealized_pnl = current_value - total_cost;
        let unrealized_pnl_pct = if total_cost > 0.0 {
            (unrealized_pnl / total_cost) * 100.0
        } else {
            0.0
        };

        Ok(Some(Position {
            token_mint: mint,
            symbol: None,
            balance,
            avg_cost_basis,
            current_price,
            unrealized_pnl,
            unrealized_pnl_pct,
            first_buy: 0,
            total_cost,
            current_value,
        }))
    }

    fn calculate_summary(
        &self,
        _positions: &[Position],
        closed: &[ClosedPosition],
        total_invested: f64,
        current_value: f64,
    ) -> PnlSummary {
        let winning = closed.iter().filter(|p| p.realized_pnl > 0.0).count();
        let losing = closed.iter().filter(|p| p.realized_pnl < 0.0).count();
        let total_trades = winning + losing;

        let win_rate = if total_trades > 0 {
            (winning as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };

        let best_trade = closed
            .iter()
            .max_by(|a, b| a.realized_pnl.partial_cmp(&b.realized_pnl).unwrap())
            .map(|p| TradeResult {
                token_mint: p.token_mint,
                pnl: p.realized_pnl,
                pnl_pct: p.realized_pnl_pct,
            });

        let worst_trade = closed
            .iter()
            .min_by(|a, b| a.realized_pnl.partial_cmp(&b.realized_pnl).unwrap())
            .map(|p| TradeResult {
                token_mint: p.token_mint,
                pnl: p.realized_pnl,
                pnl_pct: p.realized_pnl_pct,
            });

        let avg_hold_time = if closed.is_empty() {
            0.0
        } else {
            closed.iter().map(|p| p.hold_duration_days).sum::<f64>() / closed.len() as f64
        };

        let total_pnl = current_value - total_invested;
        let total_pnl_pct = if total_invested > 0.0 {
            (total_pnl / total_invested) * 100.0
        } else {
            0.0
        };

        PnlSummary {
            total_invested,
            current_portfolio_value: current_value,
            total_pnl,
            total_pnl_pct,
            winning_trades: winning,
            losing_trades: losing,
            win_rate,
            best_trade,
            worst_trade,
            avg_hold_time_days: avg_hold_time,
        }
    }
}

/// Calculate simple PnL from buy/sell prices.
pub fn calculate_simple_pnl(buy_price: f64, sell_price: f64, amount: f64) -> (f64, f64) {
    let pnl = (sell_price - buy_price) * amount;
    let pnl_pct = if buy_price > 0.0 {
        ((sell_price - buy_price) / buy_price) * 100.0
    } else {
        0.0
    };
    (pnl, pnl_pct)
}

/// Calculate cost basis using FIFO method.
pub fn calculate_fifo_cost_basis(buys: &[(u64, f64)], sell_amount: u64) -> f64 {
    let mut remaining = sell_amount;
    let mut total_cost = 0.0;

    for (amount, price) in buys {
        if remaining == 0 {
            break;
        }
        let use_amount = remaining.min(*amount);
        total_cost += use_amount as f64 * price;
        remaining -= use_amount;
    }

    if sell_amount > 0 {
        total_cost / sell_amount as f64
    } else {
        0.0
    }
}

/// Calculate cost basis using average method.
pub fn calculate_avg_cost_basis(buys: &[(u64, f64)]) -> f64 {
    let total_amount: u64 = buys.iter().map(|(a, _)| a).sum();
    let total_cost: f64 = buys.iter().map(|(a, p)| *a as f64 * p).sum();

    if total_amount > 0 {
        total_cost / total_amount as f64
    } else {
        0.0
    }
}

/// Format PnL for display.
pub fn format_pnl(pnl: f64, pnl_pct: f64) -> String {
    let sign = if pnl >= 0.0 { "+" } else { "" };
    format!("{}{:.2} ({}{:.2}%)", sign, pnl, sign, pnl_pct)
}
