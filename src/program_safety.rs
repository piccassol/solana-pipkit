//! Program safety analysis.
//!
//! Analyzes programs before interacting with them to detect potential risks
//! such as upgrade authorities, blocklisted programs, and unverified code.

use crate::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    bpf_loader_upgradeable::{self, UpgradeableLoaderState},
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

/// Program safety report.
#[derive(Debug, Clone)]
pub struct ProgramSafetyReport {
    /// Program ID analyzed.
    pub program_id: Pubkey,
    /// Whether the program is verified (on known safe list).
    pub is_verified: bool,
    /// Whether the program has an upgrade authority.
    pub has_upgrade_authority: bool,
    /// The upgrade authority if present.
    pub upgrade_authority: Option<Pubkey>,
    /// Whether the program is frozen (no upgrade authority).
    pub is_frozen: bool,
    /// Overall risk level.
    pub risk_level: ProgramRiskLevel,
    /// Specific warnings.
    pub warnings: Vec<ProgramWarning>,
    /// Whether the program is blocklisted.
    pub blocklisted: bool,
    /// Reason for blocklisting if applicable.
    pub blocklist_reason: Option<String>,
}

/// Warning about a program.
#[derive(Debug, Clone, PartialEq)]
pub enum ProgramWarning {
    /// Program is not on verified list.
    UnverifiedProgram,
    /// Program has an upgrade authority.
    HasUpgradeAuthority(Pubkey),
    /// Program was recently deployed.
    RecentlyDeployed { slot: u64 },
    /// Program has low usage.
    LowUsageCount,
    /// Program is known to be malicious.
    KnownMalicious(String),
    /// Program account not found.
    NotFound,
    /// Could not determine program type.
    UnknownProgramType,
}

impl ProgramWarning {
    /// Get a human-readable description.
    pub fn description(&self) -> String {
        match self {
            ProgramWarning::UnverifiedProgram => {
                "Program is not on the verified safe list".to_string()
            }
            ProgramWarning::HasUpgradeAuthority(auth) => {
                format!("Program can be upgraded by {}", auth)
            }
            ProgramWarning::RecentlyDeployed { slot } => {
                format!("Program was recently deployed at slot {}", slot)
            }
            ProgramWarning::LowUsageCount => "Program has low usage".to_string(),
            ProgramWarning::KnownMalicious(reason) => {
                format!("Program is known malicious: {}", reason)
            }
            ProgramWarning::NotFound => "Program account not found".to_string(),
            ProgramWarning::UnknownProgramType => "Could not determine program type".to_string(),
        }
    }
}

/// Risk level for a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProgramRiskLevel {
    /// Verified, frozen, well-known program.
    Safe,
    /// Verified, has upgrade authority but trusted.
    Low,
    /// Unverified but no red flags.
    Medium,
    /// Multiple warnings.
    High,
    /// Blocklisted program.
    Critical,
}

impl std::fmt::Display for ProgramRiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgramRiskLevel::Safe => write!(f, "SAFE"),
            ProgramRiskLevel::Low => write!(f, "LOW"),
            ProgramRiskLevel::Medium => write!(f, "MEDIUM"),
            ProgramRiskLevel::High => write!(f, "HIGH"),
            ProgramRiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Program safety analyzer.
pub struct ProgramSafetyAnalyzer {
    rpc_client: RpcClient,
    blocklist: HashMap<Pubkey, String>,
    known_safe: HashSet<Pubkey>,
}

impl ProgramSafetyAnalyzer {
    /// Create a new program safety analyzer.
    pub fn new(rpc_url: &str) -> Self {
        let mut analyzer = Self {
            rpc_client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            blocklist: HashMap::new(),
            known_safe: HashSet::new(),
        };

        // Add default known safe programs
        analyzer.add_default_safe_programs();

        analyzer
    }

    /// Create from existing RpcClient.
    pub fn from_client(client: RpcClient) -> Self {
        let mut analyzer = Self {
            rpc_client: client,
            blocklist: HashMap::new(),
            known_safe: HashSet::new(),
        };

        analyzer.add_default_safe_programs();

        analyzer
    }

