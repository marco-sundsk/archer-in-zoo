use sr_primitives::traits::{
	SimpleArithmetic, Member, One, Zero,
	CheckedAdd, CheckedSub,
	Saturating, Bounded, SaturatedConversion,
};
use support::{
	decl_module, decl_storage, decl_event,
	traits::{
		LockableCurrency, Currency,
		Time, OnUnbalanced,
	},
	StorageValue, Parameter,
	dispatch::Result
};
use system::ensure_signed;

/// The module's configuration trait.
pub trait Trait: system::Trait {
	type ItemId: Parameter
		+ Member
		+ SimpleArithmetic
		+ Bounded
		+ Default
		+ Copy;

	type AuctionId: Parameter
		+ Member
		+ SimpleArithmetic
		+ Bounded
		+ Default
		+ Copy;

	/// Currency type for this module.
	type Currency: LockableCurrency<Self::AccountId>;

	/// Time used for computing auction.
	type Time: Time;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// Handler for the unbalanced reduction when taking a auction fee.
	type OnAuctionPayment: OnUnbalanced<NegativeImbalanceOf<Self>>;

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
		fn deposit_event() = default;

		pub fn create_auction(origin, item: ItemId, start_at: Time, stop_at: Time ) -> Result {
			Ok(())
		}

		pub fn pause_auction(origin, auction: AuctionId) -> Result {
			Ok(())
		}

		pub fn resume_auction(origin, auction: AuctionId) -> Result {
			Ok(())
		}

		pub fn start_auction(
			origin,
			auction: AuctionId,
			signature: <T::AuthorityId as RuntimeAppPublic>::Signature
		) -> Result { // Called by offchain worker
			Ok(())
		}

		pub fn stop_auction(
			origin,
			auction: AuctionId,
			signature: <T::AuthorityId as RuntimeAppPublic>::Signature
		) -> Result { // Called by offchain worker
			Ok(())
		}

		pub fn participate_auction(
			origin,
			auction: AuctionId,
			price: BalanceOf<T>
		) -> Result {
			Ok(())
		}

		// Runs after every block.
		fn offchain_worker(now: T::BlockNumber) {
		}
	}
}

impl<T: Trait> Module<T> {
	fn do_create_auction(owner: T::AccountId, item: ItemId, start_at: Time, stop_at: Time) -> result::Result<T::AuctionId, &'static str> {
		Ok(())
	}

	fn do_enable_auction(auction: AuctionId) -> Result {
		Ok(())
	}

	fn do_disable_auction(auction: AuctionId) -> Result {
		Ok(())
	}

	fn do_settle_auction(auction: AuctionId) -> Result {
		Ok(())
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		SomethingStored(u32, AccountId),
	}
);
