# 🏆 Friends Pool — Soroban Smart Contract

> **Put skin in the game. Let the crowd decide who wins it.**

A trustless prize-pool contract on [Stellar](https://stellar.org) built with the [Soroban SDK](https://developers.stellar.org/docs/smart-contracts). Friends each pay an entry fee to compete; the audience votes for their favourite; the winner claims the entire pot — automatically, on-chain, no middleman.

---

## 📖 Project Description

**Friends Pool** is a Soroban smart contract that turns any friendly competition into a transparent, self-executing prize pool. Whether it's a cooking showdown, a coding hackathon, a lip-sync battle, or a trivia night — participants lock in their entry fees on-chain, the crowd votes live, and the smart contract pays out the winner the moment voting closes.

No trust required. No manual transfers. No disputes. The contract holds every token and only releases the prize when a winner is determined by popular vote.

---

## ⚙️ What It Does

The contract lifecycle has four stages:

```
OPEN ──► VOTING ──► CLOSED ──► PAID
```

| Stage | Who acts | What happens |
|-------|----------|--------------|
| **OPEN** | Participants | Each participant calls `enter()` and pays the entry fee in tokens. The fee is transferred directly into the contract. |
| **VOTING** | Admin | Admin calls `open_voting()` to lock entries and start the audience vote. |
| **VOTING** | Audience | Any address (fan, friend, spectator) calls `vote(voter, candidate)` — one vote per address, cast for any registered participant. |
| **CLOSED** | Admin | Admin calls `close_voting()`. The contract tallies votes, finds the top-voted participant, and records them as winner. |
| **PAID** | Winner | The winner calls `claim_prize()`. The full token pot is transferred to their wallet. |

---

## ✨ Features

### 🔒 Trustless Custody
The entry fee is transferred from each participant to the contract address at the moment they enter. No admin can touch those tokens — only a verified winner can claim.

### 🗳️ Open Audience Voting
Voting is not limited to participants. Any Stellar address can vote once, making this a true crowd-choice award. One address = one vote, enforced on-chain.

### 🏅 Automatic Winner Resolution
`close_voting()` iterates all participants, tallies vote counts, and records the highest-voted address as winner — no off-chain oracle needed.

### 💸 Instant Prize Payout
`claim_prize()` transfers the entire prize pot (`entry_fee × number_of_participants`) to the winner in a single atomic token transfer.

### 🪙 Any Stellar Token
The contract works with any Stellar Asset Contract (SAC) token — XLM wrapped as a SAC, USDC, or any custom token on the network.

### 🛡️ Guard Rails & Error Handling
Every action is protected by explicit state checks and a typed `PoolError` enum:

| Code | Error | Meaning |
|------|-------|---------|
| 1 | `AlreadyInitialized` | `initialize()` called more than once |
| 2 | `NotAdmin` | Caller is not the pool admin |
| 3 | `PoolNotOpen` | Action requires OPEN status |
| 4 | `PoolNotVoting` | Action requires VOTING status |
| 5 | `PoolNotClosed` | Claim requires CLOSED status |
| 6 | `AlreadyEntered` | Participant tried to enter twice |
| 7 | `AlreadyVoted` | Voter tried to vote twice |
| 8 | `EntryFeeNotMet` | Insufficient token balance *(raised by token contract)* |
| 9 | `NoParticipants` | Voting opened with zero entries |
| 10 | `InvalidCandidate` | Vote cast for unregistered address |
| 11 | `WinnerAlreadyPaid` | Prize claimed twice |
| 12 | `NobodyVoted` | `close_voting()` called with zero votes |

### 📡 On-Chain Events
The contract emits events at every stage so dApps and explorers can react in real time:

- `pool_initialized` — admin, token, entry fee
- `participant_entered` — address, running pot total
- `voting_opened`
- `vote_cast` — voter, candidate
- `voting_closed` — winner address, winning vote count
- `prize_claimed` — winner address, amount

### 🔍 Read-Only Queries
Inspect the pool state at any time without paying fees:

```rust
get_status()        // → PoolStatus
get_participants()  // → Vec<Address>
get_vote_counts()   // → Map<Address, u32>
get_prize_pot()     // → i128
get_winner()        // → Address
get_entry_fee()     // → i128
```

---

## 🚀 Quick Start

### Prerequisites

- Rust + `wasm32-unknown-unknown` target
- [Stellar CLI](https://developers.stellar.org/docs/smart-contracts/getting-started/setup)

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked stellar-cli --features opt
```

### Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

The compiled `.wasm` will be at:
```
target/wasm32-unknown-unknown/release/friends_pool.wasm
```

### Run Tests

```bash
cargo test
```

### Deploy to Testnet

```bash
# Fund a test account
stellar keys generate alice --network testnet --fund

# Deploy
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/friends_pool.wasm \
  --source alice \
  --network testnet

# Initialize (replace CONTRACT_ID and TOKEN_ADDRESS)
stellar contract invoke \
  --id CONTRACT_ID \
  --source alice \
  --network testnet \
  -- initialize \
  --admin $(stellar keys address alice) \
  --token_id TOKEN_ADDRESS \
  --entry_fee 1000000   # 0.1 XLM in stroops
```

---

## 🗂 Contract Interface Summary

```rust
// Admin
fn initialize(env, admin: Address, token_id: Address, entry_fee: i128)
fn open_voting(env, admin: Address)
fn close_voting(env, admin: Address)

// Participants
fn enter(env, participant: Address)

// Audience
fn vote(env, voter: Address, candidate: Address)

// Winner
fn claim_prize(env)

// Read-only
fn get_status(env)        -> PoolStatus
fn get_participants(env)  -> Vec<Address>
fn get_vote_counts(env)   -> Map<Address, u32>
fn get_prize_pot(env)     -> i128
fn get_winner(env)        -> Address
fn get_entry_fee(env)     -> i128
```

---

## 🛣️ Potential Extensions

- **Tie-breaking** — run-off vote or time-based tiebreaker
- **Platform fee** — take a small % before paying out winner
- **Deadline enforcement** — use `env.ledger().timestamp()` to auto-close voting
- **Multi-winner** — distribute pot proportionally by vote share
- **Frontend dApp** — React + Stellar Wallets Kit integration

---

## 📄 License

MIT — use freely, fork liberally.

wallet address: GB3X72JHINDKU4JHCWVORELHDLOPSZX2755KNGVEMACTIC6CTA2UGWRE

contract address: CDIXKIMBUT2HCGFMRS7YZVDXZV4ZL54BWGT7Z7EHVPDFG7XEG3KRIMLM

https://stellar.expert/explorer/testnet/contract/CDIXKIMBUT2HCGFMRS7YZVDXZV4ZL54BWGT7Z7EHVPDFG7XEG3KRIMLM

<img width="1868" height="903" alt="image" src="https://github.com/user-attachments/assets/331e34cb-50ef-4046-b02c-45ec87dbc3cd" />