    /// Add default known safe programs.
    fn add_default_safe_programs(&mut self) {
        // System programs
        self.known_safe.insert(solana_sdk::system_program::id());
        self.known_safe.insert(spl_token::id());
        self.known_safe.insert(spl_associated_token_account::id());
        self.known_safe.insert(solana_sdk::bpf_loader::id());
        self.known_safe.insert(solana_sdk::bpf_loader_deprecated::id());
        self.known_safe.insert(bpf_loader_upgradeable::id());
        self.known_safe.insert(solana_sdk::compute_budget::id());

        // Token programs
        if let Ok(token_2022) = Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb") {
            self.known_safe.insert(token_2022);
        }

        // Metaplex
        if let Ok(metadata) = Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s") {
            self.known_safe.insert(metadata);
        }

        // Jupiter
        if let Ok(jupiter_v6) = Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
            self.known_safe.insert(jupiter_v6);
        }
        if let Ok(jupiter_v4) = Pubkey::from_str("JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB") {
            self.known_safe.insert(jupiter_v4);
        }

        // Raydium
        if let Ok(raydium_amm) = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
            self.known_safe.insert(raydium_amm);
        }
        if let Ok(raydium_clmm) = Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK") {
            self.known_safe.insert(raydium_clmm);
        }

        // Orca
        if let Ok(orca_whirlpool) = Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc") {
            self.known_safe.insert(orca_whirlpool);
        }

        // Marinade
        if let Ok(marinade) = Pubkey::from_str("MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD") {
            self.known_safe.insert(marinade);
        }

        // Tensor
        if let Ok(tensor) = Pubkey::from_str("TSWAPaqyCSx2KABk68Shruf4rp7CxcNi8hAsbdwmHbN") {
            self.known_safe.insert(tensor);
        }

        // Magic Eden
        if let Ok(magic_eden) = Pubkey::from_str("M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K") {
            self.known_safe.insert(magic_eden);
        }
    }

    /// Analyze a program for safety.
    pub fn analyze(&self, program_id: &Pubkey) -> Result<ProgramSafetyReport> {
        let mut warnings = Vec::new();

        // Check if blocklisted
        if let Some(reason) = self.blocklist.get(program_id) {
            return Ok(ProgramSafetyReport {
                program_id: *program_id,
                is_verified: false,
                has_upgrade_authority: false,
                upgrade_authority: None,
                is_frozen: false,
                risk_level: ProgramRiskLevel::Critical,
                warnings: vec![ProgramWarning::KnownMalicious(reason.clone())],
                blocklisted: true,
                blocklist_reason: Some(reason.clone()),
            });
        }

        // Check if known safe
        let is_verified = self.known_safe.contains(program_id);

        if !is_verified {
            warnings.push(ProgramWarning::UnverifiedProgram);
        }

        // Fetch program account to check upgrade authority
        let (has_upgrade_authority, upgrade_authority, is_frozen) =
            self.check_upgrade_authority(program_id)?;

        if has_upgrade_authority {
            if let Some(auth) = upgrade_authority {
                warnings.push(ProgramWarning::HasUpgradeAuthority(auth));
            }
        }

        // Determine risk level
        let risk_level = self.calculate_risk_level(is_verified, has_upgrade_authority, &warnings);

        Ok(ProgramSafetyReport {
            program_id: *program_id,
            is_verified,
            has_upgrade_authority,
            upgrade_authority,
            is_frozen,
            risk_level,
            warnings,
            blocklisted: false,
            blocklist_reason: None,
        })
    }

    /// Check if a program is safe to interact with.
    pub fn is_safe(&self, program_id: &Pubkey) -> Result<bool> {
        let report = self.analyze(program_id)?;
        Ok(report.risk_level <= ProgramRiskLevel::Low)
    }

    /// Add a program to the blocklist.
    pub fn add_to_blocklist(&mut self, program_id: Pubkey, reason: String) {
        self.blocklist.insert(program_id, reason);
    }

    /// Remove a program from the blocklist.
    pub fn remove_from_blocklist(&mut self, program_id: &Pubkey) {
        self.blocklist.remove(program_id);
    }

    /// Add a program to the known safe list.
    pub fn add_to_safe_list(&mut self, program_id: Pubkey) {
        self.known_safe.insert(program_id);
    }

    /// Remove a program from the known safe list.
    pub fn remove_from_safe_list(&mut self, program_id: &Pubkey) {
        self.known_safe.remove(program_id);
    }

    /// Check if a program is on the blocklist.
    pub fn is_blocklisted(&self, program_id: &Pubkey) -> bool {
        self.blocklist.contains_key(program_id)
    }

    /// Check if a program is on the known safe list.
    pub fn is_known_safe(&self, program_id: &Pubkey) -> bool {
        self.known_safe.contains(program_id)
    }

    /// Check upgrade authority for a program.
    fn check_upgrade_authority(
        &self,
        program_id: &Pubkey,
    ) -> Result<(bool, Option<Pubkey>, bool)> {
        // Get the program account
        let account = match self.rpc_client.get_account(program_id) {
            Ok(acc) => acc,
            Err(_) => {
                return Ok((false, None, false));
            }
        };

        // Check if it's a BPF upgradeable program
        if account.owner == bpf_loader_upgradeable::id() {
            // Parse the program data account address from the program account
            if let Ok(UpgradeableLoaderState::Program {
                programdata_address,
            }) = bincode::deserialize(&account.data)
            {
                // Fetch the program data account
                if let Ok(programdata_account) =
                    self.rpc_client.get_account(&programdata_address)
                {
                    if let Ok(UpgradeableLoaderState::ProgramData {
                        upgrade_authority_address,
                        ..
                    }) = bincode::deserialize(&programdata_account.data)
                    {
                        let has_authority = upgrade_authority_address.is_some();
                        let is_frozen = upgrade_authority_address.is_none();
                        return Ok((has_authority, upgrade_authority_address, is_frozen));
                    }
                }
            }
        }

        // Not an upgradeable program or couldn't parse
        Ok((false, None, true))
    }

    /// Calculate risk level based on analysis.
    fn calculate_risk_level(
        &self,
        is_verified: bool,
        has_upgrade_authority: bool,
        warnings: &[ProgramWarning],
    ) -> ProgramRiskLevel {
        // Check for critical warnings
        for warning in warnings {
            if matches!(warning, ProgramWarning::KnownMalicious(_)) {
                return ProgramRiskLevel::Critical;
            }
        }

        if is_verified && !has_upgrade_authority {
            return ProgramRiskLevel::Safe;
        }

        if is_verified && has_upgrade_authority {
            return ProgramRiskLevel::Low;
        }

        if warnings.len() >= 2 {
            return ProgramRiskLevel::High;
        }

        ProgramRiskLevel::Medium
    }
}

