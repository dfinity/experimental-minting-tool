#[macro_use]
extern crate candid;
#[macro_use]
extern crate thiserror;

use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    path::{Path, PathBuf},
    process,
};

use anyhow::{bail, Context, Result};
use candid::Principal;
use cid::Cid;
use clap::Parser;
use dialoguer::Confirm;
use ic_agent::{
    agent::http_transport::ReqwestHttpReplicaV2Transport, identity::BasicIdentity, Agent,
    AgentError,
};
use sha2::{Digest, Sha256};
use types::*;
use uriparse::URI;

mod types;

#[tokio::main]
async fn main() {
    if let Err(e) = rmain().await {
        eprintln!("{}", e);
        process::exit(1);
    }
}

async fn rmain() -> Result<()> {
    let mint = Args::parse();
    if mint.file.is_none()
        && !mint.yes
        && !Confirm::new()
            .with_prompt("Are you sure you don't want to specify a file? No content will be uploaded, only metadata!")
            .interact()?
    {
        println!("Aborted upload");
        return Ok(())
    }
    let canister = mint.canister;
    let owner = mint.owner;
    let agent = get_agent(
        &mint.network,
        mint.fetch_root_key || mint.network == "local",
    )
    .await?;
    let res = agent
        .query(&canister, "supportedInterfacesDip721")
        .with_arg(Encode!()?)
        .call()
        .await;
    let res = if let Err(AgentError::ReplicaError { reject_code: 3, .. }) = &res {
        res.context(format!(
            "canister {canister} does not appear to be a DIP-721 NFT canister"
        ))?
    } else {
        res?
    };
    let interfaces = Decode!(&res, Vec<InterfaceId>)?;
    if !interfaces.contains(&InterfaceId::Mint) {
        bail!("canister {canister} does not support minting");
    }
    let mut metadata = HashMap::new();
    use MetadataVal::*;
    if let Some(ipfs_location) = mint.ipfs_location {
        metadata.insert("locationType", Nat8Content(1));
        let cid: Cid = ipfs_location.parse()?;
        metadata.insert("location", BlobContent(cid.to_bytes()));
    } else if let Some(asset_canister) = mint.asset_canister {
        metadata.insert("locationType", Nat8Content(2));
        metadata.insert("location", TextContent(format!("{asset_canister}")));
    } else if let Some(uri) = mint.uri {
        URI::try_from(&*uri)?;
        metadata.insert("locationType", Nat8Content(3));
        metadata.insert("location", TextContent(uri));
    } else {
        metadata.insert("locationType", Nat8Content(4));
    }
    if let Some(sha2) = mint.sha2 {
        let hex = hex::decode(sha2)?;
        metadata.insert("contentHash", BlobContent(hex));
    }
    let (data, content_type) = if let Some(file) = mint.file {
        let data = fs::read(&file)?;
        if mint.sha2_auto {
            metadata.insert(
                "contentHash",
                BlobContent(Vec::from_iter(Sha256::digest(&data))),
            );
        }
        let content_type = mint
            .mime_type
            .or_else(|| mime_guess::from_path(&file).first().map(|m| format!("{m}")));
        (data, content_type)
    } else {
        (vec![], mint.mime_type)
    };
    let content_type = content_type.unwrap_or_else(|| String::from("application/octet-stream"));
    metadata.insert("contentType", TextContent(content_type));
    let metadata = MetadataPart {
        purpose: MetadataPurpose::Rendered,
        data: &data,
        key_val_data: metadata,
    };
    let res = agent
        .update(&mint.canister, "mintDip721")
        .with_arg(Encode!(&owner, &[metadata], &data)?)
        .call_and_wait()
        .await;
    let res = if let Err(AgentError::ReplicaError { reject_code: 3, .. }) = &res {
        res.context(format!("canister {canister} does not support minting"))?
    } else {
        res?
    };
    let MintReceipt { token_id, id } = Decode!(&res, Result<MintReceipt, MintError>)??;
    println!("Successfully minted token {token_id} to {owner} (transaction id {id})");
    Ok(())
}

/// A tool for minting DIP-721 NFTs.
///
/// Mints a new NFT with the provided content. You can use an IPFS CID, the
/// principal of an asset canister, or a web URI as the source, although this
/// is not required. A file path must also be supplied if you want the content
/// of the file to be uploaded to the NFT canister, rather than just the
/// metadata. A SHA256 hash can also be supplied, and is required if the source
/// is a URI, but can be calculated for you via the `--sha2-auto` flag.
///
/// DFINITY's dip721-nft-container canister supports the minting operation, but
/// not all canisters do. Additionally, each canister differs in who is
/// authorized to mint; usually only the original canister creator is. That may
/// mean your wallet, rather than your DFX principal, depending on how the
/// canister was initialized. Either of these things can cause an error.
#[derive(Parser)]
struct Args {
    /// The network the canister is running on. Can be 'ic', 'local', or a URL.
    network: String,
    /// The DIP-721 compliant NFT container.
    canister: Principal,
    /// The owner of the new NFT.
    #[clap(long)]
    owner: Principal,
    /// The CID of the file on IPFS.
    #[clap(long, conflicts_with_all(&["asset-canister", "uri"]))]
    ipfs_location: Option<String>,
    /// The principal of the file's asset canister on the IC.
    #[clap(long, conflicts_with_all(&["ipfs-location", "uri"]))]
    asset_canister: Option<Principal>,
    /// The URI of the file on the internet.
    #[clap(long, conflicts_with_all(&["ipfs-location", "asset-canister"]), requires("hash"))]
    uri: Option<String>,
    /// The path to the file. Required if you want the file contents sent to
    /// the smart contract.
    #[clap(long, required_unless_present_any(&["asset-canister", "uri", "ipfs-location"]))]
    file: Option<PathBuf>,
    /// The SHA-256 hash of the file. SHA2 is required if `--uri` is specified
    #[clap(long, group("hash"))]
    sha2: Option<String>,
    /// Calculates the SHA-256 hash of the file and includes it.
    #[clap(long, conflicts_with("sha2"), requires("file"), group("hash"))]
    sha2_auto: bool,
    /// The MIME type of the file. Can be inferred if `--file` is specified,
    /// required otherwise.
    #[clap(long, required_unless_present("file"))]
    mime_type: Option<String>,
    /// Skips confirmation for a minted NFT with no `--file`.
    #[clap(short)]
    yes: bool,
    /// Fetches the root key for the network. Auto-set for `--network local`. Do not use this with real data or on the real IC.
    #[clap(long)]
    fetch_root_key: bool,
}

#[derive(Deserialize)]
struct DefaultIdentity {
    default: String,
}

async fn get_agent(network: &str, fetch_root_key: bool) -> Result<Agent> {
    let url = match network {
        "local" => "http://localhost:4943",
        "ic" => "https://ic0.app",
        url => url,
    };
    let user_home = env::var_os("HOME").unwrap();
    let file = File::open(Path::new(&user_home).join(".config/dfx/identity.json"))
        .context("Configure an identity in `dfx` or provide an --identity flag")?;
    let default: DefaultIdentity = serde_json::from_reader(file)?;
    let pemfile = PathBuf::from_iter([
        &*user_home,
        ".config/dfx/identity/".as_ref(),
        default.default.as_ref(),
        "identity.pem".as_ref(),
    ]);
    let identity = BasicIdentity::from_pem_file(pemfile)?;
    let agent = Agent::builder()
        .with_transport(ReqwestHttpReplicaV2Transport::create(url)?)
        .with_identity(identity)
        .build()?;
    if fetch_root_key {
        agent.fetch_root_key().await?;
    }
    Ok(agent)
}
