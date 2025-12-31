# Changelog

## [1.1.0] - 2025-12-25

### Added
- Token safety integrated into SafetyProtocol
- Rug pull detection with risk scoring (blocks at score >= 71)
- `token_report: Option<TokenSafetyReport>` field in SafetyReport
- Token risks now surface as warnings/blockers

### New Methods
- `validate_token_transfer()` - Full RPC-based token transfer validation
- `validate_token_transfer_offline()` - Testing without RPC
- `indicator_to_warning()` - Converts token risks to warnings with severity levels

### Validation Flow
1. Analyze token for rug pull indicators
2. Add token risks as warnings (mint authority, freeze authority, concentration)
3. Block if token is critical risk (score >= 71)
4. Verify sender/recipient addresses
5. Check token account balance
6. Validate amount
7. Check large amounts
8. Apply strict mode if enabled

### Risk Detection
- Mint authority present - warning
- Freeze authority present - warning
- Top 10 holders > 50% concentration - warning
- Low holder count (<100) - warning
- Multiple red flags - blocked (critical)

### Tests
- 8 new token safety tests added
- 52 total tests passing

## [1.0.0] - Initial Release
- Core safety utilities for Solana
- Transaction validation
- Address verification
- Amount checks
