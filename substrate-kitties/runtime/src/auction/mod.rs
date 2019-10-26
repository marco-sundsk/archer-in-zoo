use codec::{Encode, Decode};
use rstd::prelude::*;
use rstd::{result, vec::Vec};
use sr_primitives::{RuntimeAppPublic, RuntimeDebug};
use sr_primitives::traits::{
	SimpleArithmetic, Member, Bounded, Zero, One,
	Printable,
	CheckedAdd, CheckedSub,
};
use sr_primitives::transaction_validity::{
	TransactionValidity, TransactionLongevity, ValidTransaction, InvalidTransaction,
};
use support::dispatch::Result;
use support::{
	decl_module, decl_storage, decl_event, Parameter, ensure, print, debug,
	traits::{
		LockIdentifier, WithdrawReasons, WithdrawReason,
		LockableCurrency, Currency, ExistenceRequirement,
		OnUnbalanced,
	}
};
use system::{ensure_none, ensure_signed};
use system::offchain::SubmitUnsignedTransaction;

use crate::traits::ItemTransfer;

// Tests part
// mod mocks;
// mod tests;

const AUCTION_ID: LockIdentifier = *b"auction ";

/// Error which may occur while executing the off-chain code.
#[derive(RuntimeDebug)]
enum OffchainErr {
	MissingKey,
	FailedSigning,
	SubmitTransaction,
}

impl Printable for OffchainErr {
	fn print(&self) {
		match self {
			OffchainErr::MissingKey => print("Offchain error: failed to find authority key"),
			OffchainErr::FailedSigning => print("Offchain error: signing failed!"),
			OffchainErr::SubmitTransaction => print("Offchain error: submitting transaction failed!"),
		}
	}
}

/// The module's configuration trait.
pub trait Trait: timestamp::Trait + aura::Trait {
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
	type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// A dispatchable call type.
	type Call: From<Call<Self>>;

	/// A transaction submitter.
	type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;
	
	/// Interface for transfer item
	type AuctionTransfer: ItemTransfer<Self::AccountId, Self::ItemId>;

	/// Handler for the unbalanced reduction when taking a auction fee.
	type OnAuctionPayment: OnUnbalanced<NegativeImbalanceOf<Self>>;
}

pub type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
type NegativeImbalanceOf<T> =
	<<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;
