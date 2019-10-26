//! Test utilities
#![cfg(test)]

use std::cell::RefCell;
use {runtime_io, system};

use primitives::H256;
use aura_primitives::ed25519::AuthorityId;
use sr_primitives::{
  Perbill,
  weights::Weight,
  testing::{Header, TestXt},
  traits::{
    BlakeTwo256, IdentityLookup
  },
};
use support::{
  impl_outer_origin, impl_outer_dispatch, parameter_types,
  // traits::{Currency},
  dispatch::Result,
};

use super::*;
use crate::traits::ItemTransfer;

/// The AccountId alias in this test module.
pub type AccountId = u64;
pub type Balance = u64;
pub type ItemId = u32;

impl_outer_origin! {
  pub enum Origin for Test {}
}

impl_outer_dispatch! {
  pub enum Call for Test where origin: Origin {
    auction::Auctions,
	}
}

thread_local! {
	pub static VALIDATORS: RefCell<Option<Vec<u64>>> = RefCell::new(Some(vec![1, 2, 3]));
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;
type SubmitTransaction = system::offchain::TransactionSubmitter<(), Call, Extrinsic>;

/// struct for item transfer 
pub struct SomeItemModule;
impl ItemTransfer<AccountId, ItemId> for SomeItemModule {
  fn is_item_owner(who: &AccountId, item_id: ItemId) -> bool {
    true
  }
	fn transfer_item(source: &AccountId, dest: &AccountId, item_id: ItemId) -> Result {
		Ok(())
	}
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
  pub const BlockHashCount: u64 = 250;
  pub const MaximumBlockWeight: Weight = 1024;
  pub const MaximumBlockLength: u32 = 2 * 1024;
  pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}
impl system::Trait for Test {
  type Origin = Origin;
  type Call = ();
  type Index = u64;
  type BlockNumber = u64;
  type Hash = H256;
  type Hashing = BlakeTwo256;
  type AccountId = u64;
  type Lookup = IdentityLookup<Self::AccountId>;
  type Header = Header;
  type Event = ();
  type BlockHashCount = BlockHashCount;
  type MaximumBlockWeight = MaximumBlockWeight;
  type MaximumBlockLength = MaximumBlockLength;
  type AvailableBlockRatio = AvailableBlockRatio;
  type Version = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl timestamp::Trait for Test {
	type Moment = u64;
	type OnTimestampSet = aura::Module<Self>;
	type MinimumPeriod = MinimumPeriod;
}

impl aura::Trait for Test {
  type AuthorityId = AuthorityId;
}

parameter_types! {
	pub const TransferFee: Balance = 0;
	pub const CreationFee: Balance = 0;
}
impl balances::Trait for Test {
	type Balance = Balance;
	type OnFreeBalanceZero = ();
	type OnNewAccount = ();
	type Event = ();
	type TransferPayment = ();
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}

impl Trait for Test {
  type Event = ();
  type ItemId = ItemId;
  type AuctionId = u32;
  type Currency = balances::Module<Self>;
	type OnAuctionPayment = ();
  // Offchain worker
  type Call = Call;
	type SubmitTransaction = SubmitTransaction;
	/// Interface for transfer item
	type AuctionTransfer = SomeItemModule;
}

pub type Auctons = Module<Test>;
pub type System = system::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Aura = aura::Module<Test>;

pub fn new_test_ext() -> runtime_io::TestExternalities {
	let t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	t.into()
}

pub fn next_block() {
	let now = System::block_number();
	System::set_block_number(now + 1);
}
