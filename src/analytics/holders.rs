//! Token holder analysis and distribution tracking.

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use crate::{Result, ToolkitError};

#[derive(Debug, Clone)]
pub struct HolderAnalysis {
    pub mint: Pubkey,
    pub total_supply: u64,
    pub holder_count: usize,
    pub distribution: HolderDistribution,
    pub top_holders: Vec<HolderInfo>,
    pub concentration: ConcentrationMetrics,
    pub flags: Vec<HolderFlag>,
}

#[derive(Debug, Clone)]
pub struct HolderDistribution {
    pub top_1_pct: f64,
    pub top_5_pct: f64,
    pub top_10_pct: f64,
    pub top_20_pct: f64,
    pub top_50_pct: f64,
    pub gini_coefficient: f64,
}

#[derive(Debug, Clone)]
pub struct HolderInfo {
    pub address: Pubkey,
    pub balance: u64,
    pub percentage: f64,
    pub rank: usize,
    pub is_exchange: bool,
    pub is_contract: bool,
}

#[derive(Debug, Clone)]
pub struct ConcentrationMetrics {
    /// Herfindahl-Hirschman Index (0-10000, higher = more concentrated)
    pub hhi: f64,
    /// Number of holders needed to reach 50% supply
    pub nakamoto_coefficient: usize,
    /// Whether single entity could dump price significantly
    pub dump_risk: DumpRisk,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DumpRisk {
    Critical, // Single holder > 20%
    High,     // Top 5 holders > 50%
    Medium,   // Top 10 holders > 50%
    Low,      // Well distributed
}

#[derive(Debug, Clone)]
pub enum HolderFlag {
    HighConcentration,
    WhalePresence,
    ExchangeHeavy,
    FewHolders,
    HealthyDistribution,
    NewToken,
}

pub struct HolderAnalyzer<'a> {
    client: &'a RpcClient,
}

impl<'a> HolderAnalyzer<'a> {
    pub fn new(client: &'a RpcClient) -> Self {
        Self { client }
    }

