#[cfg(test)]
mod tests {
    use aurora_sdk_integration_tests::aurora_engine::erc20::ERC20;
    use aurora_sdk_integration_tests::aurora_engine_types::parameters::engine::{
        FunctionCallArgsV2, SubmitResult, TransactionStatus,
    };
    use aurora_sdk_integration_tests::aurora_engine_types::types::WeiU256;
    use aurora_sdk_integration_tests::aurora_engine_types::H160;
    use aurora_sdk_integration_tests::wnear::Wnear;
    use aurora_sdk_integration_tests::workspaces::network::Sandbox;
    use aurora_sdk_integration_tests::workspaces::result::ExecutionFinalResult;
    use aurora_sdk_integration_tests::workspaces::{Account, Contract, Worker};
    use aurora_sdk_integration_tests::{
        aurora_engine::{self, AuroraEngine},
        aurora_engine_types::{
            parameters::engine::{CallArgs, FunctionCallArgsV1},
            types::{Address, Wei},
            U256,
        },
        ethabi, tokio,
        utils::{ethabi::DeployedContract, forge},
        wnear,
        workspaces::{self, AccountId},
    };
    use std::path::Path;

    const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;

    const ATTACHED_NEAR: u128 = 5 * ONE_NEAR;
    const NEAR_DEPOSIT: u128 = 2 * ONE_NEAR;
    // This deposit is required to subsidise the ONE_YOCTO deposit on call `ft_transfer_call`
    const ATTACHED_NEAR_TO_INIT_CONTRACT: u128 = 10 * ONE_NEAR;

    const TRANSFER_TOKENS_AMOUNT: u64 = 100;
    const TOKEN_SUPPLY: u64 = 1000000000;

    const NEP141_STORAGE_DEPOSIT: u128 = (0.0125 * ONE_NEAR as f64) as u128;

    struct TestsInfrastructure {
        worker: Worker<Sandbox>,
        engine: AuroraEngine,
        silo: AuroraEngine,
        engine_wnear: Wnear,
        silo_wnear: Wnear,
        user_account: Account,
        user_address: Address,
        engine_silo_to_silo_contract: DeployedContract,
        silo_silo_to_silo_contract: DeployedContract,
        mock_token: Contract,
        engine_mock_token: ERC20,
        silo_mock_token: ERC20,
    }

    impl TestsInfrastructure {
        pub async fn init(storage_deposit: Option<u128>) -> Self {
            let worker = workspaces::sandbox().await.unwrap();
            let engine = aurora_engine::deploy_latest_silo(&worker, "aurora.test.near")
                .await
                .unwrap();
            let silo = aurora_engine::deploy_latest_silo(&worker, "silo.test.near")
                .await
                .unwrap();

            let engine_wnear = wnear::Wnear::deploy(&worker, &engine).await.unwrap();
            let silo_wnear = wnear::Wnear::deploy(&worker, &silo).await.unwrap();
            let user_account = worker.dev_create_account().await.unwrap();
            let user_address =
                aurora_sdk_integration_tests::aurora_engine_sdk::types::near_account_to_evm_address(
                    user_account.id().as_bytes(),
                );

            let engine_silo_to_silo_contract =
                deploy_silo_to_silo_sol_contract(&engine, &user_account, &engine_wnear).await;

            let mock_token = deploy_mock_token(&worker, user_account.id(), storage_deposit).await;
            let engine_mock_token = engine.bridge_nep141(mock_token.id()).await.unwrap();
            let silo_mock_token = silo.bridge_nep141(mock_token.id()).await.unwrap();

            let silo_silo_to_silo_contract =
                deploy_silo_to_silo_sol_contract(&silo, &user_account, &silo_wnear).await;

            TestsInfrastructure {
                worker: worker,
                engine,
                silo,
                engine_wnear,
                silo_wnear,
                user_account,
                user_address,
                engine_silo_to_silo_contract,
                silo_silo_to_silo_contract,
                mock_token,
                engine_mock_token,
                silo_mock_token,
            }
        }

        pub async fn mint_wnear_engine(&self, user_address: Option<Address>) {
            self.engine
                .mint_wnear(
                    &self.engine_wnear,
                    user_address.unwrap_or(self.user_address),
                    2 * (ATTACHED_NEAR + NEAR_DEPOSIT),
                )
                .await
                .unwrap();
        }

        pub async fn mint_wnear_silo(&self) {
            self.silo
                .mint_wnear(
                    &self.silo_wnear,
                    self.user_address,
                    2 * (ATTACHED_NEAR + NEAR_DEPOSIT),
                )
                .await
                .unwrap();
        }

        pub async fn mint_eth_engine(&self, user_address: Option<Address>, amount: u64) {
            self.engine
                .mint_account(
                    user_address.unwrap_or(self.user_address.clone()),
                    0,
                    Wei::new_u64(amount),
                )
                .await
                .unwrap();
        }

        pub async fn approve_spend_wnear_engine(&self, user_account: Option<Account>) {
            approve_spend_tokens(
                &self.engine_wnear.aurora_token,
                self.engine_silo_to_silo_contract.address,
                &user_account.unwrap_or(self.user_account.clone()),
                &self.engine,
            )
            .await;
        }