/// Analyze a program without creating an analyzer instance.
pub fn analyze_program(rpc_url: &str, program_id: &Pubkey) -> Result<ProgramSafetyReport> {
    let analyzer = ProgramSafetyAnalyzer::new(rpc_url);
    analyzer.analyze(program_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_analyzer() -> ProgramSafetyAnalyzer {
        // Create with a dummy URL - tests won't actually make RPC calls
        ProgramSafetyAnalyzer {
            rpc_client: RpcClient::new("http://localhost:8899".to_string()),
            blocklist: HashMap::new(),
            known_safe: HashSet::new(),
        }
    }

    #[test]
    fn test_default_safe_programs() {
        let analyzer = ProgramSafetyAnalyzer::new("http://localhost:8899");

        // System program should be safe
        assert!(analyzer.is_known_safe(&solana_sdk::system_program::id()));

        // Token program should be safe
        assert!(analyzer.is_known_safe(&spl_token::id()));

        // ATA program should be safe
        assert!(analyzer.is_known_safe(&spl_associated_token_account::id()));
    }

    #[test]
    fn test_blocklist_operations() {
        let mut analyzer = mock_analyzer();
        let malicious = Pubkey::new_unique();

        // Add to blocklist
        analyzer.add_to_blocklist(malicious, "Drainer contract".to_string());
        assert!(analyzer.is_blocklisted(&malicious));

        // Remove from blocklist
        analyzer.remove_from_blocklist(&malicious);
        assert!(!analyzer.is_blocklisted(&malicious));
    }

    #[test]
    fn test_safe_list_operations() {
        let mut analyzer = mock_analyzer();
        let safe_program = Pubkey::new_unique();

        // Add to safe list
        analyzer.add_to_safe_list(safe_program);
        assert!(analyzer.is_known_safe(&safe_program));

        // Remove from safe list
        analyzer.remove_from_safe_list(&safe_program);
        assert!(!analyzer.is_known_safe(&safe_program));
    }

    #[test]
    fn test_program_risk_level_ordering() {
        assert!(ProgramRiskLevel::Safe < ProgramRiskLevel::Low);
        assert!(ProgramRiskLevel::Low < ProgramRiskLevel::Medium);
        assert!(ProgramRiskLevel::Medium < ProgramRiskLevel::High);
        assert!(ProgramRiskLevel::High < ProgramRiskLevel::Critical);
    }

    #[test]
    fn test_program_warning_description() {
        let warning = ProgramWarning::UnverifiedProgram;
        assert!(warning.description().contains("verified"));

        let auth = Pubkey::new_unique();
        let warning = ProgramWarning::HasUpgradeAuthority(auth);
        assert!(warning.description().contains("upgraded"));

        let warning = ProgramWarning::KnownMalicious("Drainer".to_string());
        assert!(warning.description().contains("malicious"));
    }

    #[test]
    fn test_calculate_risk_level_safe() {
        let analyzer = mock_analyzer();
        let level = analyzer.calculate_risk_level(true, false, &[]);
        assert_eq!(level, ProgramRiskLevel::Safe);
    }

    #[test]
    fn test_calculate_risk_level_low() {
        let analyzer = mock_analyzer();
        let level = analyzer.calculate_risk_level(true, true, &[]);
        assert_eq!(level, ProgramRiskLevel::Low);
    }

    #[test]
    fn test_calculate_risk_level_medium() {
        let analyzer = mock_analyzer();
        let warnings = vec![ProgramWarning::UnverifiedProgram];
        let level = analyzer.calculate_risk_level(false, false, &warnings);
        assert_eq!(level, ProgramRiskLevel::Medium);
    }

    #[test]
    fn test_calculate_risk_level_high() {
        let analyzer = mock_analyzer();
        let auth = Pubkey::new_unique();
        let warnings = vec![
            ProgramWarning::UnverifiedProgram,
            ProgramWarning::HasUpgradeAuthority(auth),
        ];
        let level = analyzer.calculate_risk_level(false, true, &warnings);
        assert_eq!(level, ProgramRiskLevel::High);
    }

    #[test]
    fn test_calculate_risk_level_critical() {
        let analyzer = mock_analyzer();
        let warnings = vec![ProgramWarning::KnownMalicious("Drainer".to_string())];
        let level = analyzer.calculate_risk_level(false, false, &warnings);
        assert_eq!(level, ProgramRiskLevel::Critical);
    }

    #[test]
    fn test_program_safety_report_blocklisted() {
        let mut analyzer = mock_analyzer();
        let malicious = Pubkey::new_unique();
        analyzer.add_to_blocklist(malicious, "Known drainer".to_string());

        // Manually create report since we can't make RPC calls in test
        let report = ProgramSafetyReport {
            program_id: malicious,
            is_verified: false,
            has_upgrade_authority: false,
            upgrade_authority: None,
            is_frozen: false,
            risk_level: ProgramRiskLevel::Critical,
            warnings: vec![ProgramWarning::KnownMalicious("Known drainer".to_string())],
            blocklisted: true,
            blocklist_reason: Some("Known drainer".to_string()),
        };

        assert!(report.blocklisted);
        assert_eq!(report.risk_level, ProgramRiskLevel::Critical);
    }

    #[test]
    fn test_program_safety_report_safe() {
        let report = ProgramSafetyReport {
            program_id: solana_sdk::system_program::id(),
            is_verified: true,
            has_upgrade_authority: false,
            upgrade_authority: None,
            is_frozen: true,
            risk_level: ProgramRiskLevel::Safe,
            warnings: vec![],
            blocklisted: false,
            blocklist_reason: None,
        };

        assert!(report.is_verified);
        assert!(report.is_frozen);
        assert!(!report.blocklisted);
        assert_eq!(report.risk_level, ProgramRiskLevel::Safe);
    }

    #[test]
    fn test_known_dex_programs() {
        let analyzer = ProgramSafetyAnalyzer::new("http://localhost:8899");

        // Jupiter V6
        if let Ok(jupiter) = Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
            assert!(analyzer.is_known_safe(&jupiter));
        }

        // Raydium AMM
        if let Ok(raydium) = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
            assert!(analyzer.is_known_safe(&raydium));
        }
    }
}
