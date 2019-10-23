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