        pub async fn silo_to_silo_register_token_engine(
            &self,
            user_account: Option<Account>,
            check_result: bool,
        ) {
            silo_to_silo_register_token(
                &self.engine_silo_to_silo_contract,
                self.engine_mock_token.address.raw(),
                &user_account.unwrap_or(self.user_account.clone()),
                &self.engine,
                check_result,
            )
            .await;
        }

        pub async fn silo_to_silo_register_token_silo(&self) {
            silo_to_silo_register_token(
                &self.silo_silo_to_silo_contract,
                self.silo_mock_token.address.raw(),
                &self.user_account,
                &self.silo,
                true,
            )
            .await;
        }

        pub async fn check_token_is_regester_engine(&self, expected_result: bool) {
            check_token_account_id(
                &self.engine_silo_to_silo_contract,
                self.engine_mock_token.address.raw(),
                self.mock_token.id().to_string(),
                &self.user_account,
                &self.engine,
                expected_result,
            )
            .await;
        }

        pub async fn approve_spend_mock_tokens_engine(&self) {
            approve_spend_tokens(
                &self.engine_mock_token,
                self.engine_silo_to_silo_contract.address,
                &self.user_account,
                &self.engine,
            )
            .await;
        }

        pub async fn get_mock_token_balance_engine(&self) -> U256 {
            self.engine
                .erc20_balance_of(&self.engine_mock_token, self.user_address)
                .await
                .unwrap()
        }

        pub async fn get_mock_token_balance_silo(&self) -> U256 {
            self.silo
                .erc20_balance_of(&self.silo_mock_token, self.user_address)
                .await
                .unwrap()
        }

        pub async fn engine_to_silo_transfer(&self, check_output: bool) {
            silo_to_silo_transfer(
                &self.engine_silo_to_silo_contract,
                &self.engine_mock_token,
                self.engine.inner.id(),
                self.silo.inner.id(),
                self.user_account.clone(),
                self.user_address.encode(),
                check_output,
            )
            .await;
        }

        pub async fn silo_to_engine_transfer(&self) {
            silo_to_silo_transfer(
                &self.silo_silo_to_silo_contract,
                &self.silo_mock_token,
                self.silo.inner.id(),
                self.engine.inner.id(),
                self.user_account.clone(),
                self.user_address.encode(),
                true,
            )
            .await;
        }

        pub async fn approve_spend_tokens_silo(&self) {
            approve_spend_tokens(
                &self.silo_mock_token,
                self.silo_silo_to_silo_contract.address,
                &self.user_account,
                &self.silo,
            )
            .await;
        }

        pub async fn check_token_account_id_silo(&self) {
            check_token_account_id(
                &self.silo_silo_to_silo_contract,
                self.silo_mock_token.address.raw(),
                self.mock_token.id().to_string(),
                &self.user_account,
                &self.silo,
                true,
            )
            .await;
        }

        pub async fn approve_spend_wnear_silo(&self) {
            approve_spend_tokens(
                &self.silo_wnear.aurora_token,
                self.silo_silo_to_silo_contract.address,
                &self.user_account,
                &self.silo,
            )
            .await;
        }

        pub async fn check_user_balance_engine(&self, expected_value: u64) {
            check_get_user_balance(
                &self.engine_silo_to_silo_contract,
                &self.user_account,
                self.engine_mock_token.address.raw(),
                self.user_address.raw(),
                &self.engine,
                expected_value,
            )
            .await;
        }

        pub async fn call_ft_transfer_call_callback_engine(&self, user_account: Account) {
            let nonce = 0;
            let contract_args = self
                .engine_silo_to_silo_contract
                .create_call_method_bytes_with_args(
                    "ftTransferCallCallback",
                    &[
                        ethabi::Token::Uint(U256::from(nonce)),
                        ethabi::Token::Address(self.user_address.raw()),
                        ethabi::Token::Address(self.engine_mock_token.address.raw()),
                        ethabi::Token::Uint(U256::from(TRANSFER_TOKENS_AMOUNT)),
                        ethabi::Token::String("receiverId".into()),
                    ],
                );

            call_aurora_contract(
                self.engine_silo_to_silo_contract.address,
                contract_args,
                &user_account,
                self.engine.inner.id(),
                0,
                true,
            )
            .await
            .unwrap();
        }
    }

    #[tokio::test]
    async fn test_ft_transfer_to_silo() {
        let infra = TestsInfrastructure::init(None).await;

        mint_tokens_near(&infra.mock_token, infra.engine.inner.id()).await;

        infra.mint_wnear_engine(None).await;
        infra.approve_spend_wnear_engine(None).await;

        infra.silo_to_silo_register_token_engine(None, true).await;
        infra.check_token_is_regester_engine(true).await;
        check_near_account_id(
            &infra.engine_silo_to_silo_contract,
            &infra.user_account,
            &infra.engine,
        )
        .await;

        storage_deposit(&infra.mock_token, infra.engine.inner.id(), None).await;
        storage_deposit(&infra.mock_token, infra.silo.inner.id(), None).await;

        engine_mint_tokens(infra.user_address, &infra.engine_mock_token, &infra.engine).await;
        infra.approve_spend_mock_tokens_engine().await;

        let balance_engine_before = infra.get_mock_token_balance_engine().await;
        infra.engine_to_silo_transfer(true).await;

        let balance_engine_after = infra.get_mock_token_balance_engine().await;
        assert_eq!(
            (balance_engine_before - balance_engine_after).as_u64(),
            TRANSFER_TOKENS_AMOUNT
        );

        let balance_silo = infra.get_mock_token_balance_silo().await;
        assert_eq!(balance_silo.as_u64(), TRANSFER_TOKENS_AMOUNT);

        // Transfer from silo back to aurora
        infra.mint_wnear_silo().await;
        infra.approve_spend_wnear_silo().await;

        infra.silo_to_silo_register_token_silo().await;
        infra.check_token_account_id_silo().await;

        infra.approve_spend_tokens_silo().await;
        infra.silo_to_engine_transfer().await;

        let balance_engine_after_silo = infra.get_mock_token_balance_engine().await;
        assert_eq!(
            (balance_engine_after_silo - balance_engine_after).as_u64(),
            TRANSFER_TOKENS_AMOUNT
        );

        let balance_silo = infra.get_mock_token_balance_silo().await;
        assert_eq!(balance_silo.as_u64(), 0);
    }

