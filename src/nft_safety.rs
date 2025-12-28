//! NFT safety analysis for detecting fake NFTs and verifying collections.
//!
//! Provides tools to verify NFT authenticity, detect counterfeits,
//! and validate collection membership before purchasing or interacting.

use crate::{Result, ToolkitError};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

/// Metaplex Token Metadata Program ID.
pub const METAPLEX_METADATA_PROGRAM: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";

/// Known verified collections with their collection addresses.
pub const KNOWN_COLLECTIONS: &[(&str, &str)] = &[
    ("DeGods", "6XxjKYFbcndh2gDcsUrmZgVEsoDxXMnfsaGY6fpTJzNr"),
    ("y00ts", "4mKSoDDqApmF1DqXvVTSL6tu2zixrSSNjqMxUnwvVzy2"),
    ("Mad Lads", "J1S9H3QjnRtBbbuD4HjPV6RpRhwuk4zKbxsnCHuTgh9w"),
    ("Okay Bears", "3saAedkM9o5g1u5DCqsuMZuC4GRqPB4TuMkvSsSVvGQ3"),
    ("Claynosaurz", "6mszaj17KSfVqADrQj3o4W3zoLMTykgmV37W4QadCczK"),
    ("Famous Fox Federation", "BUjZjAS2vbbb65g7Z1Ca9ZRVYoJscURG5L3AkVvHP9ac"),
    ("Solana Monkey Business", "SMBtHCCC6RYRutFEPb4gZqeBLUZbMNhRKaMKZZLHi7W"),
    ("Aurory", "FEg3mmpcrcRwqLbXNgPMZbqg4TaQrZLLrmYvJB3KVGrb"),
];

/// Indicators of a potentially fake NFT.
#[derive(Debug, Clone, PartialEq)]
pub enum FakeIndicator {
    /// No verified creator found.
    NoVerifiedCreator,
    /// Creator address does not match known collection.
    CreatorMismatch { expected: Pubkey, found: Pubkey },
    /// Collection not verified on chain.
    UnverifiedCollection,
    /// Metadata name does not match collection pattern.
    NameMismatch { expected_pattern: String, found: String },
    /// Symbol does not match collection.
    SymbolMismatch { expected: String, found: String },
    /// Seller fee basis points unusual for collection.
    UnusualSellerFee { expected: u16, found: u16 },
    /// Update authority is not the expected one.
    WrongUpdateAuthority { expected: Pubkey, found: Pubkey },
    /// Metadata URI points to suspicious domain.
    SuspiciousUri(String),
    /// NFT minted recently (potential rug).
    RecentMint { slot: u64 },
    /// Collection address does not match.
    CollectionMismatch { expected: Pubkey, found: Pubkey },
}

impl std::fmt::Display for FakeIndicator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FakeIndicator::NoVerifiedCreator => write!(f, "No verified creator found"),
            FakeIndicator::CreatorMismatch { expected, found } => {
                write!(f, "Creator mismatch: expected {}, found {}", expected, found)
            }
            FakeIndicator::UnverifiedCollection => write!(f, "Collection not verified on-chain"),
            FakeIndicator::NameMismatch { expected_pattern, found } => {
                write!(f, "Name mismatch: expected pattern '{}', found '{}'", expected_pattern, found)
            }
            FakeIndicator::SymbolMismatch { expected, found } => {
                write!(f, "Symbol mismatch: expected '{}', found '{}'", expected, found)
            }
            FakeIndicator::UnusualSellerFee { expected, found } => {
                write!(f, "Unusual seller fee: expected {}bps, found {}bps", expected, found)
            }
            FakeIndicator::WrongUpdateAuthority { expected, found } => {
                write!(f, "Wrong update authority: expected {}, found {}", expected, found)
            }
            FakeIndicator::SuspiciousUri(uri) => write!(f, "Suspicious metadata URI: {}", uri),
            FakeIndicator::RecentMint { slot } => write!(f, "Recently minted at slot {}", slot),
            FakeIndicator::CollectionMismatch { expected, found } => {
                write!(f, "Collection mismatch: expected {}, found {}", expected, found)
            }
        }
    }
}

