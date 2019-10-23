//! Test utilities
#![cfg(test)]

use super::*;

use primitives::H256;
use support::{impl_outer_origin, assert_ok, parameter_types};
use sr_primitives::{
  traits::{BlakeTwo256, IdentityLookup}, testing::Header, weights::Weight, Perbill,
};

impl_outer_origin! {
  pub enum Origin for Test {}
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
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
	pub const TransferFee: Balance = 0;
	pub const CreationFee: Balance = 0;
}
impl balances::Trait for Test {
	type Balance = Balance;
	type OnFreeBalanceZero = Staking;
	type OnNewAccount = ();
	type Event = ();
	type TransferPayment = ();
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}

impl Trait for Test {
  type Event = ();
  type ItemId = u32;
  type AuctionId = u32;
  type Currency = balances::Module<Self>;
	type Time = timestamp::Module<Self>;
  type OnAuctionPayment = ();
}
pub type TheModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}