    #[tokio::test]
    async fn test_withdraw() {
        let infra = TestsInfrastructure::init(None).await;
        infra.mint_wnear_engine(None).await;
        infra.approve_spend_wnear_engine(None).await;

        mint_tokens_near(&infra.mock_token, infra.engine.inner.id()).await;

        infra.silo_to_silo_register_token_engine(None, true).await;
        infra.check_token_is_regester_engine(true).await;
        check_near_account_id(
            &infra.engine_silo_to_silo_contract,
            &infra.user_account,
            &infra.engine,
        )
        .await;

        storage_deposit(&infra.mock_token, infra.engine.inner.id(), None).await;

        engine_mint_tokens(infra.user_address, &infra.engine_mock_token, &infra.engine).await;
        infra.approve_spend_mock_tokens_engine().await;

        let balance_engine_before = infra.get_mock_token_balance_engine().await;
        infra.engine_to_silo_transfer(false).await;

        let balance_engine_after = infra.get_mock_token_balance_engine().await;

        assert_eq!(
            (balance_engine_before - balance_engine_after).as_u64(),
            TRANSFER_TOKENS_AMOUNT
        );

        let balance_silo = infra.get_mock_token_balance_silo().await;
        assert_eq!(balance_silo.as_u64(), 0);

        infra.check_user_balance_engine(0).await;
        withdraw(
            &infra.engine_silo_to_silo_contract,
            infra.engine_mock_token.address.raw(),
            infra.engine.inner.id(),
            infra.user_account.clone(),
        )
        .await;

        let balance_engine_after_withdraw = infra.get_mock_token_balance_engine().await;
        assert_eq!(balance_engine_after_withdraw, 0.into());

        infra.check_user_balance_engine(0).await;
    }

    #[tokio::test]
    async fn check_access_control() {
        let infra = TestsInfrastructure::init(None).await;
        //create new user
        let regular_user_account = infra.worker.dev_create_account().await.unwrap();

        //error on call ftTransferCallCallback by regular user
        infra
            .call_ft_transfer_call_callback_engine(regular_user_account.clone())
            .await;
        infra.check_user_balance_engine(0).await;

        //error on call ftTransferCallCallback by admin
        infra
            .call_ft_transfer_call_callback_engine(infra.user_account.clone())
            .await;
        infra.check_user_balance_engine(0).await;

        //error on call ftTransferCallCallback by aurora
        infra
            .call_ft_transfer_call_callback_engine(infra.engine.inner.as_account().clone())
            .await;
        infra.check_user_balance_engine(0).await;
    }

    #[tokio::test]
    async fn transfer_not_register_tokens() {
        let infra = TestsInfrastructure::init(None).await;

        mint_tokens_near(&infra.mock_token, infra.engine.inner.id()).await;
        infra.mint_wnear_engine(None).await;
        infra.approve_spend_wnear_engine(None).await;

        infra.check_token_is_regester_engine(false).await;

        storage_deposit(&infra.mock_token, infra.engine.inner.id(), None).await;
        storage_deposit(&infra.mock_token, infra.silo.inner.id(), None).await;

        engine_mint_tokens(infra.user_address, &infra.engine_mock_token, &infra.engine).await;
        infra.approve_spend_mock_tokens_engine().await;

        let balance_engine_before = infra.get_mock_token_balance_engine().await;

        infra.engine_to_silo_transfer(true).await;

        let balance_engine_after = infra.get_mock_token_balance_engine().await;
        assert_eq!(balance_engine_before, balance_engine_after);
    }

    #[tokio::test]
    async fn storage_deposit_outside_contract_test() {
        let infra = TestsInfrastructure::init(None).await;

        mint_tokens_near(&infra.mock_token, infra.engine.inner.id()).await;

        infra.mint_wnear_engine(None).await;
        infra.approve_spend_wnear_engine(None).await;

        let implicit_account =
            infra.engine_silo_to_silo_contract.address.encode() + "." + infra.engine.inner.id();

        storage_deposit(&infra.mock_token, infra.engine.inner.id(), None).await;
        storage_deposit(&infra.mock_token, infra.silo.inner.id(), None).await;
        storage_deposit(&infra.mock_token, &implicit_account, None).await;

        infra.silo_to_silo_register_token_engine(None, true).await;

        engine_mint_tokens(infra.user_address, &infra.engine_mock_token, &infra.engine).await;
        infra.approve_spend_mock_tokens_engine().await;

        let balance_engine_before = infra.get_mock_token_balance_engine().await;
        infra.engine_to_silo_transfer(true).await;

        let balance_engine_after = infra.get_mock_token_balance_engine().await;
        assert_eq!(
            (balance_engine_before - balance_engine_after).as_u64(),
            TRANSFER_TOKENS_AMOUNT
        );

        let balance_silo = infra.get_mock_token_balance_silo().await;
        assert_eq!(balance_silo.as_u64(), TRANSFER_TOKENS_AMOUNT);
    }

