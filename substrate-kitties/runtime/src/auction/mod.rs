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
	Parameter,
	dispatch::Result
};
use system::ensure_signed;
use crate::traits::ItemTransfer;

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
		fn deposit_event() = default;

		pub fn do_something(origin, something: u32) -> Result {
			// TODO: You only need this if you want to check it was signed.
			let who = ensure_signed(origin)?;

			// TODO: Code to execute when something calls this.
			// For example: the following line stores the passed in u32 in the storage
			Something::put(something);

			// here we are raising the Something event
			Self::deposit_event(RawEvent::SomethingStored(something, who));
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		SomethingStored(u32, AccountId),
	}
);
