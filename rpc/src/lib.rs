//! Darwinia-specific RPCs implementation.

#![warn(missing_docs)]

// --- std ---
use std::sync::Arc;
// --- substrate ---
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{HeaderBackend, HeaderMetadata};
// --- darwinia ---
use darwinia_primitives::{AccountId, Balance, Block, BlockNumber, Hash, Nonce, Power};

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Light client extra dependencies.
pub struct LightDeps<C, F, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Remote access to the blockchain (async).
	pub remote_blockchain: Arc<dyn sc_client_api::light::RemoteBlockchain<Block>>,
	/// Fetcher instance.
	pub fetcher: Arc<F>,
}

/// Extra dependencies for BABE.
pub struct BabeDeps {
	/// BABE protocol config.
	pub babe_config: sc_consensus_babe::Config,
	/// BABE pending epoch changes.
	pub shared_epoch_changes:
		sc_consensus_epochs::SharedEpochChanges<Block, sc_consensus_babe::Epoch>,
	/// The keystore that manages the keys of the node.
	pub keystore: sc_keystore::KeyStorePtr,
}

/// Dependencies for GRANDPA
pub struct GrandpaDeps {
	/// Voting round info.
	pub shared_voter_state: sc_finality_grandpa::SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: sc_finality_grandpa::SharedAuthoritySet<Hash, BlockNumber>,
}

/// Full client dependencies
pub struct FullDeps<C, P, SC> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// The SelectChain Strategy
	pub select_chain: SC,
	/// Whether to deny unsafe calls
	pub deny_unsafe: sc_rpc::DenyUnsafe,
	/// BABE specific dependencies.
	pub babe: BabeDeps,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P, UE, SC>(deps: FullDeps<C, P, SC>) -> RpcExtension
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = sp_blockchain::Error>,
	C: 'static + Send + Sync,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance, UE>,
	C::Api: sp_consensus_babe::BabeApi<Block>,
	C::Api: darwinia_balances_rpc::BalancesRuntimeApi<Block, AccountId, Balance>,
	C::Api: darwinia_staking_rpc::StakingRuntimeApi<Block, AccountId, Power>,
	P: 'static + Sync + Send + sp_transaction_pool::TransactionPool,
	UE: 'static + Send + Sync + codec::Codec,
	SC: 'static + sp_consensus::SelectChain<Block>,
{
	// --- substrate ---
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
	use sc_consensus_babe_rpc::{BabeApi, BabeRpcHandler};
	use sc_finality_grandpa_rpc::{GrandpaApi, GrandpaRpcHandler};
	use substrate_frame_rpc_system::{FullSystem, SystemApi};
	// --- darwinia ---
	use darwinia_balances_rpc::{Balances, BalancesApi};
	use darwinia_staking_rpc::{Staking, StakingApi};

	let FullDeps {
		client,
		pool,
		select_chain,
		deny_unsafe,
		babe,
		grandpa,
	} = deps;

	let mut io = jsonrpc_core::IoHandler::default();
	io.extend_with(SystemApi::to_delegate(FullSystem::new(
		client.clone(),
		pool,
	)));
	io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
		client.clone(),
	)));
	{
		let BabeDeps {
			keystore,
			babe_config,
			shared_epoch_changes,
		} = babe;
		io.extend_with(BabeApi::to_delegate(BabeRpcHandler::new(
			client.clone(),
			shared_epoch_changes,
			keystore,
			babe_config,
			select_chain,
			deny_unsafe,
		)));
	}
	{
		let GrandpaDeps {
			shared_voter_state,
			shared_authority_set,
		} = grandpa;
		io.extend_with(GrandpaApi::to_delegate(GrandpaRpcHandler::new(
			shared_authority_set,
			shared_voter_state,
		)));
	}
	io.extend_with(BalancesApi::to_delegate(Balances::new(client.clone())));
	io.extend_with(StakingApi::to_delegate(Staking::new(client)));

	io
}

/// Instantiate all RPC extensions for light node.
pub fn create_light<C, P, F, UE>(deps: LightDeps<C, F, P>) -> RpcExtension
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C: 'static + Send + Sync,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance, UE>,
	C::Api: darwinia_balances_rpc::BalancesRuntimeApi<Block, AccountId, Balance>,
	C::Api: darwinia_staking_rpc::StakingRuntimeApi<Block, AccountId, Power>,
	P: 'static + Send + Sync + sp_transaction_pool::TransactionPool,
	F: 'static + sc_client_api::light::Fetcher<Block>,
	UE: 'static + Send + Sync + codec::Codec,
{
	// --- substrate ---
	use substrate_frame_rpc_system::{LightSystem, SystemApi};

	let LightDeps {
		client,
		pool,
		remote_blockchain,
		fetcher,
	} = deps;

	let mut io = jsonrpc_core::IoHandler::default();
	io.extend_with(SystemApi::<AccountId, Nonce>::to_delegate(
		LightSystem::new(client, remote_blockchain, fetcher, pool),
	));

	io
}
