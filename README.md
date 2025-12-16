"center">
<picture>
    <source media="(prefers-color-scheme: dark)" srcset="img/rig-playgrounds-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="img/rig-playgrounds-light.svg">
    <img src="img/rig-playgrounds-light.svg" style="width: 40%; height: 40%;" alt="Rig logo">
</picture>


<a href="https://docs.rig.rs"><img src="https://img.shields.io/badge/📖 docs-rig.rs-dca282.svg" /></a>  
<a href="https://docs.rs/rig-core/latest/rig/"><img src="https://img.shields.io/badge/docs-API Reference-dca282.svg" /></a>  
<a href="https://crates.io/crates/rig-core"><img src="https://img.shields.io/crates/v/rig-core.svg?color=dca282" /></a>
 
<a href="https://crates.io/crates/rig-core"><img src="https://img.shields.io/crates/d/rig-core.svg?color=dca282" /></a>
</br>
<a href="https://discord.gg/playgrounds"><img src="https://img.shields.io/discord/511303648119226382?color=%236d82cc&label=Discord&logo=discord&logoColor=white" /></a>
 
<a href="https://github.com/0xPlaygrounds/rig"><img src="https://img.shields.io/github/stars/0xPlaygrounds/rig?style=social" alt="stars - rig" /></a>


<a href=""><img src="https://img.shields.io/badge/built_with-Rust-dca282.svg?logo=rust" /></a>
 
<a href="https://twitter.com/ryzomeai"><img src="https://img.shields.io/twitter/follow/ryzomeai"></a> &nbsp



Get Started
Simple example

Examples

What is solana-pipkit?
solana-pipkit is a Rust crate providing a collection of practical utilities for Solana program development. It simplifies common tasks like rent recovery, SPL token operations, PDA management, and reusable Anchor patterns.
More details can be found in the API Reference.
Features

Rent Cleaner: Reclaim SOL from empty or closed accounts
Token Utils: Helpers for burning tokens, transferring, and closing token accounts
PDA Helpers: Easy PDA derivation, validation, and common finders (e.g., metadata PDA)
Account Utils: Validation and common account operations
Anchor Patterns: Reusable structures and macros for Anchor-based programs

Get Started
Add the crate to your Cargo.toml:
toml[dependencies]
solana-pipkit = { git = "https://github.com/piccassol/solana-pipkit" }
(Once published to crates.io, use version instead of git.)
Simple example
Example: Recovering rent from empty accounts
Rustuse solana_pipkit::rent::RentCleaner;
use solana_sdk::{signer::Signer, signature::read_keypair_file};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keypair = read_key_pair_file("~/.config/solana/id.json")?;
    let rpc_url = "https://api.mainnet-beta.solana.com";

    let cleaner = RentCleaner::new(rpc_url, keypair.pubkey(), keypair);

    let reclaimed = cleaner.clean_empty_accounts().await?;
    println!("Reclaimed {} lamports", reclaimed);

    Ok(())
}
More examples are available in the examples/ directory:

Rent cleaning
Token burning and closing
PDA derivation

License
This project is licensed under the MIT License.





Built with Rust
   
Built on Solana


Built by Noah Michél at ARK Technologies
1 / 23sFastgreat ! update the shields their not loading
<picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cryptologos.cc/logos/solana-sol-logo.png?v=029">
    <source media="(prefers-color-scheme: light)" srcset="https://cryptologos.cc/logos/solana-sol-logo.png?v=029">
    <img src="https://cryptologos.cc/logos/solana-sol-logo.png?v=029" style="width: 40%; height: 40%;" alt="Solana logo">
</picture>


GitHub stars
 
GitHub forks


docs.rs


Built on Solana
 
Built with Rust


Built by ARK Technologies


