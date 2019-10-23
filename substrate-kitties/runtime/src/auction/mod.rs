use sr_primitives::{RuntimeAppPublic};
use sr_primitives::traits::{
	SimpleArithmetic, Member, One, Zero,
	CheckedAdd, CheckedSub,
	Saturating, Bounded, SaturatedConversion,
};
use rstd::result;
use support::dispatch::Result;
use support::{
	decl_module, decl_storage, decl_event, Parameter, StorageDoubleMap,
	traits::{
		LockableCurrency, Currency,
		OnUnbalanced,
	}
};
use system::ensure_signed;
use codec::{Encode, Decode};
use rstd::vec::Vec;
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

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum AuctionStatus {
    PendingStart,
	Paused,
	Active,
	Stopped,
}

#[derive(Encode, Decode, Clone, PartialEq, Copy)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Auction<T> where T: Trait {
    id: T::AuctionId,
	item: T::ItemId, // 拍卖物品id
	owner: T::AccountId, // 拍卖管理账户，可以控制暂停和继续
    start_at: Option<T::Moment>, // 自动开始时间
	stop_at: Option<T::Moment>, // 截止时间
	wait_period: Option<T::Moment>, // 等待时间
	begin_price: BalanceOf<T>, // 起拍价
	upper_bound_price: Option<BalanceOf<T>>, // 封顶价（可选）
	minimum_step: BalanceOf<T>, // 最小加价幅度
	latest_participate: Option<(T::AccountId, T::Moment)>, // 最后出价人/时间
	status: AuctionStatus,
}

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Auction {
		NextAuctionId get(next_auction_id): T::AuctionId;
		
		// 物品id映射auctionid，一个物品只能在一个auction中参拍，创建auction后添加映射，auction结束后删除映射
		AuctionItems get(auction_items): map T::ItemId => Option<T::AuctionId>;
		Auctions get(auctions): map T::AuctionId => Option<Auction<T>>;
		AuctionBids get(auction_bids): double_map T::AuctionId, twox_128(T::AccountId) => Option<BalanceOf<T>>;
		AuctionParticipants get(action_participants): map T::AuctionId => Option<Vec<T::AccountId>>;
		PendingAuctions get(pending_auctions): Vec<T::AuctionId>; // 尚未开始的auction
		ActiveAuctions get(active_auctions): Vec<T::AuctionId>; // 尚未结束的auction，已经暂停的也在这里
	}
}

// add by sunhao 20191023
decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::AuctionId,
		Balance = BalanceOf<T>,
	{
		/// A price and/or amount is changed in some auction. 
		/// (auction_id, latest_bidder, latest_price, remain_amount)
		BidderUpdated(AuctionId, AccountId, Balance, u32),
		/// A auction's status has changed. (auction_id, status_from, status_to)
		AuctionUpdated(AuctionId, AuctionStatus, AuctionStatus),
	}
);

// The module's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		fn deposit_event() = default;

		pub fn create_auction(origin,
			item: T::ItemId,//竞拍对象
			begin_price: BalanceOf<T>,//起拍价
			minimum_step: BalanceOf<T>,//最小加价幅度
			upper_bound_price: Option<BalanceOf<T>>,//封顶价
			// start_at: T::Moment,//起拍时间
			// stop_at: T::Moment,//结束时间
			// wait_period: T::Moment //竞价等待时间
			) -> Result {
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


// decl_event!(
// 	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
// 		SomethingStored(u32, AccountId),
// 	}
// );

impl<T: Trait> Module<T> {
	fn do_create_auction(owner: T::AccountId, item: T::ItemId) -> result::Result<T::AuctionId, &'static str> {
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