    #[tokio::test]
    async fn long_msg_test() {
        let infra = TestsInfrastructure::init(None).await;

        mint_tokens_near(&infra.mock_token, infra.engine.inner.id()).await;
        infra.mint_wnear_engine(None).await;
        infra.approve_spend_wnear_engine(None).await;
        storage_deposit(&infra.mock_token, infra.engine.inner.id(), None).await;
        storage_deposit(&infra.mock_token, infra.silo.inner.id(), None).await;
        infra.silo_to_silo_register_token_engine(None, true).await;

        let mut msg_len = 1;
        let mut step = 1;

        for i in 0..17 {
            let mut msg = String::with_capacity(msg_len);
            for _ in 0..msg_len {
                msg.push('?');
            }

            println!("{}", msg_len);

            engine_mint_tokens(infra.user_address, &infra.engine_mock_token, &infra.engine).await;

            let balance_engine_before = infra.get_mock_token_balance_engine().await;
            assert_eq!(balance_engine_before.as_u64(), TRANSFER_TOKENS_AMOUNT);

            infra.approve_spend_mock_tokens_engine().await;

            let contract_args = infra
                .engine_silo_to_silo_contract
                .create_call_method_bytes_with_args(
                    "ftTransferCallToNear",
                    &[
                        ethabi::Token::Address(infra.engine_mock_token.address.raw()),
                        ethabi::Token::Uint(U256::from(TRANSFER_TOKENS_AMOUNT)),
                        ethabi::Token::String(infra.silo.inner.id().to_string()),
                        ethabi::Token::String(msg),
                    ],
                );

            let _ = call_aurora_contract(
                infra.engine_silo_to_silo_contract.address,
                contract_args,
                &infra.user_account,
                infra.engine.inner.id(),
                0,
                false,
            )
            .await;

            let balance_engine_after = infra.get_mock_token_balance_engine().await;
            if balance_engine_after.as_u64() == 0 {
                let balance_silo = infra.get_mock_token_balance_silo().await;
                assert_eq!(balance_silo.as_u64(), 0);
                infra
                    .check_user_balance_engine((i + 1) * TRANSFER_TOKENS_AMOUNT)
                    .await;
                step = step * 2;
                msg_len += step;
            } else {
                assert_eq!(balance_engine_after.as_u64(), TRANSFER_TOKENS_AMOUNT);
                let balance_silo = infra.get_mock_token_balance_silo().await;
                assert_eq!(balance_silo.as_u64(), 0);
                infra
                    .check_user_balance_engine(i * TRANSFER_TOKENS_AMOUNT)
                    .await;
                break;
            }
        }
    }