type SignatureOf<T> = <<T as aura::Trait>::AuthorityId as RuntimeAppPublic>::Signature;

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum AuctionStatus {
	PendingStart,
	Paused,
	Active,
	Stopped,
}
// method for error string
impl AuctionStatus {
	/// Whether this block is the new best block.
	pub fn error_str(self) -> &'static str {
		match self {
			AuctionStatus::PendingStart => "Auction is already started or over.",
			AuctionStatus::Paused => "Auction should be paused.",
			AuctionStatus::Active => "Auction should be acive.",
			AuctionStatus::Stopped => "Auction should be stopped.",
		}
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Copy)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Auction<T> where T: Trait {
	id: T::AuctionId,
	item: Option<T::ItemId>, // 拍卖物品id
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
// No need [commented by Tang]
// #[derive(Encode, Decode, Clone, PartialEq)]
// #[cfg_attr(feature = "std", derive(Debug))]
// pub struct DetailAuction<T> where T: Trait {
// 	auction: Auction<T>,//
// 	is_participate: bool,//是否参与
// 	participate_price: BalanceOf<T>,//参与的最新出价
// }

// helper enum for auction_ids vec
enum StoreVecs {
	PendingVec,
	ActiveVec
}

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Auctions {
		NextAuctionId get(fn next_auction_id): T::AuctionId;
		
		// 记录账户全局lock的余额数量，不同auction中lock的余额汇总在这里
		AccountLocks get(fn account_locks): map T::AccountId => BalanceOf<T>;

		// 物品id映射auctionid，一个物品只能在一个auction中参拍，创建auction后添加映射，auction结束后删除映射
		AuctionItems get(fn auction_items): map T::ItemId => Option<T::AuctionId>;
		Auctions get(fn auctions): map T::AuctionId => Option<Auction<T>>;
		AuctionBids get(fn auction_bids): double_map T::AuctionId, twox_128(T::AccountId) => BalanceOf<T>;
		AuctionParticipants get(fn auction_participants): map T::AuctionId => Option<Vec<T::AccountId>>;

		// Auction workinig list
		PendingAuctions get(fn pending_auctions): Vec<T::AuctionId>; // 尚未开始的auction
		ActiveAuctions get(fn active_auctions): Vec<T::AuctionId>; // 尚未结束的auction，已经暂停的也在这里
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
			// item: T::ItemId,//竞拍对象
			begin_price: BalanceOf<T>,//起拍价
			minimum_step: BalanceOf<T>,//最小加价幅度
			upper_bound_price: Option<BalanceOf<T>>,//封顶价
			// start_at: T::Moment,//起拍时间
			// stop_at: T::Moment,//结束时间
			// wait_period: T::Moment //竞价等待时间
		) -> Result {
			let sender = ensure_signed(origin)?;

			Self::do_create_auction(&sender, begin_price,minimum_step, upper_bound_price)?;

			Ok(())
		}
		pub fn add_item(origin,
			auction_id: T::AuctionId,
			item: T::ItemId,//竞拍对象
		) -> Result {
			let sender = ensure_signed(origin)?;

			Self::do_add_item(&sender, auction_id,item)
		}
		// setup start and/or stop Moment, and wait_period after someone's bid
		// add by sunhao 20191023
		// separated by Tang 20191024
		pub fn setup_moments(origin,
			auction_id: T::AuctionId, 
			start_at: Option<T::Moment>,  //起拍时间
			stop_at: Option<T::Moment>,  //结束时间
			wait_period: Option<T::Moment>  //竞价等待时间
		) -> Result {
			let sender = ensure_signed(origin)?;

			Self::do_setup_moments(&sender, auction_id, start_at, stop_at, wait_period)
		}

		// Owner can pause the auction when it is in active.
		// add by sunhao 20191024
		// separated by Tang 20191024
		pub fn pause_auction(origin, auction_id: T::AuctionId) -> Result {
			let sender = ensure_signed(origin)?;

			Self::do_pause_auction(&sender, auction_id)
		}

		// Owner can resume the auction paused before.
		// add by sunhao 20191024
		// separated by Tang 20191024
		pub fn resume_auction(origin, auction_id: T::AuctionId) -> Result {
			let sender = ensure_signed(origin)?;

			Self::do_resume_auction(&sender, auction_id)
		}

		// owner can stop an active or paused auction by his will.
		// add by sunhao 20191024
		pub fn stop_auction(
			origin,
			auction_id: T::AuctionId
		) -> Result {
			let sender = ensure_signed(origin)?;

			Self::do_stop_auction(&sender, auction_id)?;
			
			Ok(())
		}

		pub fn participate_auction(
			origin,
			auction_id: T::AuctionId,
			price: BalanceOf<T>
		) -> Result {
			let participant = ensure_signed(origin)?;

			// unwrap auction and ensure its status is Active
			let auction = Self::_ensure_auction_with_status(auction_id, Some(AuctionStatus::Active), None)?;

			match auction.latest_participate {
					Some((_account, _moment)) => { // 已经有用户出价
							let bid_price = <AuctionBids<T>>::get(auction.id, _account);
							ensure!(price > bid_price + auction.minimum_step, "Increment of bid price less than minimum step ");
					},
					_ => {}, // 尚无用户出价
			};

			let mut delta_price = price;
			if <AuctionBids<T>>::exists(auction.id, &participant) { // 已经参与过的用户再次出价
				let prev_bid = <AuctionBids<T>>::get(auction.id, &participant);
				delta_price = price - prev_bid;
			}

			ensure!(delta_price < T::Currency::free_balance(&participant), "No enough balance to lock");

			Self::do_lock_balance(&auction_id, &participant, delta_price)?;
			Self::do_participate_auction(&auction_id, &participant, price)?;
			
			Ok(())
		}

		// No need [commented by Tang]
		// //query one auction with auctionId
		// pub fn query_one_auction(
		// 	origin,
		// 	auction: T::AuctionId,
		// 	) {
		// 	let sender = ensure_signed(origin)?;
		// 	Self::do_query_one_auction(auction, sender.clone())?;
		// }

		// ===== passive method =====
		// starting auction methods
		// Called by offchain worker
		fn start_auctions_passive(
			origin,
			auction_ids: Vec<T::AuctionId>,
			signature: SignatureOf<T>
		) -> Result {
			ensure_none(origin)?;
			// ensure status
			ensure!(Self::is_auctions_with_status(&auction_ids, AuctionStatus::PendingStart, false), "auctions should be pending start");

			// key validating
			if let Some(key) = Self::authority_id() {
				let signature_valid = auction_ids.using_encoded(|encoded_auction_ids| {
					key.verify(&encoded_auction_ids, &signature)
				});
				ensure!(signature_valid, "Invalid signature.");

				// set status as active
				auction_ids.iter().for_each(|auction_id| {
					Self::_change_auction_status(*auction_id, AuctionStatus::PendingStart, AuctionStatus::Active);
				});
				// remove auction_ids from pendings
				Self::remove_all_from_set(StoreVecs::PendingVec, &auction_ids);
				// add auction_ids to active_auctions
				Self::add_all_to_set(StoreVecs::ActiveVec, &auction_ids);

				Ok(())
			} else {
				Err("Non existent public key.")?
			}
		}

		// stoping auction methods
		// Called by offchain worker
		fn stop_auctions_passive(
			origin,
			auction_ids: Vec<T::AuctionId>,
			signature: SignatureOf<T>
		) -> Result {
			ensure_none(origin)?;
			// ensure status
			ensure!(Self::is_auctions_with_status(&auction_ids, AuctionStatus::Stopped, true), "auctions should be non stopped");

			// key validating
			if let Some(key) = Self::authority_id() {
				let signature_valid = auction_ids.using_encoded(|encoded_auction_ids| {
					key.verify(&encoded_auction_ids, &signature)
				});
				ensure!(signature_valid, "Invalid signature.");

				// set status as active
				auction_ids.iter().for_each(|auction_id| {
					if let Some(auction) = Self::auctions(auction_id) {
						// call settle func if needed.
						if auction.status != AuctionStatus::PendingStart {
							match Self::do_settle_auction(&auction) {
								Err(_) => {}, // DO SOMETHING?
								Ok(_) => {},
							}
						}
						Self::_change_auction_status(*auction_id, auction.status, AuctionStatus::Stopped);
					}
				});
				// remove auction_ids from active_auctions
				Self::remove_all_from_set(StoreVecs::ActiveVec, &auction_ids);

				Ok(())
			} else {
				Err("Non existent public key.")?
			}
		}
		
		// Runs after every block.
		fn offchain_worker(now: <T as system::Trait>::BlockNumber) {
			debug::RuntimeLogger::init();

			// Only send messages if we are a potential validator.
			if runtime_io::is_validator() {
				Self::offchain(now);
			}
		}
	}
}

