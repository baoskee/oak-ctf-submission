use common::flash_loan::{Config, FlashLoanState};
use cw_storage_plus::Item;

// #[cw_serde]
// pub struct Config {
//     pub owner: Addr,
//     pub proxy_addr: Option<Addr>,
// }

// #[cw_serde]
// pub struct FlashLoanState {
//     pub requested_amount: Option<Uint128>,
// }

pub const CONFIG: Item<Config> = Item::new("config");
pub const FLASH_LOAN: Item<FlashLoanState> = Item::new("flash_loan");