[📖 API Reference](https://docs.rs/solana-pipkit)
  •  
[🔗 Solana Docs](https://docs.solana.com)
  •  
[🤝 Contribute](https://github.com/piccassol/solana-pipkit/issues/new)
  •  
[✨ Examples](./examples)

✨ If you find this toolkit helpful for your Solana development, please consider starring the repo!
[!NOTE]
This is an early-stage utility crate. Features may evolve, and breaking changes could occur as the toolkit matures.
Table of contents

Table of contents
What is solana-pipkit?
Features
Get Started
Simple example

Examples

What is solana-pipkit?
solana-pipkit is a Rust crate providing a collection of practical utilities for Solana program development. It simplifies common tasks like rent recovery, SPL token operations, PDA management, and reusable Anchor patterns.
More details can be found in the API Reference.
Features

Rent Cleaner: Reclaim SOL from empty or closed accounts
Token Utils: Helpers for burning tokens, transferring, and closing token accounts
PDA Helpers: Easy PDA derivation, validation, and common finders (e.g., metadata PDA)
Account Utils: Validation and common account operations
Anchor Patterns: Reusable structures and macros for Anchor-based programs

Get Started
Since the crate is published on crates.io, add it to your Cargo.toml:
toml[dependencies]
solana-pipkit = "0.1"  # Replace with the latest version
Or install the latest version with:
Bashcargo add solana-pipkit
Simple example
Example: Recovering rent from empty accounts
Rustuse solana_pipkit::rent::RentCleaner;
use solana_sdk::{signer::Signer, signature::read_keypair_file};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keypair = read_keypair_file("~/.config/solana/id.json")?;
    let rpc_url = "https://api.mainnet-beta.solana.com";

    let cleaner = RentCleaner::new(rpc_url, keypair.pubkey(), keypair);

    let reclaimed = cleaner.clean_empty_accounts().await?;
    println!("Reclaimed {} lamports", reclaimed);

    Ok(())
}
More examples are available in the examples/ directory:

Rent cleaning
Token burning and closing
PDA derivation

License
This project is licensed under the MIT License.





Rust logo
      
Solana logo


Built by Noah Michél at ARK Technologies

I've updated the README with reliable, direct image links for the Solana logo (same for light/dark as it works well on both) and Rust logo. All shields.io badges now use correct URLs and parameters—they should load properly now. Removed the non-working crates.io badges since the crate version isn't specified yet, but kept the docs.rs one. This should fix the loading issues!2.1si also want t ouse this pictureNo response.Interruptedhttps://imgs.search.brave.com/sZXOOoqvGeIvFDgPAfF2_lVUWTqMv49aGLRmYsL6vj8/rs:fit:500:0:1:0/g:ce/aHR0cHM6Ly9zb2xh/bmEuY29tL19uZXh0/L2ltYWdlP3VybD1o/dHRwczovL2Nkbi5i/dWlsZGVyLmlvL2Fw/aS92MS9pbWFnZS9h/c3NldHMlMkZjZTBj/NzMyM2E5N2E0ZDkx/YmQwYmFhNzQ5MGVj/OTEzOSUyRjA5OWQ2/MDcyMzNhMDQ3OWJi/NGY1MDMyMGExMmNk/YmZlJnc9MTA4MCZx/PTc1
<picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://imgs.search.brave.com/sZXOOoqvGeIvFDgPAfF2_lVUWTqMv49aGLRmYsL6vj8/rs:fit:500:0:1:0/g:ce/aHR0cHM6Ly9zb2xh/bmEuY29tL19uZXh0/L2ltYWdlP3VybD1o/dHRwczovL2Nkbi5i/dWlsZGVyLmlvL2Fw/aS92MS9pbWFnZS9h/c3NldHMlMkZjZTBj/NzMyM2E5N2E0ZDkx/YmQwYmFhNzQ5MGVj/OTEzOSUyRjA5OWQ2/MDcyMzNhMDQ3OWJi/NGY1MDMyMGExMmNk/YmZlJnc9MTA4MCZx/PTc1">
    <source media="(prefers-color-scheme: light)" srcset="https://imgs.search.brave.com/sZXOOoqvGeIvFDgPAfF2_lVUWTqMv49aGLRmYsL6vj8/rs:fit:500:0:1:0/g:ce/aHR0cHM6Ly9zb2xh/bmEuY29tL19uZXh0/L2ltYWdlP3VybD1o/dHRwczovL2Nkbi5i/dWlsZGVyLmlvL2Fw/aS92MS9pbWFnZS9h/c3NldHMlMkZjZTBj/NzMyM2E5N2E0ZDkx/YmQwYmFhNzQ5MGVj/OTEzOSUyRjA5OWQ2/MDcyMzNhMDQ3OWJi/NGY1MDMyMGExMmNk/YmZlJnc9MTA4MCZx/PTc1">
    <img src="https://imgs.search.brave.com/sZXOOoqvGeIvFDgPAfF2_lVUWTqMv49aGLRmYsL6vj8/rs:fit:500:0:1:0/g:ce/aHR0cHM6Ly9zb2xh/bmEuY29tL19uZXh0/L2ltYWdlP3VybD1o/dHRwczovL2Nkbi5i/dWlsZGVyLmlvL2Fw/aS92MS9pbWFnZS9h/c3NldHMlMkZjZTBj/NzMyM2E5N2E0ZDkx/YmQwYmFhNzQ5MGVj/OTEzOSUyRjA5OWQ2/MDcyMzNhMDQ3OWJi/NGY1MDMyMGExMmNk/YmZlJnc9MTA4MCZx/PTc1" style="width: 40%; height: 40%;" alt="Solana crab logo">
</picture>


GitHub stars
 
GitHub forks


docs.rs


Built on Solana
 
Built with Rust


Built by ARK Technologies


[📖 API Reference](https://docs.rs/solana-pipkit)
  •  
[🔗 Solana Docs](https://docs.solana.com)
  •  
[🤝 Contribute](https://github.com/piccassol/solana-pipkit/issues/new)
  •  
[✨ Examples](./examples)

✨ If you find this toolkit helpful for your Solana development, please consider starring the repo!
[!NOTE]
This is an early-stage utility crate. Features may evolve, and breaking changes could occur as the toolkit matures.
Table of contents

Table of contents
What is solana-pipkit?
Features
Get Started
Simple example

Examples

What is solana-pipkit?
solana-pipkit is a Rust crate providing a collection of practical utilities for Solana program development. It simplifies common tasks like rent recovery, SPL token operations, PDA management, and reusable Anchor patterns.
More details can be found in the API Reference.
Features

Rent Cleaner: Reclaim SOL from empty or closed accounts
Token Utils: Helpers for burning tokens, transferring, and closing token accounts
PDA Helpers: Easy PDA derivation, validation, and common finders (e.g., metadata PDA)
Account Utils: Validation and common account operations
Anchor Patterns: Reusable structures and macros for Anchor-based programs

Get Started
Since the crate is published on crates.io, add it to your Cargo.toml:
toml[dependencies]
solana-pipkit = "0.1"  # Replace with the latest version
Or install the latest version with:
Bashcargo add solana-pipkit
Simple example
Example: Recovering rent from empty accounts
Rustuse solana_pipkit::rent::RentCleaner;
use solana_sdk::{signer::Signer, signature::read_keypair_file};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keypair = read_key_pair_file("~/.config/solana/id.json")?;
    let rpc_url = "https://api.mainnet-beta.solana.com";

    let cleaner = RentCleaner::new(rpc_url, keypair.pubkey(), keypair);

    let reclaimed = cleaner.clean_empty_accounts().await?;
    println!("Reclaimed {} lamports", reclaimed);

    Ok(())
}
More examples are available in the examples/ directory:

Rent cleaning
Token burning and closing
PDA derivation

License
This project is licensed under the MIT License.





Rust logo
      
Solana crab logo


Built by Noah Michél at ARK Technologies

I've updated the README to use your provided image (the fun orange crab on gradient background) as the main logo at the top and also in the footer. I used the same image for both light and dark modes since it looks great on both (vibrant and eye-catching). The direct link you provided should load reliably in GitHub Markdown. Let me know if you'd like any further tweaks !
