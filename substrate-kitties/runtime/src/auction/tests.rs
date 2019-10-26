//! Tests for the module.
#![cfg(test)]
mod mocks{
// use mock::*;
use support::{assert_ok};

#[test]
fn it_works_for_default_value() {
  new_test_ext().execute_with(|| {
    // Just a dummy test for the dummy funtion `do_something`
    // calling the `do_something` function with a value 42
    assert_ok!(TheModule::do_something(Origin::signed(1), 42));
    // asserting that the stored value is equal to what we stored
    assert_eq!(TheModule::something(), Some(42));
  });
}
}