impl<T: Trait> Module<T> {
	// ====== exported public methods ======
	/// Returns own authority identifier iff it is part of the current authority
	/// set, otherwise this function returns None. The restriction might be
	/// softened in the future in case a consumer needs to learn own authority
	/// identifier.
	pub fn authority_id() -> Option<T::AuthorityId> {
		let authorities = <aura::Module<T>>::authorities();

		let local_keys = T::AuthorityId::all();

		authorities.into_iter().find_map(|authority| {
			if local_keys.contains(&authority) {
				Some(authority)
			} else {
				None
			}
		})
	}
	pub fn is_auctions_with_status (
		auction_ids: &Vec<T::AuctionId>,
		status: AuctionStatus,
		not_equal: bool
	) -> bool {
		auction_ids.iter().all(|&auction_id| {
			if let Some(auction) = Self::auctions(auction_id) {
				if not_equal {
					auction.status != status
				} else {
					auction.status == status
				}
			} else {
				false
			}
		})
	}

	// ====== module private methods ======
	fn get_next_auction_id() -> result::Result<T::AuctionId, &'static str> {
		let auction_id = Self::next_auction_id();
		if auction_id == T::AuctionId::max_value() {
			return Err("Auction count overflow");
		}
		Ok(auction_id)
	}

	// utility method for ensure auction status
	// add by Tang 20191024
	fn _ensure_auction_with_status(
		auction_id: T::AuctionId,
		status: Option<AuctionStatus>,
		owner: Option<&T::AccountId>
	) -> result::Result<Auction<T>, &'static str> {
		// unwrap auction and ensure its status
		let auction = Self::auctions(auction_id);
		ensure!(auction.is_some(), "Auction does not exist");

		let auction = auction.unwrap();
		// check status equel
		if let Some(s) = status {
			ensure!(auction.status == s, s.error_str());
		}
		// check owner or not
		if let Some(account) = owner {
			// ensure only owner can call this
			ensure!(auction.owner == *account, "Only owner can call this fn.");
		}

		Ok(auction)
	}

	/// add auction ids to a store vec set.
	/// add by sunhao 20191024
	/// modified by Tang 20191025
	fn add_all_to_set(vec_type: StoreVecs, auction_ids: &Vec<T::AuctionId>) {
		let mut stored_auction_ids = match vec_type {
			StoreVecs::PendingVec => Self::pending_auctions(),
			StoreVecs::ActiveVec => Self::active_auctions(),
		};
		// add to set
		for auction_id in auction_ids.iter() {
			let exists = stored_auction_ids.iter().all(|auc_id| auc_id != auction_id);
			if !exists {
				stored_auction_ids.push(*auction_id);
			}
		}
		// save to store
		match vec_type {
			StoreVecs::PendingVec => <PendingAuctions<T>>::put(stored_auction_ids),
			StoreVecs::ActiveVec => <ActiveAuctions<T>>::put(stored_auction_ids),
		};
	}

	/// remove an auction from a store vec set.
	/// add by sunhao 20191024
	/// modified by Tang 20191025
	fn remove_all_from_set(vec_type: StoreVecs, auction_ids: &Vec<T::AuctionId>) {
		let mut stored_auction_ids = match vec_type {
			StoreVecs::PendingVec => Self::pending_auctions(),
			StoreVecs::ActiveVec => Self::active_auctions(),
		};
		// remove from set
		for auction_id in auction_ids.iter() {
			stored_auction_ids = stored_auction_ids.iter()
				.filter_map(|auc_id| if auc_id == auction_id {
					None
				} else {
					Some(*auc_id)
				})
				.collect();
		}
		
		// save to store
		match vec_type {
			StoreVecs::PendingVec => <PendingAuctions<T>>::put(stored_auction_ids),
			StoreVecs::ActiveVec => <ActiveAuctions<T>>::put(stored_auction_ids),
		};
	}

	fn insert_auction(auction_id: T::AuctionId, auction:Auction<T>) {
		// Create and store kitty
		<Auctions<T>>::insert(auction_id, auction);
		<NextAuctionId<T>>::put(auction_id + 1.into());
	}

	fn do_create_auction(
		owner: &T::AccountId, 
		begin_price: BalanceOf<T>,//起拍价
		minimum_step: BalanceOf<T>,//最小加价幅度
		upper_bound_price: Option<BalanceOf<T>>
	) -> result::Result<T::AuctionId, &'static str> {
		// 判断id
		let auction_id = Self::get_next_auction_id()?;
		let new_auction = Auction {
			id: auction_id,
			item: None, // 拍卖物品id
			owner: (*owner).clone(), // 拍卖管理账户，可以控制暂停和继续
			begin_price: begin_price, // 起拍价
			minimum_step: minimum_step, // 最小加价幅度
			status: AuctionStatus::PendingStart,
			upper_bound_price: upper_bound_price,
			start_at: None,
			stop_at:None,
			wait_period: None,
			latest_participate: None,
		};
		Self::insert_auction(auction_id, new_auction);
		Ok(auction_id)
	}

	fn do_add_item(
		sender: &T::AccountId, 
		auction_id: T::AuctionId,
		item: T::ItemId,//竞拍对象
	) -> Result {
		// ensure item owner
		ensure!(T::AuctionTransfer::is_item_owner(sender, item), "you should be item's owner.");

		// unwrap auction and ensure its status is PendingStart
		let mut auction = Self::_ensure_auction_with_status(auction_id, Some(AuctionStatus::PendingStart), Some(sender))?;

		// change status of auction
		auction.item = Some(item);
		<Auctions<T>>::insert(auction_id, auction);

		Ok(())
	}

	// real work for do_setup_moments.
	// separated by Tang 20191024
	fn do_setup_moments(
		owner: &T::AccountId,
		auction_id: T::AuctionId,
		start_at: Option<T::Moment>,  //起拍时间
		stop_at: Option<T::Moment>,  //结束时间
		wait_period: Option<T::Moment>  //竞价等待时间
	) -> Result {
		// unwrap auction and ensure its status is PendingStart
		let mut auction = Self::_ensure_auction_with_status(auction_id, Some(AuctionStatus::PendingStart), Some(owner))?;

		// set moments into storage
		if start_at.is_some() {
			auction.start_at = start_at;
		}
		if stop_at.is_some() {
			auction.stop_at = stop_at;
		}
		if wait_period.is_some() {
			auction.wait_period = wait_period;
		}

		// save to storage
		<Auctions<T>>::insert(auction_id, auction);

		// ensure this auction in pending queue, once owner call this fn.
		Self::add_all_to_set(StoreVecs::PendingVec,  &vec![auction_id]);
			
		Ok(())
	}

	// real work for do_pause_auction
	// modified by Tang 20191025
	fn do_pause_auction(
		owner: &T::AccountId,
		auction_id: T::AuctionId
	) -> Result {
		// unwrap auction and ensure its status is Active
		let auction = Self::_ensure_auction_with_status(auction_id, Some(AuctionStatus::Active), Some(owner))?;

		// change status of auction
		Self::_change_auction_status(auction_id, auction.status, AuctionStatus::Paused);

		Ok(())
	}

	// real work for do_resume_auction
	// separated by Tang 20191024
	// modified by Tang 20191025
	fn do_resume_auction(
		owner: &T::AccountId,
		auction_id: T::AuctionId
	) -> Result {
		// unwrap auction and ensure its status is Paused
		let auction = Self::_ensure_auction_with_status(auction_id, Some(AuctionStatus::Paused), Some(owner))?;

		// change status of auction
		Self::_change_auction_status(auction_id, auction.status, AuctionStatus::Active);

		Ok(())
	}

	// storage work for auction status
	// added by Tang 20191025
	fn _change_auction_status(
		auction_id: T::AuctionId,
		old_status: AuctionStatus,
		new_status: AuctionStatus,
	) {
		<Auctions<T>>::mutate(auction_id, |auc| {
			if let Some(auction) = auc {
				auction.status = new_status;
			}
		});
		// emit event
		Self::deposit_event(RawEvent::AuctionUpdated(auction_id, old_status, new_status));
	}

	// real work for stopping a auction.
	// added by sunhao 20191024
	// modified by Tang 20191024
	fn do_stop_auction(
		owner: &T::AccountId,
		auction_id: T::AuctionId
	) -> Result {
		// unwrap auction and ensure its status is not stopped yet.
		let auction = Self::_ensure_auction_with_status(auction_id, None, Some(owner))?;

		ensure!(auction.status != AuctionStatus::Stopped,
			"Auction can NOT be stopped now.");

		// call settle func if needed.
		if auction.status != AuctionStatus::PendingStart {
			Self::do_settle_auction(&auction)?;
		}

		// change status of auction
		Self::_change_auction_status(auction_id, auction.status, AuctionStatus::Stopped);
		
		// remove from active vecs
		Self::remove_all_from_set(StoreVecs::ActiveVec, &vec![auction_id]);

		Ok(())
	}

	fn do_settle_auction(auction: &Auction<T>) -> Result {
		// unlock all participents' balance
		if let Some(participants) = <AuctionParticipants<T>>::get(auction.id) {
			participants.iter().try_for_each(|account| {
				Self::do_unlock_balance(&auction.id, account)
			})?;
		}

		// handle winner
		let owner = &auction.owner;
		// transfer auction item to winner
		if let Some(item_id) = auction.item {
			if let Some((winner, _)) = &auction.latest_participate {
				let winner_bid = <AuctionBids<T>>::get(&auction.id, winner);
				let (tranfer_value, fee) = Self::_calc_auctino_fee(winner_bid);

				// FIXME: first ensure then modify store
				// withdraw imbalance
				let fee_imbalance = T::Currency::withdraw(
					winner,
					fee,
					WithdrawReason::Fee,
					ExistenceRequirement::KeepAlive
				)?;
				// transfer auction balance
				T::Currency::transfer(winner, owner, tranfer_value)?;

				// try transfer item
				T::AuctionTransfer::transfer_item(owner, winner, item_id)?;

				// trigger imbalance interface
				T::OnAuctionPayment::on_unbalanced(fee_imbalance);
			}
		}

		Ok(())
	}

	/// FIXME using configable ratio
	/// return transfer value and fee
	fn _calc_auctino_fee (
		price: BalanceOf<T>
	) -> (BalanceOf<T>, BalanceOf<T>) {
		let fee = (price / 20.into()).min(One::one());
		(price - fee, fee)
	}

	fn do_lock_balance(auction: &T::AuctionId, account: &T::AccountId, balance: BalanceOf<T>) -> Result {
		// 账户在auction下锁定一些资产，如果已经锁过，这里会追加锁仓， balance是追价的delta部分

		// 增加全局锁仓
		let mut global_lock = balance;
		if <AccountLocks<T>>::exists(account) {
			global_lock = global_lock.checked_add(&Self::account_locks(account)).ok_or("balance add overflow")?;
		}
		<AccountLocks<T>>::insert(account, global_lock);
		
		// 增加auction下锁仓
		let mut auction_lock = balance;
		if <AuctionBids<T>>::exists(auction, account) {
			auction_lock = auction_lock.checked_add(&Self::auction_bids(auction, account)).ok_or("balance add overflow")?;
		}
		<AuctionBids<T>>::insert(auction, account, auction_lock);

		// 调用锁仓接口
		T::Currency::extend_lock(
			AUCTION_ID,
			account,
			global_lock,
			<T as system::Trait>::BlockNumber::max_value(),
			WithdrawReasons::none());
	
		Ok(())
	}

	fn do_unlock_balance(auction: &T::AuctionId, account: &T::AccountId) -> Result {
		// 解锁账户在auction下锁定的所有资产

		// 获取用户在auction下的锁仓
		if <AuctionBids<T>>::exists(auction, account) {
			let auction_lock = Self::auction_bids(auction, account);

			// 获取用户全局锁仓
			ensure!(<AccountLocks<T>>::exists(account), "fatal error, can not find global lock for account");
			let mut global_lock = Self::account_locks(account);
			ensure!(global_lock >= auction_lock, "fatal error, global lock less than auction lock");
			
			// [No need remove, (commented by Tang)]
			// <AuctionBids<T>>::remove(auction, account);
			global_lock = global_lock.checked_sub(&auction_lock).ok_or("balance sub overflow")?;
			// 调用锁仓接口
			if global_lock == Zero::zero() {
				<AccountLocks<T>>::remove(account);
				T::Currency::remove_lock(AUCTION_ID, account);
			} else {
				<AccountLocks<T>>::insert(account, global_lock);
				T::Currency::set_lock(
					AUCTION_ID,
					account,
					global_lock,
					<T as system::Trait>::BlockNumber::max_value(),
					WithdrawReasons::none());
			}
		}
		Ok(())
	}

	fn do_participate_auction(auction_id: &T::AuctionId, account: &T::AccountId, price: BalanceOf<T>) -> Result {
		<Auctions<T>>::mutate(auction_id, |a|{
				if let Some(auc) = a {
					auc.latest_participate = Option::Some((account.clone(), <aura::Module<T>>::last()));
				}
			});

		let mut participants;
		if let Some(p) = Self::auction_participants(auction_id) {
			participants = p;
		} else {
			participants = Vec::<T::AccountId>::new();
		}

		if !participants.contains(account) {
			participants.push(account.clone());
		}

		<AuctionParticipants<T>>::insert(auction_id, participants);

		// emit event
		Self::deposit_event(RawEvent::BidderUpdated(*auction_id, account.clone(), price, 0));

		Ok(())
	}

	// ====== offchain worker related methods ======
	/// only run by current validator
	pub(crate) fn offchain(_now: T::BlockNumber) {
		// check auction start
		let pending_auctions = <PendingAuctions<T>>::get();
		let last_timestamp = <aura::Module<T>>::last();

		let starting_auction_ids: Vec<T::AuctionId> = pending_auctions.into_iter()
			.filter_map(|auction_id| {
				let auction = match <Auctions<T>>::get(auction_id) {
					Some(a) => a,
					None => return None,
				};
				// ensure now is pending start
				if auction.status != AuctionStatus::PendingStart {
					return None;
				}
				let start_at = match auction.start_at {
					Some(t) => t,
					None => return None,
				};
				// Condition: start_at < now
				if start_at < last_timestamp {
					Some(auction.id)
				} else {
					None
				}
			})
			.collect();
		// only start matched
		match Self::_send_auction_start_tx(starting_auction_ids) {
			Ok(_) => {},
			Err(err) => print(err),
		}

		// check auction end
		let active_auctions = <ActiveAuctions<T>>::get();

		let stoping_auction_ids: Vec<T::AuctionId> = active_auctions.into_iter()
			.filter_map(|auction_id| {
				let auction = match <Auctions<T>>::get(auction_id) {
					Some(a) => a,
					None => return None,
				};
				// ensure now auction is not Stopped
				if auction.status == AuctionStatus::Stopped {
					return None;
				}
				let stop_at = match auction.stop_at {
					Some(t) => t,
					None => return None,
				};
				// Condition A: stop_at < now
				if stop_at < last_timestamp {
					return Some(auction.id);
				}
				// Condition B: reach upper_bound_price
				let upper_bound_price = match auction.upper_bound_price {
					Some(v) => v,
					None => return None,
				};
				// get last participate price
				if let Some((account_id, _)) = auction.latest_participate {
					let last_price = <AuctionBids<T>>::get(&auction.id, account_id);
					if last_price >= upper_bound_price {
						return Some(auction.id);
					}
				}
				None
			})
			.collect();
		// only stop matched
		match Self::_send_auction_stop_tx(stoping_auction_ids) {
			Ok(_) => {},
			Err(err) => print(err),
		}
	}

	fn _send_auction_start_tx(
		auction_ids: Vec<T::AuctionId>
	) -> result::Result<(), OffchainErr> {
		let signature = Self::_sign_unchecked_payload(&auction_ids.encode())?;
		let call = Call::<T>::start_auctions_passive(auction_ids, signature);
		
		T::SubmitTransaction::submit_unsigned(call)
			.map_err(|_| OffchainErr::SubmitTransaction)?;
		Ok(())
	}

	fn _send_auction_stop_tx(
		auction_ids: Vec<T::AuctionId>
	) -> result::Result<(), OffchainErr> {
		let signature = Self::_sign_unchecked_payload(&auction_ids.encode())?;
		let call = Call::<T>::stop_auctions_passive(auction_ids, signature);
		
		T::SubmitTransaction::submit_unsigned(call)
			.map_err(|_| OffchainErr::SubmitTransaction)?;
		Ok(())
	}

	/// Sign for unchecked transaction
	fn _sign_unchecked_payload(payload: &Vec<u8>) -> result::Result<SignatureOf<T>, OffchainErr> {
		let key = Self::authority_id();
		if key.is_none() {
			return Err(OffchainErr::MissingKey);
		}
		let sig = key.unwrap().sign(payload).ok_or(OffchainErr::FailedSigning)?;
		Ok(sig)
	}

	// No need [commented by Tang]
	// fn do_query_one_auction(
	// 	auction: T::AuctionId,
	// 	sender: T::AccountId
	// ) -> result::Result<DetailAuction<T>, &'static str> {
	// 	let one_auction = Self::auctions(auction);
	// 	let sender_bid = Self::auction_bids(auction, sender.clone());
	// 	let account_ids = Self::auction_participants(auction).unwrap();
	// 	let mut is_bool: bool = false;
	// 
	// 	ensure!(one_auction.is_some(), "One invalid auction");
	// 
	// 	let detail_auction :DetailAuction<T>;
	// 	if let Some(auction) = one_auction{
	// 		for i in &account_ids {
	// 			if let i = sender.clone() {
	// 				is_bool = true;
	// 			}
	// 		}
	// 		detail_auction = DetailAuction {
	// 			auction: auction,
	// 			is_participate: is_bool,
	// 			participate_price: sender_bid,
	// 		};
	// 		return Ok(detail_auction);
	// 	}
	// 	Err("query fail")
	// }
}

