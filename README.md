# Team Archer-in-zoo じ☆贰期①組☆じ

参赛题目：拍卖行，且具备一定的Offchain worker功能

## 数据结构定义 for polkadot.js.org Developer

```json
{
  "KittyIndex": "u32",
  "Kitty": "([u8; 16])",
  "KittyLinkedItem": {
    "prev": "Option<KittyIndex>",
    "next": "Option<KittyIndex>"
  },
  "ItemId": "u32",
  "AuctionId": "u32",
  "AuctionStatus": {
    "_enum": [
      "PendingStart",
      "Paused",
      "Active",
      "Stopped"
    ]
  },
  "Auction": {
    "id": "AuctionId",
    "item": "ItemId",
    "owner": "AccountId",
    "start_at": "Option<Moment>",
    "stop_at": "Option<Moment>",
    "wait_period": "Option<Moment>",
    "begin_price": "Balance",
    "upper_bound_price": "Option<Balance>",
    "minimum_step": "Balance",
    "latest_participate": "Option<(AccountId, Moment)>",
    "status": "AuctionStatus"
  }
}
```

## 其他模块如何使用拍卖行模块

任意一个模块只需要实现以下Trait即可使用拍卖行模块，将道具进行拍卖操作。

```rust
pub trait ItemTransfer<AccountId, ItemId> {
  /// Ensure item's owner
  fn is_item_owner(who: &AccountId, item_id: ItemId) -> bool;
  /// Transfer item from one to one
  fn transfer_item(source: &AccountId, dest: &AccountId, item_id: ItemId) -> Result;
}
```

## 操作拍卖的流程说明

> Step.1 创建拍卖场子（目前版本仅支持一场一件，但可扩展为一场多件）

此时仅创建一个全新Auctoin实例，为方便不同类型的拍卖方式，目前不做拍品添加。

```rust
pub fn create_auction(origin,
  begin_price: BalanceOf<T>,//起拍价
  minimum_step: BalanceOf<T>,//最小加价幅度
  upper_bound_price: Option<BalanceOf<T>>,//封顶价
);
```

> Step.2 添加拍品（目前仅支持一件，若重复调用将覆盖上一件）

添加拍品时，目前需要拍卖创建者必须为道具持有者。（通过ItemTransfer trait的is_item_owner方法判断。）

```rust
pub fn add_item(origin,
  auction_id: T::AuctionId,
  item: T::ItemId,//竞拍对象
);
```

> Step.3 设置拍卖参数（起拍时间，停拍时间，竞价等待时间）

在该方法中设置起拍和停拍时间，同时将加入pending start队列，等待开拍。

```rust
pub fn setup_moments(origin,
  auction_id: T::AuctionId,
  start_at: Option<T::Moment>,  //起拍时间
  stop_at: Option<T::Moment>,  //结束时间
  wait_period: Option<T::Moment>  //竞价等待时间
);
```

> Step.4 自动起拍(Offchain worker)

该方法由offchain worker调用，自动启动一批符合起拍条件的拍卖场次。

```rust
fn start_auctions_passive(
  origin,
  auction_ids: Vec<T::AuctionId>, // 需要开拍的场次id
  signature: SignatureOf<T>
);
```

> Step.5 拍卖控制功能（暂停拍卖，恢复拍卖，停止拍卖）

提供拍卖场次的owner, 多种拍卖控制的方法。

```rust
pub fn pause_auction(origin, auction_id: T::AuctionId);
pub fn resume_auction(origin, auction_id: T::AuctionId);
pub fn stop_auction(origin, auction_id: T::AuctionId);
```

> Step.6 参与竞拍（使用LockableCurrency进行锁仓）

该模块的`LockIdentifier = *b"auction "`，同时在Store中记录了用户对全部Auctions的累计锁仓额和对各个Auction的锁仓额。
对相同Auction，重复出价将仅保留最大出价额。

```rust
pub fn participate_auction(
  origin,
  auction_id: T::AuctionId,
  price: BalanceOf<T> // 出价金额
);
```

> Step.7 自动停拍(Offchain worker)

该方法由offchain worker调用，当达到停拍条件后将自动调用。被动的停拍条件见下文。

```rust
fn stop_auctions_passive(
  origin,
  auction_ids: Vec<T::AuctionId>,
  signature: SignatureOf<T>
);
```

> Step.8 多种停拍条件和拍卖结算

停拍条件：

- 【主动型】owner调用stop_auction
- 【被动型】当前区块时间达到stop_at(offchain worker检测触发)
- 【被动型】最后的出价额达到upper_bound_price(offchain worker检测触发)
- 【被动型】最后的出价时间与当前时间差值达到wait_period(offchain worker检测触发)

拍卖结算：

- 解锁全部参拍者在这次拍卖中锁定的资金
- 计算拍卖手续费
- 将中标者的竞拍额扣除手续费后转账给拍卖者
- 触发拍卖手续费on_unbalanced接口，将手续费imbalance输出到外部
- 将拍卖物转移给中标者
- 设置拍卖结束标志位
- 触发Event