/// Risk level for NFT authenticity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NftRiskLevel {
    /// Verified authentic NFT.
    Verified,
    /// Likely authentic but some minor concerns.
    LikelyAuthentic,
    /// Unable to fully verify.
    Uncertain,
    /// Multiple red flags detected.
    Suspicious,
    /// Almost certainly fake.
    Fake,
}

impl std::fmt::Display for NftRiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NftRiskLevel::Verified => write!(f, "VERIFIED"),
            NftRiskLevel::LikelyAuthentic => write!(f, "LIKELY AUTHENTIC"),
            NftRiskLevel::Uncertain => write!(f, "UNCERTAIN"),
            NftRiskLevel::Suspicious => write!(f, "SUSPICIOUS"),
            NftRiskLevel::Fake => write!(f, "FAKE"),
        }
    }
}

/// NFT metadata information.
#[derive(Debug, Clone)]
pub struct NftMetadata {
    /// NFT name.
    pub name: String,
    /// NFT symbol.
    pub symbol: String,
    /// Metadata URI.
    pub uri: String,
    /// Seller fee basis points.
    pub seller_fee_basis_points: u16,
    /// Creators with their shares and verification status.
    pub creators: Vec<NftCreator>,
    /// Collection information if present.
    pub collection: Option<NftCollection>,
    /// Update authority.
    pub update_authority: Pubkey,
    /// Is the metadata mutable.
    pub is_mutable: bool,
    /// Primary sale happened.
    pub primary_sale_happened: bool,
}

/// NFT creator information.
#[derive(Debug, Clone)]
pub struct NftCreator {
    /// Creator address.
    pub address: Pubkey,
    /// Whether the creator has verified this NFT.
    pub verified: bool,
    /// Share percentage (0-100).
    pub share: u8,
}

/// NFT collection information.
#[derive(Debug, Clone)]
pub struct NftCollection {
    /// Collection mint address.
    pub key: Pubkey,
    /// Whether collection is verified.
    pub verified: bool,
}

/// Safety report for an NFT.
#[derive(Debug, Clone)]
pub struct NftSafetyReport {
    /// The NFT mint address analyzed.
    pub mint: Pubkey,
    /// Whether the NFT is considered authentic.
    pub authentic: bool,
    /// Risk level.
    pub risk_level: NftRiskLevel,
    /// Matched collection name if recognized.
    pub collection_name: Option<String>,
    /// Fake indicators found.
    pub fake_indicators: Vec<FakeIndicator>,
    /// Warnings that do not necessarily indicate fake.
    pub warnings: Vec<String>,
    /// Metadata if successfully retrieved.
    pub metadata: Option<NftMetadata>,
    /// Confidence score (0-100).
    pub confidence: u8,
}

impl NftSafetyReport {
    /// Create a new report for a mint.
    pub fn new(mint: Pubkey) -> Self {
        Self {
            mint,
            authentic: false,
            risk_level: NftRiskLevel::Uncertain,
            collection_name: None,
            fake_indicators: Vec::new(),
            warnings: Vec::new(),
            metadata: None,
            confidence: 0,
        }
    }

    /// Add a fake indicator.
    pub fn add_fake_indicator(&mut self, indicator: FakeIndicator) {
        self.fake_indicators.push(indicator);
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Determine if safe to purchase.
    pub fn is_safe_to_purchase(&self) -> bool {
        self.authentic && self.risk_level <= NftRiskLevel::LikelyAuthentic
    }
}

/// Collection verification configuration.
#[derive(Debug, Clone)]
pub struct CollectionConfig {
    /// Expected creators for this collection.
    pub creators: Vec<Pubkey>,
    /// Expected collection address.
    pub collection_address: Option<Pubkey>,
    /// Expected symbol.
    pub symbol: Option<String>,
    /// Expected seller fee basis points.
    pub seller_fee_bps: Option<u16>,
    /// Name pattern (prefix).
    pub name_pattern: Option<String>,
}

impl CollectionConfig {
    /// Create a new collection config.
    pub fn new() -> Self {
        Self {
            creators: Vec::new(),
            collection_address: None,
            symbol: None,
            seller_fee_bps: None,
            name_pattern: None,
        }
    }

    /// Add an expected creator.
    pub fn with_creator(mut self, creator: Pubkey) -> Self {
        self.creators.push(creator);
        self
    }

    /// Set expected collection address.
    pub fn with_collection(mut self, address: Pubkey) -> Self {
        self.collection_address = Some(address);
        self
    }