impl<T: Trait> support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
		// verify that the incoming (unverified) pubkey is actually an authority id
		let authority_id = match <Module<T>>::authority_id() {
			Some(id) => id,
			None => return InvalidTransaction::BadProof.into(),
		};

		if let Call::start_auctions_passive(auction_ids, signature) = call {
			if !<Module<T>>::is_auctions_with_status(&auction_ids, AuctionStatus::PendingStart, false) {
				// all auction ids should be pending start
				return InvalidTransaction::Stale.into();
			}
			
			// check signature (this is expensive so we do it last).
			let signature_valid = auction_ids.using_encoded(|encoded_auction_ids| {
				authority_id.verify(&encoded_auction_ids, &signature)
			});

			if !signature_valid {
				return InvalidTransaction::BadProof.into();
			}

			Ok(ValidTransaction {
				priority: 0,
				requires: vec![],
				provides: vec![(auction_ids, authority_id).encode()],
				longevity: TransactionLongevity::max_value(),
				propagate: true,
			})
		} else if let Call::stop_auctions_passive(auction_ids, signature) = call {			
			if !<Module<T>>::is_auctions_with_status(&auction_ids, AuctionStatus::Stopped, true) {
				// all auction ids should be active start
				return InvalidTransaction::Stale.into();
			}

			// check signature (this is expensive so we do it last).
			let signature_valid = auction_ids.using_encoded(|encoded_auction_ids| {
				authority_id.verify(&encoded_auction_ids, &signature)
			});

			if !signature_valid {
				return InvalidTransaction::BadProof.into();
			}

			Ok(ValidTransaction {
				priority: 0,
				requires: vec![],
				provides: vec![(auction_ids, authority_id).encode()],
				longevity: TransactionLongevity::max_value(),
				propagate: true,
			})
		} else {
			InvalidTransaction::Call.into()
		}
	}
}