    #[tokio::test]
    async fn transfer_eth_test() {
        let infra = TestsInfrastructure::init(None).await;

        mint_tokens_near(&infra.mock_token, infra.engine.inner.id()).await;
        infra.mint_wnear_engine(None).await;
        infra
            .mint_wnear_engine(Some(infra.engine_silo_to_silo_contract.address))
            .await;
        infra.approve_spend_wnear_engine(None).await;

        infra.mint_eth_engine(None, 200).await;
        let silo_eth_token = infra
            .silo
            .bridge_nep141(infra.engine.inner.id())
            .await
            .unwrap();

        storage_deposit(&infra.engine.inner, infra.engine.inner.id(), None).await;
        storage_deposit(&infra.engine.inner, infra.silo.inner.id(), None).await;

        let contract_args = infra
            .engine_silo_to_silo_contract
            .create_call_method_bytes_with_args(
                "storageDeposit",
                &[
                    ethabi::Token::Address(
                        "0x0000000000000000000000000000000000000000"
                            .parse()
                            .unwrap(),
                    ),
                    ethabi::Token::Uint(NEP141_STORAGE_DEPOSIT.into()),
                ],
            );

        call_aurora_contract(
            infra.engine_silo_to_silo_contract.address,
            contract_args,
            &infra.user_account,
            infra.engine.inner.id(),
            0,
            true,
        )
        .await
        .unwrap();

        let user_balance_before = infra
            .engine
            .get_balance(infra.user_address)
            .await
            .unwrap()
            .raw()
            .as_u64();
        assert_eq!(user_balance_before, 200);

        let user_balance_before_silo = infra
            .silo
            .erc20_balance_of(&silo_eth_token, infra.user_address)
            .await
            .unwrap();

        assert_eq!(user_balance_before_silo.as_u64(), 0);

        let contract_args = infra
            .engine_silo_to_silo_contract
            .create_call_method_bytes_with_args(
                "ftTransferCallToNear",
                &[
                    ethabi::Token::Address(
                        "0x0000000000000000000000000000000000000000"
                            .parse()
                            .unwrap(),
                    ),
                    ethabi::Token::Uint(U256::from(200)),
                    ethabi::Token::String(infra.silo.inner.id().to_string()),
                    ethabi::Token::String(infra.user_address.encode()),
                ],
            );

        call_aurora_contract(
            infra.engine_silo_to_silo_contract.address,
            contract_args,
            &infra.user_account,
            infra.engine.inner.id(),
            200,
            true,
        )
        .await
        .unwrap();

        let user_balance_after = infra
            .engine
            .get_balance(infra.user_address)
            .await
            .unwrap()
            .raw()
            .as_u64();
        assert_eq!(user_balance_after, 0);

        let user_balance_after_silo = infra
            .silo
            .erc20_balance_of(&silo_eth_token, infra.user_address)
            .await
            .unwrap();

        assert_eq!(user_balance_after_silo.as_u64(), 200);

        // Transfer from silo back to aurora
        infra.mint_wnear_silo().await;
        infra.approve_spend_wnear_silo().await;

        silo_to_silo_register_token(
            &infra.silo_silo_to_silo_contract,
            silo_eth_token.address.raw(),
            &infra.user_account,
            &infra.silo,
            true,
        )
        .await;

        approve_spend_tokens(
            &silo_eth_token,
            infra.silo_silo_to_silo_contract.address,
            &infra.user_account,
            &infra.silo,
        )
        .await;

        infra.silo_to_engine_transfer().await;

        let contract_args = infra
            .silo_silo_to_silo_contract
            .create_call_method_bytes_with_args(
                "ftTransferCallToNear",
                &[
                    ethabi::Token::Address(silo_eth_token.address.raw()),
                    ethabi::Token::Uint(U256::from(200)),
                    ethabi::Token::String(infra.engine.inner.id().to_string()),
                    ethabi::Token::String(
                        "fake.near:0000000000000000000000000000000000000000000000000000000000000000"
                            .to_string() + &infra.user_address.encode(),
                    ),
                ],
            );

        call_aurora_contract(
            infra.silo_silo_to_silo_contract.address,
            contract_args,
            &infra.user_account,
            infra.silo.inner.id(),
            0,
            true,
        )
        .await
        .unwrap();

        let user_balance_after_back_transfer = infra
            .engine
            .get_balance(infra.user_address)
            .await
            .unwrap()
            .raw()
            .as_u64();
        assert_eq!(user_balance_after_back_transfer, 200);

        let user_balance_after_back_transfer_silo = infra
            .silo
            .erc20_balance_of(&silo_eth_token, infra.user_address)
            .await
            .unwrap();

        assert_eq!(user_balance_after_back_transfer_silo.as_u64(), 0);
    }

    #[tokio::test]
    async fn eth_withdraw_test() {
        let infra = TestsInfrastructure::init(None).await;

        mint_tokens_near(&infra.mock_token, infra.engine.inner.id()).await;
        infra.mint_wnear_engine(None).await;
        infra
            .mint_wnear_engine(Some(infra.engine_silo_to_silo_contract.address))
            .await;
        infra.approve_spend_wnear_engine(None).await;

        infra.mint_eth_engine(None, 200).await;

        let contract_args = infra
            .engine_silo_to_silo_contract
            .create_call_method_bytes_with_args(
                "storageDeposit",
                &[
                    ethabi::Token::Address(
                        "0x0000000000000000000000000000000000000000"
                            .parse()
                            .unwrap(),
                    ),
                    ethabi::Token::Uint(NEP141_STORAGE_DEPOSIT.into()),
                ],
            );

        call_aurora_contract(
            infra.engine_silo_to_silo_contract.address,
            contract_args,
            &infra.user_account,
            infra.engine.inner.id(),
            0,
            true,
        )
        .await
        .unwrap();

        let user_balance_before = infra
            .engine
            .get_balance(infra.user_address)
            .await
            .unwrap()
            .raw()
            .as_u64();
        assert_eq!(user_balance_before, 200);

        let contract_args = infra
            .engine_silo_to_silo_contract
            .create_call_method_bytes_with_args(
                "ftTransferCallToNear",
                &[
                    ethabi::Token::Address(
                        "0x0000000000000000000000000000000000000000"
                            .parse()
                            .unwrap(),
                    ),
                    ethabi::Token::Uint(U256::from(200)),
                    ethabi::Token::String(infra.silo.inner.id().to_string()),
                    ethabi::Token::String(infra.user_address.encode()),
                ],
            );

        call_aurora_contract(
            infra.engine_silo_to_silo_contract.address,
            contract_args,
            &infra.user_account,
            infra.engine.inner.id(),
            200,
            true,
        )
        .await
        .unwrap();

        let user_balance_after = infra
            .engine
            .get_balance(infra.user_address)
            .await
            .unwrap()
            .raw()
            .as_u64();
        assert_eq!(user_balance_after, 0);

        check_get_user_balance(
            &infra.engine_silo_to_silo_contract,
            &infra.user_account,
            "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            infra.user_address.raw(),
            &infra.engine,
            200,
        )
        .await;

        withdraw(
            &infra.engine_silo_to_silo_contract,
            "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            infra.engine.inner.id(),
            infra.user_account.clone(),
        )
        .await;

        let balance_engine_after_withdraw = infra
            .engine
            .get_balance(infra.user_address)
            .await
            .unwrap()
            .raw()
            .as_u64();
        assert_eq!(balance_engine_after_withdraw, 200);

        check_get_user_balance(
            &infra.engine_silo_to_silo_contract,
            &infra.user_account,
            "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            infra.user_address.raw(),
            &infra.engine,
            0,
        )
        .await;
    }