    /// Set expected symbol.
    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    /// Set expected seller fee.
    pub fn with_seller_fee(mut self, bps: u16) -> Self {
        self.seller_fee_bps = Some(bps);
        self
    }

    /// Set name pattern.
    pub fn with_name_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.name_pattern = Some(pattern.into());
        self
    }
}

impl Default for CollectionConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Suspicious URI domains that may indicate fakes.
const SUSPICIOUS_DOMAINS: &[&str] = &[
    "ipfs.io", // Not suspicious per se, but often used by fakes
    "gateway.pinata.cloud",
    "pastebin.com",
    "imgur.com",
    "discord.com",
    "discordapp.com",
];

/// NFT safety analyzer.
pub struct NftSafetyAnalyzer {
    rpc_client: RpcClient,
    known_collections: HashMap<String, Pubkey>,
    collection_configs: HashMap<Pubkey, CollectionConfig>,
    trusted_creators: HashSet<Pubkey>,
}

impl NftSafetyAnalyzer {
    /// Create a new NFT safety analyzer.
    pub fn new(rpc_url: &str) -> Self {
        let mut known_collections = HashMap::new();
        for (name, address) in KNOWN_COLLECTIONS {
            if let Ok(pubkey) = Pubkey::from_str(address) {
                known_collections.insert(name.to_string(), pubkey);
            }
        }

        Self {
            rpc_client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            known_collections,
            collection_configs: HashMap::new(),
            trusted_creators: HashSet::new(),
        }
    }

    /// Create from existing RpcClient.
    pub fn from_client(client: RpcClient) -> Self {
        let mut known_collections = HashMap::new();
        for (name, address) in KNOWN_COLLECTIONS {
            if let Ok(pubkey) = Pubkey::from_str(address) {
                known_collections.insert(name.to_string(), pubkey);
            }
        }

        Self {
            rpc_client: client,
            known_collections,
            collection_configs: HashMap::new(),
            trusted_creators: HashSet::new(),
        }
    }

    /// Add a trusted creator.
    pub fn add_trusted_creator(&mut self, creator: Pubkey) {
        self.trusted_creators.insert(creator);
    }

    /// Add a collection configuration for verification.
    pub fn add_collection_config(&mut self, collection: Pubkey, config: CollectionConfig) {
        self.collection_configs.insert(collection, config);
    }

    /// Perform full analysis on an NFT.
    pub fn analyze(&self, mint: &Pubkey) -> Result<NftSafetyReport> {
        let mut report = NftSafetyReport::new(*mint);

        // Get metadata PDA
        let metadata_pubkey = self.get_metadata_pda(mint);

        // Fetch metadata account
        let metadata_account = self.rpc_client.get_account(&metadata_pubkey).map_err(|e| {
            ToolkitError::custom(format!("Failed to fetch metadata: {}", e))
        })?;

        // Parse metadata (simplified - actual implementation would use borsh)
        let metadata = self.parse_metadata(&metadata_account.data)?;
        report.metadata = Some(metadata.clone());

        // Check for verified creator
        let has_verified_creator = metadata.creators.iter().any(|c| c.verified);
        if !has_verified_creator {
            report.add_fake_indicator(FakeIndicator::NoVerifiedCreator);
        }

        // Check collection verification
        if let Some(ref collection) = metadata.collection {
            if !collection.verified {
                report.add_fake_indicator(FakeIndicator::UnverifiedCollection);
            }

            // Check if matches known collection
            for (name, address) in &self.known_collections {
                if collection.key == *address {
                    report.collection_name = Some(name.clone());
                    if collection.verified {
                        report.confidence += 50;
                    }
                    break;
                }
            }

            // Verify against collection config if available
            if let Some(config) = self.collection_configs.get(&collection.key) {
                self.verify_against_config(&metadata, config, &mut report);
            }
        } else {
            report.add_warning("No collection information found");
        }

        // Check URI for suspicious domains
        for domain in SUSPICIOUS_DOMAINS {
            if metadata.uri.contains(domain) {
                report.add_warning(format!("Metadata URI uses {}", domain));
            }
        }

        // Check for trusted creators
        for creator in &metadata.creators {
            if creator.verified && self.trusted_creators.contains(&creator.address) {
                report.confidence += 30;
            }
        }

        // Calculate risk level and authenticity
        self.calculate_risk(&mut report);

        Ok(report)
    }

