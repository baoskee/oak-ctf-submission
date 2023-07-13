# Sample Report template

## Challenge 01: *Mjolnir*

### Description

The bug occurs in `withdraw`, because `ids` does not 
check for duplicates. User can deposit once and 
send a withdraw message with their deposit ID repeated
in the array such that the contract is drained. 

### Recommendation

The fix should be check `ids` in message for duplicates.

### Proof of concept

```rust
// code goes here

```

---

## Challenge 02: *Gungnir*

### Description

The bug occurs in `unstake` 
```rust
// voting_power is a u128 and Cargo.toml has `overflow_checks=false`.
// Message with unlock_amount < voting_power will overflow the u128 
user.voting_power -= unlock_amount;
```

### Recommendation

The fix should be changing `overflow_checks=true` or changing 
u128 to Uint128. 

### Proof of concept

```rust
// code goes here
```

---

## Challenge 03: *Laevateinn*

### Description

The bug occurs in `request_flash_loan` because
the message can call the proxy itself to transfer
ownership of the contract.

### Recommendation

The fix should prevent the proxy contract's recipient to 
be itself.

### Proof of concept

```rust
// code goes here
```

---

## Challenge 04: *Gram*

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

## Challenge 05: *Draupnir*

### Description

The bug occurs in ...

### Recommendation

The fix should be ...

### Proof of concept

```rust
// code goes here
```

---

## Challenge 06: *Hofund*

### Description




### Recommendation


```rust

```

### Proof of concept

```rust
// code goes here
```

---

## Challenge 07: *Tyrfing*

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

```rust
// code goes here
```

---

## Challenge 08: *Gjallarhorn*

### Description

The bug occurs in ...

### Recommendation

The fix should be ...

### Proof of concept

```rust
// code goes here
```

---

## Challenge 09: *Brisingamen*

### Description

The bug occurs in ...

### Recommendation

The fix should be ...

### Proof of concept

```rust
// code goes here
```

---

## Challenge 10: *Mistilteinn*

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

```rust
// code goes here
```
