use core::future::{pending, ready};

use hazard3_futures::block_on;
use hazard3_futures::select::{Either, select};

#[test]
fn selects_ready_future_through_hazard3_crate() {
    let first = block_on(select(ready(7), pending::<u8>()));
    assert!(matches!(first, Either::First(7)));

    let second = block_on(select(pending::<u8>(), ready(9)));
    assert!(matches!(second, Either::Second(9)));
}
