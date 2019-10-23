use support::dispatch::Result;

/// Means for interacting with transfering items between accounts
pub trait ItemTransfer<AccountId, ItemId> {
	/// Transfer item from one to one
	fn transfer_item(source: &AccountId, dest: &AccountId, item_id: ItemId) -> Result;
}