    #[tokio::test]
    async fn test_ft_transfer_to_silo_with_incorrect_deposit() {
        let infra = TestsInfrastructure::init(None).await;

        mint_tokens_near(&infra.mock_token, infra.engine.inner.id()).await;

        infra.mint_wnear_engine(None).await;
        infra.approve_spend_wnear_engine(None).await;

        infra.silo_to_silo_register_token_engine(None, true).await;
        infra.check_token_is_regester_engine(true).await;
        check_near_account_id(
            &infra.engine_silo_to_silo_contract,
            &infra.user_account,
            &infra.engine,
        )
        .await;

        storage_deposit(&infra.mock_token, infra.engine.inner.id(), None).await;
        storage_deposit(&infra.mock_token, infra.silo.inner.id(), None).await;

        engine_mint_tokens(infra.user_address, &infra.engine_mock_token, &infra.engine).await;
        infra.approve_spend_mock_tokens_engine().await;
        infra.mint_eth_engine(None, TRANSFER_TOKENS_AMOUNT).await;

        let balance_engine_before = infra.get_mock_token_balance_engine().await;

        let mut user_eth_balance = infra
            .engine
            .get_balance(infra.user_address)
            .await
            .unwrap()
            .raw()
            .as_u64();
        assert_eq!(user_eth_balance, TRANSFER_TOKENS_AMOUNT);

        let contract_args = infra
            .engine_silo_to_silo_contract
            .create_call_method_bytes_with_args(
                "ftTransferCallToNear",
                &[
                    ethabi::Token::Address(infra.engine_mock_token.address.raw()),
                    ethabi::Token::Uint(U256::from(TRANSFER_TOKENS_AMOUNT)),
                    ethabi::Token::String(infra.silo.inner.id().to_string()),
                    ethabi::Token::String(infra.user_address.encode()),
                ],
            );

        call_aurora_contract(
            infra.engine_silo_to_silo_contract.address,
            contract_args,
            &infra.user_account,
            infra.engine.inner.id(),
            TRANSFER_TOKENS_AMOUNT as u128,
            true,
        )
        .await
        .unwrap();

        user_eth_balance = infra
            .engine
            .get_balance(infra.user_address)
            .await
            .unwrap()
            .raw()
            .as_u64();
        assert_eq!(user_eth_balance, TRANSFER_TOKENS_AMOUNT);

        let balance_engine_after = infra.get_mock_token_balance_engine().await;
        assert_eq!(balance_engine_before, balance_engine_after);
        let balance_silo = infra.get_mock_token_balance_silo().await;
        assert_eq!(balance_silo.as_u64(), 0);
    }

    #[ignore]
    #[tokio::test]
    async fn error_on_withdraw_to_near() {
        let deposit_value = Some(10_000_000_000_000_000_000_000_000u128);
        let infra = TestsInfrastructure::init(deposit_value).await;

        mint_tokens_near(&infra.mock_token, infra.engine.inner.id()).await;
        infra.mint_wnear_engine(None).await;
        infra.approve_spend_wnear_engine(None).await;

        infra.silo_to_silo_register_token_engine(None, false).await;
        infra.check_token_is_regester_engine(true).await;

        storage_deposit(&infra.mock_token, infra.engine.inner.id(), deposit_value).await;
        storage_deposit(&infra.mock_token, infra.silo.inner.id(), deposit_value).await;

        engine_mint_tokens(infra.user_address, &infra.engine_mock_token, &infra.engine).await;
        infra.approve_spend_mock_tokens_engine().await;

        let balance_engine_before = infra.get_mock_token_balance_engine().await;

        infra.engine_to_silo_transfer(false).await;

        let balance_engine_after = infra.get_mock_token_balance_engine().await;

        assert_eq!(
            (balance_engine_before - balance_engine_after).as_u64(),
            TRANSFER_TOKENS_AMOUNT
        );

        let balance_silo = infra.get_mock_token_balance_silo().await;
        assert_eq!(balance_silo.as_u64(), 0);

        infra
            .check_user_balance_engine(TRANSFER_TOKENS_AMOUNT)
            .await;

        storage_deposit(
            &infra.mock_token,
            &(infra.engine_silo_to_silo_contract.address.encode()
                + "."
                + &infra.engine.inner.id().to_string()),
            deposit_value,
        )
        .await;
        withdraw(
            &infra.engine_silo_to_silo_contract,
            infra.engine_mock_token.address.raw(),
            infra.engine.inner.id(),
            infra.user_account.clone(),
        )
        .await;

        let balance_engine_after_withdraw = infra.get_mock_token_balance_engine().await;
        assert_eq!(balance_engine_before, balance_engine_after_withdraw);

        infra.check_user_balance_engine(0).await;
    }

