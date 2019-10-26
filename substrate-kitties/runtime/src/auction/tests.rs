//! Tests for the module.
#![cfg(test)]

use crate::auction::mocks::*;
use support::{assert_ok};

#[test]
fn it_works_for_create_auction() {
  new_test_ext().execute_with(|| {
    assert_ok!(Auctions::create_auction(Origin::signed(1), 100, 1, None));
    assert_eq!(Auctions::next_auction_id(), 1);
  });
}

#[test]
fn it_works_for_tmp() {
  new_test_ext().execute_with(|| {
    next_block();
  });
}