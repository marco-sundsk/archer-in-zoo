use sr_primitives::{RuntimeAppPublic, RuntimeDebug};
use sr_primitives::traits::{
	SimpleArithmetic, Member, Bounded, One, Zero,
	Printable,
	CheckedAdd, CheckedSub, Saturating, SaturatedConversion,
};
use sr_primitives::transaction_validity::{
	TransactionValidity, TransactionLongevity, ValidTransaction, InvalidTransaction,
};
use rstd::result;
use support::dispatch::Result;
use support::{
	decl_module, decl_storage, decl_event, Parameter, ensure, print,
	traits::{
		LockIdentifier, WithdrawReasons,
		LockableCurrency, Currency,
		OnUnbalanced,
	}
};
use system::ensure_signed;
use system::offchain::SubmitUnsignedTransaction;
use codec::{Encode, Decode};
use rstd::vec::Vec;
use crate::traits::ItemTransfer;

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

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Auction {
		NextAuctionId get(fn next_auction_id): T::AuctionId;
		
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

			Self::do_stop_auction(&sender, auction_id)
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

			Self::do_lock_balance(&participant, price)?;
			Self::do_participate_auction(&auction_id, &participant, price)?;
			
			// emit event
			Self::deposit_event(RawEvent::BidderUpdated(auction_id, 
				participant, price, 0));
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
		fn start_auction_passive(
			origin,
			auctions: Vec<T::AuctionId>,
			signature: SignatureOf<T>
		) -> Result {
			Ok(())
		}

		// stoping auction methods
		// Called by offchain worker
		fn stop_auction_passive(
			origin,
			auctions: Vec<T::AuctionId>,
			signature: SignatureOf<T>
		) -> Result {
			Ok(())
		}
		
		// Runs after every block.
		fn offchain_worker(now: <T as system::Trait>::BlockNumber) {
			// Only send messages if we are a potential validator.
			if runtime_io::is_validator() {
				Self::offchain(now);
			}
		}
	}
}

impl<T: Trait> Module<T> {
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

	// add an auction to pending vec, if it is not in there yet.
	// add by sunhao 20191024
	fn add2pendings(auction_id: T::AuctionId) {
		let mut pending_auctions = Self::pending_auctions();
		let mut flag = false;
		for elem in pending_auctions.iter() {
			if *elem == auction_id {
				flag = true;
				break;
			}
		}
		if !flag {
			pending_auctions.push(auction_id);
			<PendingAuctions<T>>::put(pending_auctions);
		}
	}

	// remove an auction from active vec.
	// add by sunhao 20191024
	fn remove_from_active(auction_id: T::AuctionId) {
		let mut active_auctions = Self::active_auctions();
		let mut index = active_auctions.len();
		for (i, elem) in active_auctions.iter().enumerate() {
			if *elem == auction_id {
				index = i;
				break;
			}
		}
		if index != active_auctions.len() {
			active_auctions.remove(index);
			<ActiveAuctions<T>>::put(active_auctions);
		}
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
		Self::add2pendings(auction_id);
			
		Ok(())
	}

	// real work for do_pause_auction
	// separated by Tang 20191024
	fn do_pause_auction(
		owner: &T::AccountId,
		auction_id: T::AuctionId
	) -> Result {
		// unwrap auction and ensure its status is Active
		let mut auction = Self::_ensure_auction_with_status(auction_id, Some(AuctionStatus::Active), Some(owner))?;

		// change status of auction
		auction.status = AuctionStatus::Paused;

		// save to storage
		<Auctions<T>>::insert(auction_id, auction);

		// emit event
		Self::deposit_event(RawEvent::AuctionUpdated(auction_id, 
			AuctionStatus::Active, AuctionStatus::Paused));

		Ok(())
	}

	// real work for do_resume_auction
	// separated by Tang 20191024
	fn do_resume_auction(
		owner: &T::AccountId,
		auction_id: T::AuctionId
	) -> Result {
		// unwrap auction and ensure its status is Paused
		let mut auction = Self::_ensure_auction_with_status(auction_id, Some(AuctionStatus::Paused), Some(owner))?;

		// change status of auction
		auction.status = AuctionStatus::Active;

		// save to storage
		<Auctions<T>>::insert(auction_id, auction);

		// emit event
		Self::deposit_event(RawEvent::AuctionUpdated(auction_id, 
			AuctionStatus::Paused, AuctionStatus::Active));

		Ok(())
	}

	// real work for stopping a auction.
	// added by sunhao 20191024
	// modified by Tang 20191024
	fn do_stop_auction(
		owner: &T::AccountId,
		auction_id: T::AuctionId
	) -> Result {
		// unwrap auction and ensure its status is not stopped yet.
		let mut auction = Self::_ensure_auction_with_status(auction_id, None, Some(owner))?;

		ensure!(auction.status != AuctionStatus::Stopped,
			"Auction can NOT be stopped now.");

		// call settle func if needed.
		if auction.status != AuctionStatus::PendingStart {
			Self::do_settle_auction(auction_id)?;
		}

		// change status of auction
		let old_status = auction.status;
		auction.status = AuctionStatus::Stopped;

		// save to storage
		<Auctions<T>>::insert(auction_id, auction);

		// remove from active vecs
		Self::remove_from_active(auction_id);
		
		// emit event
		Self::deposit_event(RawEvent::AuctionUpdated(auction_id, 
			old_status, AuctionStatus::Stopped));
		
		Ok(())
	}

	fn do_settle_auction(auction: T::AuctionId) -> Result {
		// TODO auction done stuffs
		Ok(())
	}

	fn do_lock_balance(account: &T::AccountId, price: BalanceOf<T>) -> Result {
		T::Currency::extend_lock(
				AUCTION_ID,
				account,
				price,
				<T as system::Trait>::BlockNumber::max_value(),
				WithdrawReasons::none());
		Ok(())
	}

	fn do_participate_auction(auction: &T::AuctionId, account: &T::AccountId, price: BalanceOf<T>) -> Result {
		<Auctions<T>>::mutate(auction, |a|{
				if let Some(auc) = a {
					auc.latest_participate = Option::Some((account.clone(), <aura::Module<T>>::last()));
				}
			});

		<AuctionBids<T>>::insert(
			auction,
			account,
			&price
		);

		let mut participants;
		if let Some(p) = Self::auction_participants(auction) {
			participants = p;
		} else {
			participants = Vec::<T::AccountId>::new();
		}

		if ! participants.contains(account) {
			participants.push(account.clone());
		}

		<AuctionParticipants<T>>::insert(auction, participants);

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
		let call = Call::<T>::start_auction_passive(auction_ids, signature);
		// TODO
		Ok(())
	}

	fn _send_auction_stop_tx(
		auction_ids: Vec<T::AuctionId>
	) -> result::Result<(), OffchainErr> {
		// TODO
		Ok(())
	}

	/// Returns own authority identifier iff it is part of the current authority
	/// set, otherwise this function returns None. The restriction might be
	/// softened in the future in case a consumer needs to learn own authority
	/// identifier.
	fn _authority_id() -> Option<T::AuthorityId> {
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

	/// Sign for unchecked transaction
	fn _sign_unchecked_payload(payload: &Vec<u8>) -> result::Result<SignatureOf<T>, OffchainErr> {
		let key = Self::_authority_id();
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
		// TODO
		InvalidTransaction::Call.into()
	}
}