    async fn deploy_silo_to_silo_sol_contract(
        engine: &AuroraEngine,
        user_account: &workspaces::Account,
        wnear: &Wnear,
    ) -> DeployedContract {
        let aurora_sdk_path = Path::new("./aurora-contracts-sdk/aurora-solidity-sdk");
        assert!(
            aurora_sdk_path.exists(),
            "The path isn't exist: {}",
            aurora_sdk_path.to_string_lossy()
        );

        let codec_lib = forge::deploy_codec_lib(&aurora_sdk_path, engine)
            .await
            .unwrap();
        let utils_lib = forge::deploy_utils_lib(&aurora_sdk_path, engine)
            .await
            .unwrap();
        let aurora_sdk_lib =
            forge::deploy_aurora_sdk_lib(&aurora_sdk_path, engine, codec_lib, utils_lib)
                .await
                .unwrap();

        let contract_path = Path::new("../contracts");
        let constructor = forge::forge_build(
            contract_path,
            &[
                format!(
                    "@auroraisnear/aurora-sdk/aurora-sdk/AuroraSdk.sol:AuroraSdk:0x{}",
                    aurora_sdk_lib.encode()
                ),
                format!(
                    "@auroraisnear/aurora-sdk/aurora-sdk/Utils.sol:Utils:0x{}",
                    utils_lib.encode()
                ),
            ],
            &["out", "SiloToSilo.sol", "SiloToSilo.json"],
        )
        .await
        .unwrap();

        let deploy_bytes = constructor.create_deploy_bytes_without_constructor();
        let address = engine
            .deploy_evm_contract_with(user_account, deploy_bytes)
            .await
            .unwrap();

        let contract_impl = constructor.deployed_at(address);
        let contract_args = contract_impl.create_call_method_bytes_with_args(
            "initialize",
            &[
                ethabi::Token::Address(wnear.aurora_token.address.raw()),
                ethabi::Token::String(engine.inner.id().to_string()),
                ethabi::Token::String(engine.inner.id().to_string()),
            ],
        );
        call_aurora_contract(
            contract_impl.address,
            contract_args,
            &user_account,
            engine.inner.id(),
            0,
            true,
        )
        .await
        .unwrap();

        engine
            .mint_wnear(&wnear, address, ATTACHED_NEAR_TO_INIT_CONTRACT)
            .await
            .unwrap();

        contract_impl
    }

    async fn deploy_mock_token(
        worker: &workspaces::Worker<workspaces::network::Sandbox>,
        user_account_id: &str,
        storage_deposit: Option<u128>,
    ) -> workspaces::Contract {
        let artifact_path =
            Path::new("./mock_token/target/wasm32-unknown-unknown/release/mock_token.wasm");
        let wasm_bytes = tokio::fs::read(artifact_path).await.unwrap();
        let mock_token = worker.dev_deploy(&wasm_bytes).await.unwrap();

        mock_token
            .call("new_default_meta")
            .args_json(serde_json::json!({"owner_id": user_account_id, "name": "MockToken", "symbol": "MCT", "total_supply": format!("{}", TOKEN_SUPPLY), "storage_deposit": storage_deposit.map(|x| format!("{}", x))}))
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();

        mock_token
    }

    async fn mint_tokens_near(token_contract: &Contract, receiver_id: &str) {
        token_contract
            .call("mint")
            .args_json(serde_json::json!({
                "account_id": receiver_id,
                "amount": format!("{}", TOKEN_SUPPLY)
            }))
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();
    }

    async fn approve_spend_tokens(
        token_contract: &ERC20,
        spender_address: Address,
        user_account: &Account,
        engine: &AuroraEngine,
    ) {
        let evm_input = token_contract.create_approve_call_bytes(spender_address, U256::MAX);
        let result = engine
            .call_evm_contract_with(user_account, token_contract.address, evm_input, Wei::zero())
            .await
            .unwrap();
        aurora_engine::unwrap_success(result.status).unwrap();
    }

    async fn silo_to_silo_register_token(
        silo_to_silo_contract: &DeployedContract,
        engine_mock_token_address: H160,
        user_account: &Account,
        engine: &AuroraEngine,
        check_result: bool,
    ) {
        let contract_args = silo_to_silo_contract.create_call_method_bytes_with_args(
            "registerToken",
            &[ethabi::Token::Address(engine_mock_token_address)],
        );

        call_aurora_contract(
            silo_to_silo_contract.address,
            contract_args,
            user_account,
            engine.inner.id(),
            0,
            check_result,
        )
        .await
        .unwrap();

        let contract_args = silo_to_silo_contract.create_call_method_bytes_with_args(
            "storageDeposit",
            &[
                ethabi::Token::Address(engine_mock_token_address),
                ethabi::Token::Uint(NEP141_STORAGE_DEPOSIT.into()),
            ],
        );

        call_aurora_contract(
            silo_to_silo_contract.address,
            contract_args,
            user_account,
            engine.inner.id(),
            0,
            check_result,
        )
        .await
        .unwrap();
    }

    async fn check_token_account_id(
        silo_to_silo_contract: &DeployedContract,
        engine_mock_token_address: H160,
        near_mock_token_account_id: String,
        user_account: &Account,
        engine: &AuroraEngine,
        expected_result: bool,
    ) {
        let contract_args = silo_to_silo_contract.create_call_method_bytes_with_args(
            "getTokenAccountId",
            &[ethabi::Token::Address(engine_mock_token_address)],
        );

        let outcome = call_aurora_contract(
            silo_to_silo_contract.address,
            contract_args,
            user_account,
            engine.inner.id(),
            0,
            true,
        )
        .await;

        let result = outcome.unwrap().borsh::<SubmitResult>().unwrap();
        if let TransactionStatus::Succeed(res) = result.status {
            let str_res = std::str::from_utf8(&res).unwrap();
            assert_eq!(
                str_res.contains(&near_mock_token_account_id),
                expected_result
            );
        }
    }

