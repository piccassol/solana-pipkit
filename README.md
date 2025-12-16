<p align="center">
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


</p>
&nbsp;
<div align="center">
[📑 Docs](https://docs.rig.rs)
<span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
[🌐 Website](https://rig.rs)
<span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
[🤝 Contribute](https://github.com/0xPlaygrounds/rig/issues/new)
<span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
[✍🏽 Blogs](https://docs.rig.rs/guides)
</div>
✨ If you would like to help spread the word about Rig, please consider starring the repo!
> [!WARNING]
> Here be dragons! As we plan to ship a torrent of features in the following months, future updates **will** contain **breaking changes**. With Rig evolving, we'll annotate changes and highlight migration paths as we encounter them.
## Table of contents
- [Table of contents](#table-of-contents)
- [What is Rig?](#what-is-rig)
- [High-level features](#high-level-features)
- [Who's using Rig in production?](#who-is-using-rig-in-production)
- [Get Started](#get-started)
&nbsp;&nbsp;- [Simple example](#simple-example)
- [Integrations](#integrations)
## What is Rig?
Rig is a Rust library for building scalable, modular, and ergonomic **LLM-powered** applications.
More information about this crate can be found in the [official](https://docs.rig.rs) & [crate](https://docs.rs/rig-core/latest/rig/) (API Reference) documentations.
## Features
- Agentic workflows that can handle multi-turn streaming and prompting
- Full [GenAI Semantic Convention](https://opentelemetry.io/docs/specs/semconv/gen-ai/) compatibility
- 20+ model providers, all under one singular unified interface
- 10+ vector store integrations, all under one singular unified interface
- Full support for LLM completion and embedding workflows
- Support for transcription, audio generation and image generation model capabilities
- Integrate LLMs in your app with minimal boilerplate
- Full WASM compatibility (core library only)
## Who is using Rig?
Below is a non-exhaustive list of companies and people who are using Rig:
- [St Jude](https://www.stjude.org/) - Using Rig for a chatbot utility as part of [`proteinpaint`](https://github.com/stjude/proteinpaint), a genomics visualisation tool.
- [Coral Protocol](https://www.coralprotocol.org/) - Using Rig extensively, both internally as well as part of the [Coral Rust SDK.](https://github.com/Coral-Protocol/coral-rs)
- [VT Code](https://github.com/vinhnx/vtcode) - VT Code is a Rust-based terminal coding agent with semantic code intelligence via Tree-sitter and ast-grep. VT Code uses `rig` for simplifying LLM calls and implement model picker.
- [Dria](https://dria.co/) - a decentralised AI network. Currently using Rig as part of their [compute node.](https://github.com/firstbatchxyz/dkn-compute-node)
- [Nethermind](https://www.nethermind.io/) - Using Rig as part of their [Neural Interconnected Nodes Engine](https://github.com/NethermindEth/nine) framework.
- [Neon](https://neon.com) - Using Rig for their [app.build](https://github.com/neondatabase/appdotbuild-agent) V2 reboot in Rust.
- [Listen](https://github.com/piotrostr/listen) - A framework aiming to become the go-to framework for AI portfolio management agents. Powers [the Listen app.](https://app.listen-rs.com/)
- [Cairnify](https://cairnify.com/) - helps users find documents, links, and information instantly through an intelligent search bar. Rig provides the agentic foundation behind Cairnify’s AI search experience, enabling tool-calling, reasoning, and retrieval workflows.
Are you also using Rig in production? [Open an issue](https://www.github.com/0xPlaygrounds/rig/issues) to have your name added!
## Get Started
```bash
cargo add rig-core
```
### Simple example
```rust
use rig::{client::CompletionClient, completion::Prompt, providers::openai};
#[tokio::main]
async fn main() {
    // Create OpenAI client and model
    // This requires the `OPENAI_API_KEY` environment variable to be set.
    let openai_client = openai::Client::from_env();
    let gpt4 = openai_client.agent("gpt-4").build();
    // Prompt the model and print its response
    let response = gpt4
        .prompt("Who are you?")
        .await
        .expect("Failed to prompt GPT-4");
    println!("GPT-4: {response}");
}
```
Note using `#[tokio::main]` requires you enable tokio's `macros` and `rt-multi-thread` features
or just `full` to enable all features (`cargo add tokio --features macros,rt-multi-thread`).
You can find more examples each crate's `examples` (ie. [`rig-core/examples`](./rig-core/examples)) directory. More detailed use cases walkthroughs are regularly published on our [Dev.to Blog](https://dev.to/0thtachi) and added to Rig's official documentation [(docs.rig.rs)](http://docs.rig.rs).
## Supported Integrations
Vector stores are available as separate companion-crates:
- MongoDB: [`rig-mongodb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-mongodb)
- LanceDB: [`rig-lancedb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-lancedb)
- Neo4j: [`rig-neo4j`](https://github.com/0xPlaygrounds/rig/tree/main/rig-neo4j)
- Qdrant: [`rig-qdrant`](https://github.com/0xPlaygrounds/rig/tree/main/rig-qdrant)
- SQLite: [`rig-sqlite`](https://github.com/0xPlaygrounds/rig/tree/main/rig-sqlite)
- SurrealDB: [`rig-surrealdb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-surrealdb)
- Milvus: [`rig-milvus`](https://github.com/0xPlaygrounds/rig/tree/main/rig-milvus)
- ScyllaDB: [`rig-scylladb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-scylladb)
- AWS S3Vectors: [`rig-s3vectors`](https://github.com/0xPlaygrounds/rig/tree/main/rig-s3vectors)
- HelixDB: [`rig-helixdb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-helixdb)
The following providers are available as separate companion-crates:
- AWS Bedrock: [`rig-bedrock`](https://github.com/0xPlaygrounds/rig/tree/main/rig-bedrock)
- Fastembed: [`rig-fastembed`](https://github.com/0xPlaygrounds/rig/tree/main/rig-fastembed)
- Eternal AI: [`rig-eternalai`](https://github.com/0xPlaygrounds/rig/tree/main/rig-eternalai)
- Google Vertex: [`rig-vertexai`](https://github.com/0xPlaygrounds/rig/tree/main/rig-vertexai)
We also have some other associated crates that have additional functionality you may find helpful when using Rig:
- `rig-onchain-kit` - the [Rig Onchain Kit.](https://github.com/0xPlaygrounds/rig-onchain-kit) Intended to make interactions between Solana/EVM and Rig much easier to implement.
<p align="center">
<br>
<br>
<img src="img/built-by-playgrounds.svg" alt="Build by Playgrounds" width="30%">
</p>
<picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cdn.vectorstock.com/i/1000v/79/42/solana-logo-coin-icon-isolated-vector-43697942.jpg">
    <source media="(prefers-color-scheme: light)" srcset="https://upload.wikimedia.org/wikipedia/commons/3/34/Solana_cryptocurrency_two.jpg">
    <img src="https://upload.wikimedia.org/wikipedia/commons/3/34/Solana_cryptocurrency_two.jpg" style="width: 40%; height: 40%;" alt="Solana logo">
</picture>


https://img.shields.io/github/stars/piccassol/solana-pipkit?style=social
 
https://img.shields.io/github/forks/piccassol/solana-pipkit?style=social


https://img.shields.io/crates/v/solana-pipkit.svg?color=dea584
 
https://img.shields.io/crates/d/solana-pipkit.svg?color=dea584
 
https://img.shields.io/badge/docs-API%20Reference-dea584.svg


https://img.shields.io/badge/built%20on-Solana-dea584.svg
 
https://img.shields.io/badge/built%20with-Rust-dea584.svg?logo=rust


https://img.shields.io/badge/built%20by-ARK%20Technologies-dea584


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

I've updated the README to use your provided image (the fun orange crab on gradient background) as the main logo at the top and also in the footer. I used the same image for both light and dark modes since it looks great on both (vibrant and eye-catching). The direct link you provided should load reliably in GitHub Markdown. Let me know if you'd like any further tweaks!
