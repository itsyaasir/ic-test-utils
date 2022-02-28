//! Create and manage a ledger canister
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ic_agent::ic_types::Principal;
use ic_agent::identity::Identity;
use ledger_canister::{AccountIdentifier, LedgerCanisterInitPayload};

use super::{create_canister, get_identity, Agent};
use crate::Result;

pub use ledger_canister::Tokens;

/// The ledger byte code
pub const LEDGER_WASM: &[u8] = include_bytes!("ledger.wasm");

/// Create a ledger canister:
///
/// ```
/// use ic_test_utils::ledger::{new_ledger_canister, Tokens};
///
/// # fn get_agent() -> ic_test_utils::Agent { panic!() }
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// # let agent = get_agent();
/// let ledger_canister = new_ledger_canister("bob")
///     .with_account("max", Tokens::new(100_000_000, 0)?)?
///     .with_account("alex", Tokens::from_tokens(50_000_000)?)?
///     .build(&agent, "max", 1000_000_000).await;
///
/// # Ok(())
/// # }
/// ```
pub struct LedgerBuilder {
    owner: PathBuf,
    accounts: HashMap<AccountIdentifier, Tokens>,
}

impl LedgerBuilder {
    fn new(owner: impl AsRef<Path>) -> Self {
        Self {
            owner: owner.as_ref().to_owned(),
            accounts: HashMap::new(),
        }
    }

    /// Finalise the ledger canister and get the principal
    pub async fn build(
        &mut self,
        agent: &Agent,
        account_name: impl AsRef<str>,
        cycles: impl Into<Option<u64>>,
    ) -> Result<Principal> {
        let owner = AccountIdentifier::new(get_identity(&self.owner)?.sender()?.into(), None);

        let initial_values = std::mem::take(&mut self.accounts);

        let arg = LedgerCanisterInitPayload {
            minting_account: owner,
            initial_values,
            max_message_size_bytes: None,
            transaction_window: None,
            archive_options: None,
            send_whitelist: HashSet::new(),
        };

        let cycles = cycles.into().unwrap_or(1_000_000_000_000);
        let principal =
            create_canister(agent, account_name, LEDGER_WASM.to_vec(), (arg,), cycles).await?;

        Ok(principal)
    }

    /// Add an account to the ledger canister.
    /// This is now the owner account. The owner account is set when calling [`new_ledger_canister`].
    pub fn with_account(
        &mut self,
        account_name: impl AsRef<Path>,
        tokens: Tokens,
    ) -> Result<&mut Self> {
        let ident = super::get_identity(account_name)?;
        let principal = ident.sender()?;
        let account = AccountIdentifier::new(principal.into(), None);
        self.accounts.insert(account, tokens);
        Ok(self)
    }
}

/// Create a new ledger canister through the [`LedgerBuilder`]
pub fn new_ledger_canister(owner: impl AsRef<Path>) -> LedgerBuilder {
    LedgerBuilder::new(owner)
}
