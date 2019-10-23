use sr_primitives::{RuntimeAppPublic};
use sr_primitives::traits::{
	SimpleArithmetic, Member, One, Zero,
	CheckedAdd, CheckedSub,
	Saturating, Bounded, SaturatedConversion,
};
use rstd::result;
use support::dispatch::Result;
use support::{
	decl_module, decl_storage, decl_event, Parameter,
	traits::{
		LockableCurrency, Currency,
		OnUnbalanced,
	}
};
use system::ensure_signed;

use crate::traits::ItemTransfer;

/// The module's configuration trait.
pub trait Trait: timestamp::Trait {
	/// The identifier type for an authority.
	type AuthorityId: Parameter
		+ Member
		+ RuntimeAppPublic
		+ Default;

	/// Item Id
	type ItemId: Parameter
		+ Member
		+ SimpleArithmetic
		+ Bounded
		+ Default
		+ Copy;

	/// Auction Id
	type AuctionId: Parameter
		+ Member
		+ SimpleArithmetic
		+ Bounded
		+ Default
		+ Copy;

	/// Currency type for this module.
	type Currency: LockableCurrency<Self::AccountId>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// Handler for the unbalanced reduction when taking a auction fee.
	type OnAuctionPayment: OnUnbalanced<NegativeImbalanceOf<Self>>;
	
	/// Interface for transfer item
	type AuctionTransfer: ItemTransfer<Self::AccountId, Self::ItemId>;

	// TODO more fee constant for auction
	// type AuctionBaseFee: Get<BalanceOf<Self>>;
}

pub type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
type NegativeImbalanceOf<T> =
	<<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Auction {
		Something get(something): Option<u32>;
	}
}

// The module's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// fn deposit_event() = default;

		pub fn create_auction(origin, item: T::ItemId, start_at: T::Moment, stop_at: T::Moment) -> Result {
			Ok(())
		}

		pub fn pause_auction(origin, auction: T::AuctionId) -> Result {
			Ok(())
		}

		pub fn resume_auction(origin, auction: T::AuctionId) -> Result {
			Ok(())
		}

		pub fn start_auction(
			origin,
			auction: T::AuctionId,
			signature: <T::AuthorityId as RuntimeAppPublic>::Signature
		) -> Result { // Called by offchain worker
			Ok(())
		}

		pub fn stop_auction(
			origin,
			auction: T::AuctionId,
			signature: <T::AuthorityId as RuntimeAppPublic>::Signature
		) -> Result { // Called by offchain worker
			Ok(())
		}

		pub fn participate_auction(
			origin,
			auction: T::AuctionId,
			price: BalanceOf<T>
		) -> Result {
			Ok(())
		}

		fn offchain_worker(now: <T as system::Trait>::BlockNumber) {
		}
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		SomethingStored(u32, AccountId),
	}
);

impl<T: Trait> Module<T> {
	fn do_create_auction(owner: T::AccountId, item: T::ItemId, start_at: T::Moment, stop_at: T::Moment) -> result::Result<T::AuctionId, &'static str> {
		Ok(T::AuctionId::zero())
	}

	fn do_enable_auction(auction: T::AuctionId) -> Result {
		Ok(())
	}

	fn do_disable_auction(auction: T::AuctionId) -> Result {
		Ok(())
	}

	fn do_settle_auction(auction: T::AuctionId) -> Result {
		Ok(())
	}
}