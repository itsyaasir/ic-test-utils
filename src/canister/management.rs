use ic_cdk::export::candid::{Principal, CandidType, Decode, Deserialize, Encode, encode_args, utils::ArgumentEncoder};

use super::wallet::Wallet;
use super::{Agent, Canister};
use crate::Result;

/// The install mode of the canister to install. If a canister is already installed,
/// using [InstallMode::Install] will be an error. [InstallMode::Reinstall] overwrites
/// the module, and [InstallMode::Upgrade] performs an Upgrade step.
#[derive(Copy, Clone, CandidType, Deserialize, Eq, PartialEq)]
pub enum InstallMode {
    /// Install wasm
    #[serde(rename = "install")]
    Install,
    /// Reinstall wasm
    #[serde(rename = "reinstall")]
    Reinstall,
    /// Upgrade wasm
    #[serde(rename = "upgrade")]
    Upgrade,
}

/// Installation arguments for [`Canister::install_code`].
#[derive(CandidType, Deserialize)]
pub struct CanisterInstall {
    /// [`InstallMode`]
    pub mode: InstallMode,
    /// Canister id
    pub canister_id: Principal,
    #[serde(with = "serde_bytes")]
    /// Wasm module as raw bytes
    pub wasm_module: Vec<u8>,
    #[serde(with = "serde_bytes")]
    /// Any aditional arguments to be passed along
    pub arg: Vec<u8>,
}

#[derive(CandidType, Deserialize)]
struct In {
    canister_id: Principal,
}

// -----------------------------------------------------------------------------
//     - Management container -
// -----------------------------------------------------------------------------

/// The management canister is used to install code, upgrade, stop and delete
/// canisters.
///
/// ```
/// # use ic_agent::Agent;
/// use test_utils::canister::Canister;
/// # async fn run(agent: &Agent, principal: ic_cdk::export::candid::Principal) {
/// let wallet = Canister::new_wallet(agent, "account_name", None).unwrap();
/// let management = Canister::new_management(agent);
/// management.stop_canister(&wallet, principal).await;
/// # }
/// ```
pub struct Management;

impl<'agent> Canister<'agent, Management> {
    /// Create a new management canister
    pub fn new_management(agent: &'agent Agent) -> Self {
        let id = Principal::management_canister();
        Self::new(id, agent)
    }

    // Make a call through the wallet so cycles 
    // can be spent
    async fn through_wallet_call<Out>(
        &self,
        wallet: &Canister<'_, Wallet>,
        fn_name: &str,
        cycles: u64,
        arg: Option<Vec<u8>>,
    ) -> Result<Out> 
        where 
            Out: CandidType + for<'de> Deserialize<'de>
    {
        let call = self.update_raw(fn_name, arg)?;
        let result = wallet.call_forward(call, cycles).await?;
        let out = Decode!(&result, Out)?;
        Ok(out)
    }

    /// Install code in an existing canister.
    /// To create a canister first use [`Canister::create_canister`]
    pub async fn install_code<'wallet_agent, T: ArgumentEncoder + std::fmt::Debug>(
        &self,
        wallet: &Canister<'wallet_agent, Wallet>,
        canister_id: Principal,
        bytecode: Vec<u8>,
        arg: T,
    ) -> Result<()> {
        let install_args = CanisterInstall {
            mode: InstallMode::Install,
            canister_id,
            wasm_module: bytecode,
            arg: encode_args(arg)?,
        };

        let args = Encode!(&install_args)?;
        self.through_wallet_call::<()>(wallet, "install_code", 0, Some(args)).await?;

        Ok(())
    }

    /// Upgrade an existing canister.
    /// Upgrading a canister for a test is possible even if the underlying binary hasn't changed
    pub async fn upgrade_code<'wallet_agent, T: CandidType + std::fmt::Debug>(
        &self,
        wallet: &Canister<'wallet_agent, Wallet>,
        canister_id: Principal,
        bytecode: Vec<u8>,
        arg: T,
    ) -> Result<()> {
        let install_args = CanisterInstall {
            mode: InstallMode::Upgrade,
            canister_id,
            wasm_module: bytecode,
            arg: Encode!(&arg)?,
        };

        let args = Encode!(&install_args)?;
        self.through_wallet_call::<Principal>(wallet, "upgrade_code", 0, Some(args)).await?;
        Ok(())
    }

    /// Stop a running canister
    pub async fn stop_canister<'wallet_agent>(
        &self,
        wallet: &Canister<'wallet_agent, Wallet>,
        canister_id: Principal, // canister to stop
    ) -> Result<()> {
        let arg = Encode!(&In { canister_id })?;
        self.through_wallet_call::<()>(wallet, "stop_canister", 0, Some(arg)).await?;
        Ok(())
    }

    /// Delete a canister. The target canister can not be running,
    /// make sure the canister has stopped first: [`Canister::stop_canister`]
    pub async fn delete_canister<'wallet_agent>(
        &self,
        wallet: &Canister<'wallet_agent, Wallet>,
        canister_id: Principal, // canister to delete
    ) -> Result<()> {
        let arg = Encode!(&In { canister_id })?;
        self.through_wallet_call(wallet, "delete_canister", 0, Some(arg)).await?;
        Ok(())
    }
}