    async fn check_near_account_id(
        silo_to_silo_contract: &DeployedContract,
        user_account: &Account,
        engine: &AuroraEngine,
    ) {
        let contract_args = silo_to_silo_contract
            .create_call_method_bytes_with_args("getImplicitNearAccountIdForSelf", &[]);

        let outcome = call_aurora_contract(
            silo_to_silo_contract.address,
            contract_args,
            user_account,
            engine.inner.id(),
            0,
            true,
        )
        .await;

        let result = outcome.unwrap().borsh::<SubmitResult>().unwrap();
        if let TransactionStatus::Succeed(res) = result.status {
            let str_res = std::str::from_utf8(&res).unwrap();
            assert!(str_res.contains(&silo_to_silo_contract.address.encode()));
        }
    }

    async fn check_get_user_balance(
        silo_to_silo_contract: &DeployedContract,
        user_account: &Account,
        engine_mock_token_address: H160,
        user_address: H160,
        engine: &AuroraEngine,
        expected_value: u64,
    ) {
        let contract_args = silo_to_silo_contract.create_call_method_bytes_with_args(
            "getUserBalance",
            &[
                ethabi::Token::Address(engine_mock_token_address),
                ethabi::Token::Address(user_address),
            ],
        );
        let outcome = call_aurora_contract(
            silo_to_silo_contract.address,
            contract_args,
            user_account,
            engine.inner.id(),
            0,
            true,
        )
        .await;

        let result = outcome.unwrap().borsh::<SubmitResult>().unwrap();
        if let TransactionStatus::Succeed(res) = result.status {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&res.as_slice()[res.len() - 8..res.len()]);
            assert_eq!(u64::from_be_bytes(buf), expected_value);
        }
    }

    async fn storage_deposit(token_contract: &Contract, account_id: &str, deposit: Option<u128>) {
        let outcome = token_contract
            .call("storage_deposit")
            .args_json(serde_json::json!({ "account_id": account_id }))
            .max_gas()
            .deposit(deposit.unwrap_or(1_250_000_000_000_000_000_000))
            .transact()
            .await
            .unwrap();

        assert!(
            outcome.failures().is_empty(),
            "Call to set failed: {:?}",
            outcome.failures()
        );
    }

    async fn engine_mint_tokens(
        user_address: Address,
        token_account: &ERC20,
        engine: &AuroraEngine,
    ) {
        let mint_args =
            token_account.create_mint_call_bytes(user_address, U256::from(TRANSFER_TOKENS_AMOUNT));
        let call_args = CallArgs::V1(FunctionCallArgsV1 {
            contract: token_account.address,
            input: mint_args.0,
        });

        let outcome = engine
            .inner
            .call("call")
            .args_borsh(call_args)
            .max_gas()
            .transact()
            .await
            .unwrap();

        assert!(
            outcome.failures().is_empty(),
            "Call to set failed: {:?}",
            outcome.failures()
        );

        let balance = engine
            .erc20_balance_of(&token_account, user_address)
            .await
            .unwrap();
        assert_eq!(balance.as_u64(), TRANSFER_TOKENS_AMOUNT);
    }

    async fn silo_to_silo_transfer(
        silo_to_silo_contract: &DeployedContract,
        token_account: &ERC20,
        silo1_address: &AccountId,
        silo2_address: &AccountId,
        user_account: Account,
        user_address: String,
        check_output: bool,
    ) {
        let contract_args = silo_to_silo_contract.create_call_method_bytes_with_args(
            "ftTransferCallToNear",
            &[
                ethabi::Token::Address(token_account.address.raw()),
                ethabi::Token::Uint(U256::from(TRANSFER_TOKENS_AMOUNT)),
                ethabi::Token::String(silo2_address.to_string()),
                ethabi::Token::String(user_address),
            ],
        );

        call_aurora_contract(
            silo_to_silo_contract.address,
            contract_args,
            &user_account,
            silo1_address,
            0,
            check_output,
        )
        .await
        .unwrap();
    }

    async fn withdraw(
        silo_to_silo_contract: &DeployedContract,
        token_address: H160,
        engine_address: &AccountId,
        user_account: Account,
    ) {
        let contract_args = silo_to_silo_contract.create_call_method_bytes_with_args(
            "withdraw",
            &[ethabi::Token::Address(token_address)],
        );

        call_aurora_contract(
            silo_to_silo_contract.address,
            contract_args,
            &user_account,
            engine_address,
            0,
            true,
        )
        .await
        .unwrap();
    }

    async fn call_aurora_contract(
        contract_address: Address,
        contract_args: Vec<u8>,
        user_account: &Account,
        engine_account: &AccountId,
        deposit: u128,
        check_output: bool,
    ) -> ExecutionFinalResult {
        let call_args = CallArgs::V2(FunctionCallArgsV2 {
            contract: contract_address,
            value: WeiU256::from(U256::from(deposit)),
            input: contract_args,
        });

        let outcome = user_account
            .call(engine_account, "call")
            .args_borsh(call_args)
            .max_gas()
            .transact()
            .await
            .unwrap();

        if check_output {
            assert!(
                outcome.failures().is_empty(),
                "Call to set failed: {:?}",
                outcome.failures()
            );
        }

        outcome
    }
}