    pub async fn analyze(&self, mint: &Pubkey) -> Result<HolderAnalysis> {
        let accounts = self.client
            .get_program_accounts_with_config(
                &spl_token::id(),
                solana_client::rpc_config::RpcProgramAccountsConfig {
                    filters: Some(vec![
                        solana_client::rpc_filter::RpcFilterType::Memcmp(
                            solana_client::rpc_filter::Memcmp::new_raw_bytes(0, mint.to_bytes().to_vec()),
                        ),
                        solana_client::rpc_filter::RpcFilterType::DataSize(165),
                    ]),
                    account_config: solana_client::rpc_config::RpcAccountInfoConfig {
                        encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        let mut holders: Vec<(Pubkey, u64)> = Vec::new();
        let mut total_supply: u64 = 0;

        for (pubkey, account) in &accounts {
            // Account data is raw bytes from get_program_accounts
            let data = &account.data;
            if data.len() >= 72 {
                let amount = u64::from_le_bytes(data[64..72].try_into().unwrap());
                if amount > 0 {
                    // Get the owner from the token account data (bytes 32-64)
                    let owner = Pubkey::try_from(&data[32..64]).unwrap_or(*pubkey);
                    holders.push((owner, amount));
                    total_supply = total_supply.saturating_add(amount);
                }
            }
        }

        holders.sort_by(|a, b| b.1.cmp(&a.1));

        let holder_count = holders.len();
        let distribution = self.calculate_distribution(&holders, total_supply);
        let top_holders = self.extract_top_holders(&holders, total_supply, 20);
        let concentration = self.calculate_concentration(&holders, total_supply);
        let flags = self.detect_flags(&distribution, &concentration, holder_count);

        Ok(HolderAnalysis {
            mint: *mint,
            total_supply,
            holder_count,
            distribution,
            top_holders,
            concentration,
            flags,
        })
    }

    fn calculate_distribution(&self, holders: &[(Pubkey, u64)], total: u64) -> HolderDistribution {
        if holders.is_empty() || total == 0 {
            return HolderDistribution {
                top_1_pct: 0.0,
                top_5_pct: 0.0,
                top_10_pct: 0.0,
                top_20_pct: 0.0,
                top_50_pct: 0.0,
                gini_coefficient: 0.0,
            };
        }

        let total_f = total as f64;
        let n = holders.len();

        let top_1_idx = (n as f64 * 0.01).ceil() as usize;
        let top_5_idx = (n as f64 * 0.05).ceil() as usize;
        let top_10_idx = (n as f64 * 0.10).ceil() as usize;
        let top_20_idx = (n as f64 * 0.20).ceil() as usize;
        let top_50_idx = (n as f64 * 0.50).ceil() as usize;

        let sum_top = |idx: usize| -> f64 {
            holders.iter().take(idx.max(1)).map(|(_, b)| *b as f64).sum::<f64>() / total_f * 100.0
        };

        let gini = self.calculate_gini(holders);

        HolderDistribution {
            top_1_pct: sum_top(top_1_idx),
            top_5_pct: sum_top(top_5_idx),
            top_10_pct: sum_top(top_10_idx),
            top_20_pct: sum_top(top_20_idx),
            top_50_pct: sum_top(top_50_idx),
            gini_coefficient: gini,
        }
    }

    fn calculate_gini(&self, holders: &[(Pubkey, u64)]) -> f64 {
        if holders.len() < 2 {
            return 0.0;
        }

        let n = holders.len() as f64;
        let balances: Vec<f64> = holders.iter().map(|(_, b)| *b as f64).collect();
        let mean = balances.iter().sum::<f64>() / n;

        if mean == 0.0 {
            return 0.0;
        }

        let mut sum = 0.0;
        for i in 0..holders.len() {
            for j in 0..holders.len() {
                sum += (balances[i] - balances[j]).abs();
            }
        }

        sum / (2.0 * n * n * mean)
    }

    fn extract_top_holders(
        &self,
        holders: &[(Pubkey, u64)],
        total: u64,
        count: usize,
    ) -> Vec<HolderInfo> {
        let total_f = total as f64;

        holders
            .iter()
            .take(count)
            .enumerate()
            .map(|(idx, (addr, balance))| {
                let percentage = if total > 0 {
                    (*balance as f64 / total_f) * 100.0
                } else {
                    0.0
                };

                HolderInfo {
                    address: *addr,
                    balance: *balance,
                    percentage,
                    rank: idx + 1,
                    is_exchange: super::wallet::is_exchange(addr),
                    is_contract: false, // Would need additional RPC call to determine
                }
            })
            .collect()
    }

    fn calculate_concentration(
        &self,
        holders: &[(Pubkey, u64)],
        total: u64,
    ) -> ConcentrationMetrics {
        if holders.is_empty() || total == 0 {
            return ConcentrationMetrics {
                hhi: 0.0,
                nakamoto_coefficient: 0,
                dump_risk: DumpRisk::Low,
            };
        }

        let total_f = total as f64;

        // HHI calculation
        let hhi: f64 = holders
            .iter()
            .map(|(_, b)| {
                let share = (*b as f64 / total_f) * 100.0;
                share * share
            })
            .sum();

        // Nakamoto coefficient
        let mut cumulative = 0.0;
        let mut nakamoto = 0;
        for (_, balance) in holders {
            cumulative += *balance as f64 / total_f;
            nakamoto += 1;
            if cumulative >= 0.5 {
                break;
            }
        }

        // Dump risk assessment
        let top_1_pct = holders.first().map(|(_, b)| *b as f64 / total_f * 100.0).unwrap_or(0.0);
        let top_5_pct: f64 = holders
            .iter()
            .take(5)
            .map(|(_, b)| *b as f64 / total_f * 100.0)
            .sum();
        let top_10_pct: f64 = holders
            .iter()
            .take(10)
            .map(|(_, b)| *b as f64 / total_f * 100.0)
            .sum();

        let dump_risk = if top_1_pct > 20.0 {
            DumpRisk::Critical
        } else if top_5_pct > 50.0 {
            DumpRisk::High
        } else if top_10_pct > 50.0 {
            DumpRisk::Medium
        } else {
            DumpRisk::Low
        };

        ConcentrationMetrics {
            hhi,
            nakamoto_coefficient: nakamoto,
            dump_risk,
        }
    }

    fn detect_flags(
        &self,
        distribution: &HolderDistribution,
        _concentration: &ConcentrationMetrics,
        holder_count: usize,
    ) -> Vec<HolderFlag> {
        let mut flags = Vec::new();

        if distribution.gini_coefficient > 0.8 {
            flags.push(HolderFlag::HighConcentration);
        }

        if distribution.top_1_pct > 10.0 {
            flags.push(HolderFlag::WhalePresence);
        }

        if holder_count < 100 {
            flags.push(HolderFlag::FewHolders);
        }

        if holder_count < 50 {
            flags.push(HolderFlag::NewToken);
        }

        if distribution.gini_coefficient < 0.5 && holder_count > 1000 {
            flags.push(HolderFlag::HealthyDistribution);
        }

        flags
    }
}

/// Quick check for whale concentration in a token.
pub fn check_whale_concentration(holders: &[(Pubkey, u64)], total: u64, threshold_pct: f64) -> bool {
    if holders.is_empty() || total == 0 {
        return false;
    }

    let top_holder_pct = holders.first().map(|(_, b)| *b as f64 / total as f64 * 100.0).unwrap_or(0.0);
    top_holder_pct >= threshold_pct
}

/// Calculate what percentage of supply is held by top N holders.
pub fn top_n_percentage(holders: &[(Pubkey, u64)], total: u64, n: usize) -> f64 {
    if holders.is_empty() || total == 0 {
        return 0.0;
    }

    let top_sum: u64 = holders.iter().take(n).map(|(_, b)| *b).sum();
    (top_sum as f64 / total as f64) * 100.0
}