    /// Quick collection verification check.
    pub fn is_verified(&self, mint: &Pubkey) -> Result<bool> {
        let metadata_pubkey = self.get_metadata_pda(mint);

        let metadata_account = self.rpc_client.get_account(&metadata_pubkey).map_err(|e| {
            ToolkitError::custom(format!("Failed to fetch metadata: {}", e))
        })?;

        let metadata = self.parse_metadata(&metadata_account.data)?;

        // Check if collection is verified and recognized
        if let Some(collection) = metadata.collection {
            if collection.verified {
                for (_name, address) in &self.known_collections {
                    if collection.key == *address {
                        return Ok(true);
                    }
                }
            }
        }

        // Check for verified creator from trusted list
        for creator in &metadata.creators {
            if creator.verified && self.trusted_creators.contains(&creator.address) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Detect fake indicators only.
    pub fn detect_fake(&self, mint: &Pubkey) -> Result<Vec<FakeIndicator>> {
        let report = self.analyze(mint)?;
        Ok(report.fake_indicators)
    }

    /// Check if an NFT belongs to a specific collection.
    pub fn belongs_to_collection(&self, mint: &Pubkey, collection: &Pubkey) -> Result<bool> {
        let metadata_pubkey = self.get_metadata_pda(mint);

        let metadata_account = self.rpc_client.get_account(&metadata_pubkey).map_err(|e| {
            ToolkitError::custom(format!("Failed to fetch metadata: {}", e))
        })?;

        let metadata = self.parse_metadata(&metadata_account.data)?;

        if let Some(nft_collection) = metadata.collection {
            Ok(nft_collection.key == *collection && nft_collection.verified)
        } else {
            Ok(false)
        }
    }

    /// Get the metadata PDA for a mint.
    fn get_metadata_pda(&self, mint: &Pubkey) -> Pubkey {
        let metadata_program = Pubkey::from_str(METAPLEX_METADATA_PROGRAM)
            .expect("Invalid metadata program address");

        let seeds = &[
            b"metadata".as_ref(),
            metadata_program.as_ref(),
            mint.as_ref(),
        ];

        let (pda, _bump) = Pubkey::find_program_address(seeds, &metadata_program);
        pda
    }

    /// Parse metadata from account data (simplified).
    fn parse_metadata(&self, data: &[u8]) -> Result<NftMetadata> {
        // This is a simplified parser - actual implementation would use borsh
        // and properly parse the Metaplex metadata structure

        if data.len() < 100 {
            return Err(ToolkitError::custom(
                "Metadata account too small".to_string(),
            ));
        }

        // For now, return placeholder metadata
        // Real implementation would parse:
        // - Key (1 byte)
        // - Update authority (32 bytes)
        // - Mint (32 bytes)
        // - Name (string with length prefix)
        // - Symbol (string with length prefix)
        // - Uri (string with length prefix)
        // - Seller fee basis points (u16)
        // - Creators (option with vec)
        // - Collection (option)
        // - Uses (option)

        Ok(NftMetadata {
            name: String::new(),
            symbol: String::new(),
            uri: String::new(),
            seller_fee_basis_points: 0,
            creators: Vec::new(),
            collection: None,
            update_authority: Pubkey::default(),
            is_mutable: true,
            primary_sale_happened: false,
        })
    }

    /// Verify metadata against collection config.
    fn verify_against_config(
        &self,
        metadata: &NftMetadata,
        config: &CollectionConfig,
        report: &mut NftSafetyReport,
    ) {
        // Check symbol
        if let Some(ref expected_symbol) = config.symbol {
            if &metadata.symbol != expected_symbol {
                report.add_fake_indicator(FakeIndicator::SymbolMismatch {
                    expected: expected_symbol.clone(),
                    found: metadata.symbol.clone(),
                });
            }
        }

        // Check seller fee
        if let Some(expected_fee) = config.seller_fee_bps {
            if metadata.seller_fee_basis_points != expected_fee {
                report.add_fake_indicator(FakeIndicator::UnusualSellerFee {
                    expected: expected_fee,
                    found: metadata.seller_fee_basis_points,
                });
            }
        }

        // Check name pattern
        if let Some(ref pattern) = config.name_pattern {
            if !metadata.name.starts_with(pattern) {
                report.add_fake_indicator(FakeIndicator::NameMismatch {
                    expected_pattern: pattern.clone(),
                    found: metadata.name.clone(),
                });
            }
        }

        // Check creators
        if !config.creators.is_empty() {
            let verified_creators: Vec<_> = metadata
                .creators
                .iter()
                .filter(|c| c.verified)
                .map(|c| c.address)
                .collect();

            let has_expected_creator = config
                .creators
                .iter()
                .any(|expected| verified_creators.contains(expected));

            if !has_expected_creator && !verified_creators.is_empty() {
                report.add_fake_indicator(FakeIndicator::CreatorMismatch {
                    expected: config.creators[0],
                    found: verified_creators[0],
                });
            }
        }
    }

    /// Calculate risk level based on indicators.
    fn calculate_risk(&self, report: &mut NftSafetyReport) {
        let fake_count = report.fake_indicators.len();
        let warning_count = report.warnings.len();

        // Adjust confidence based on findings
        if fake_count > 0 {
            report.confidence = report.confidence.saturating_sub((fake_count * 20) as u8);
        }
        if warning_count > 0 {
            report.confidence = report.confidence.saturating_sub((warning_count * 5) as u8);
        }

        // Determine risk level
        report.risk_level = if fake_count == 0 && report.confidence >= 80 {
            NftRiskLevel::Verified
        } else if fake_count == 0 && report.confidence >= 50 {
            NftRiskLevel::LikelyAuthentic
        } else if fake_count <= 1 && report.confidence >= 30 {
            NftRiskLevel::Uncertain
        } else if fake_count <= 2 {
            NftRiskLevel::Suspicious
        } else {
            NftRiskLevel::Fake
        };

        // Set authenticity
        report.authentic = report.risk_level <= NftRiskLevel::LikelyAuthentic;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fake_indicator_display() {
        let indicator = FakeIndicator::NoVerifiedCreator;
        assert_eq!(indicator.to_string(), "No verified creator found");

        let indicator = FakeIndicator::UnverifiedCollection;
        assert_eq!(indicator.to_string(), "Collection not verified on-chain");
    }

    #[test]
    fn test_nft_risk_level_ordering() {
        assert!(NftRiskLevel::Verified < NftRiskLevel::LikelyAuthentic);
        assert!(NftRiskLevel::LikelyAuthentic < NftRiskLevel::Uncertain);
        assert!(NftRiskLevel::Uncertain < NftRiskLevel::Suspicious);
        assert!(NftRiskLevel::Suspicious < NftRiskLevel::Fake);
    }

    #[test]
    fn test_nft_risk_level_display() {
        assert_eq!(NftRiskLevel::Verified.to_string(), "VERIFIED");
        assert_eq!(NftRiskLevel::Fake.to_string(), "FAKE");
    }

    #[test]
    fn test_nft_safety_report_new() {
        let mint = Pubkey::new_unique();
        let report = NftSafetyReport::new(mint);

        assert_eq!(report.mint, mint);
        assert!(!report.authentic);
        assert_eq!(report.risk_level, NftRiskLevel::Uncertain);
        assert!(report.fake_indicators.is_empty());
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn test_nft_safety_report_add_indicator() {
        let mut report = NftSafetyReport::new(Pubkey::new_unique());
        report.add_fake_indicator(FakeIndicator::NoVerifiedCreator);
        report.add_fake_indicator(FakeIndicator::UnverifiedCollection);

        assert_eq!(report.fake_indicators.len(), 2);
    }

    #[test]
    fn test_nft_safety_report_add_warning() {
        let mut report = NftSafetyReport::new(Pubkey::new_unique());
        report.add_warning("Test warning");

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0], "Test warning");
    }

    #[test]
    fn test_nft_safety_report_safe_to_purchase() {
        let mut report = NftSafetyReport::new(Pubkey::new_unique());
        report.authentic = true;
        report.risk_level = NftRiskLevel::Verified;

        assert!(report.is_safe_to_purchase());

        report.risk_level = NftRiskLevel::Suspicious;
        assert!(!report.is_safe_to_purchase());
    }

    #[test]
    fn test_collection_config_builder() {
        let creator = Pubkey::new_unique();
        let collection = Pubkey::new_unique();

        let config = CollectionConfig::new()
            .with_creator(creator)
            .with_collection(collection)
            .with_symbol("DGOD")
            .with_seller_fee(500)
            .with_name_pattern("DeGod #");

        assert_eq!(config.creators.len(), 1);
        assert_eq!(config.creators[0], creator);
        assert_eq!(config.collection_address, Some(collection));
        assert_eq!(config.symbol, Some("DGOD".to_string()));
        assert_eq!(config.seller_fee_bps, Some(500));
        assert_eq!(config.name_pattern, Some("DeGod #".to_string()));
    }

    #[test]
    fn test_collection_config_default() {
        let config = CollectionConfig::default();
        assert!(config.creators.is_empty());
        assert!(config.collection_address.is_none());
        assert!(config.symbol.is_none());
    }

    #[test]
    fn test_known_collections_loaded() {
        let analyzer = NftSafetyAnalyzer::new("https://api.mainnet-beta.solana.com");
        assert!(!analyzer.known_collections.is_empty());
        assert!(analyzer.known_collections.contains_key("DeGods"));
        assert!(analyzer.known_collections.contains_key("y00ts"));
    }

    #[test]
    fn test_add_trusted_creator() {
        let mut analyzer = NftSafetyAnalyzer::new("https://api.mainnet-beta.solana.com");
        let creator = Pubkey::new_unique();

        analyzer.add_trusted_creator(creator);
        assert!(analyzer.trusted_creators.contains(&creator));
    }

    #[test]
    fn test_add_collection_config() {
        let mut analyzer = NftSafetyAnalyzer::new("https://api.mainnet-beta.solana.com");
        let collection = Pubkey::new_unique();
        let config = CollectionConfig::new().with_symbol("TEST");

        analyzer.add_collection_config(collection, config);
        assert!(analyzer.collection_configs.contains_key(&collection));
    }

    #[test]
    fn test_nft_metadata_struct() {
        let metadata = NftMetadata {
            name: "DeGod #1234".to_string(),
            symbol: "DGOD".to_string(),
            uri: "https://metadata.degods.com/1234.json".to_string(),
            seller_fee_basis_points: 500,
            creators: vec![NftCreator {
                address: Pubkey::new_unique(),
                verified: true,
                share: 100,
            }],
            collection: Some(NftCollection {
                key: Pubkey::new_unique(),
                verified: true,
            }),
            update_authority: Pubkey::new_unique(),
            is_mutable: false,
            primary_sale_happened: true,
        };

        assert_eq!(metadata.name, "DeGod #1234");
        assert_eq!(metadata.symbol, "DGOD");
        assert_eq!(metadata.seller_fee_basis_points, 500);
        assert!(metadata.creators[0].verified);
        assert!(metadata.collection.unwrap().verified);
    }

    #[test]
    fn test_nft_creator_struct() {
        let creator = NftCreator {
            address: Pubkey::new_unique(),
            verified: true,
            share: 50,
        };

        assert!(creator.verified);
        assert_eq!(creator.share, 50);
    }

    #[test]
    fn test_fake_indicator_creator_mismatch() {
        let expected = Pubkey::new_unique();
        let found = Pubkey::new_unique();

        let indicator = FakeIndicator::CreatorMismatch { expected, found };
        let display = indicator.to_string();

        assert!(display.contains("Creator mismatch"));
        assert!(display.contains(&expected.to_string()));
    }

    #[test]
    fn test_fake_indicator_symbol_mismatch() {
        let indicator = FakeIndicator::SymbolMismatch {
            expected: "DGOD".to_string(),
            found: "FAKE".to_string(),
        };

        let display = indicator.to_string();
        assert!(display.contains("Symbol mismatch"));
        assert!(display.contains("DGOD"));
        assert!(display.contains("FAKE"));
    }

    #[test]
    fn test_metaplex_program_constant() {
        let program = Pubkey::from_str(METAPLEX_METADATA_PROGRAM);
        assert!(program.is_ok());
    }

    #[test]
    fn test_known_collections_valid_addresses() {
        for (name, address) in KNOWN_COLLECTIONS {
            let result = Pubkey::from_str(address);
            assert!(result.is_ok(), "Invalid address for {}: {}", name, address);
        }
    }
}
