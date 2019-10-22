//! Tests for the module.
#![cfg(test)]

use mock::{TheModule, new_test_ext};
use runtime_io::with_externalities;
use support::{assert_ok};
use system::RawOrigin;

#[test]
fn it_works_for_default_value() {
  with_externalities(&mut new_test_ext(), || {
    // Just a dummy test for the dummy funtion `do_something`
    // calling the `do_something` function with a value 42
    assert_ok!(TheModule::do_something(RawOrigin::Root.into(), 42));
    // asserting that the stored value is equal to what we stored
    assert_eq!(TheModule::something(), Some(42));
  });
}