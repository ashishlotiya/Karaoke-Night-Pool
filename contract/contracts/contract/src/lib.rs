#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, contracterror, contractevent,
    Address, Env, Map, Vec,
    token,
    panic_with_error,
};

// ─── Error Codes ────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PoolError {
    AlreadyInitialized   = 1,
    NotAdmin             = 2,
    PoolNotOpen          = 3,
    PoolNotVoting        = 4,
    PoolNotClosed        = 5,
    AlreadyEntered       = 6,
    AlreadyVoted         = 7,
    EntryFeeNotMet       = 8,
    NoParticipants       = 9,
    InvalidCandidate     = 10,
    WinnerAlreadyPaid    = 11,
    NobodyVoted          = 12,
}

// ─── State Enum ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PoolStatus {
    Open,       // accepting entries
    Voting,     // voting is live
    Closed,     // voting ended, winner claimable
    Paid,       // winner has been paid out
}

// ─── Storage Keys ────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    TokenId,
    EntryFee,
    Status,
    Participants,       // Vec<Address>
    VoteCounts,         // Map<Address, u32>
    HasVoted,           // Map<Address, bool>
    PrizePot,           // u128 — total XLM/token held
    Winner,
    WinnerPaid,
}

// ─── Events ──────────────────────────────────────────────────────────────────

#[contractevent]
pub struct PoolInitialized {
    pub admin: Address,
    pub token_id: Address,
    pub entry_fee: i128,
}

#[contractevent]
pub struct ParticipantEntered {
    pub participant: Address,
    pub prize_pot: i128,
}

#[contractevent]
pub struct VotingOpened {}

#[contractevent]
pub struct VoteCast {
    pub voter: Address,
    pub candidate: Address,
}

#[contractevent]
pub struct VotingClosed {
    pub winner: Address,
    pub top_votes: u32,
}

#[contractevent]
pub struct PrizeClaimed {
    pub winner: Address,
    pub amount: i128,
}

// ─── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct FriendsPoolContract;

#[contractimpl]
impl FriendsPoolContract {

    // ── Admin: initialize ────────────────────────────────────────────────────
    /// Deploy and configure the pool.
    /// `token_id`  – SAC address of the token used for entry fees & prize.
    /// `entry_fee` – Amount (in stroops/base units) each participant must pay.
    pub fn initialize(
        env: Env,
        admin: Address,
        token_id: Address,
        entry_fee: i128,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, PoolError::AlreadyInitialized);
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin,        &admin);
        env.storage().instance().set(&DataKey::TokenId,      &token_id);
        env.storage().instance().set(&DataKey::EntryFee,     &entry_fee);
        env.storage().instance().set(&DataKey::Status,       &PoolStatus::Open);
        env.storage().instance().set(&DataKey::Participants, &Vec::<Address>::new(&env));
        env.storage().instance().set(&DataKey::VoteCounts,   &Map::<Address, u32>::new(&env));
        env.storage().instance().set(&DataKey::HasVoted,     &Map::<Address, bool>::new(&env));
        env.storage().instance().set(&DataKey::PrizePot,     &0_i128);
        env.storage().instance().set(&DataKey::WinnerPaid,   &false);

        env.events().publish_event(&PoolInitialized { admin, token_id, entry_fee });
    }

    // ── Participant: enter pool ──────────────────────────────────────────────
    /// Pay the entry fee and register as a contestant.
    pub fn enter(env: Env, participant: Address) {
        participant.require_auth();

        let status: PoolStatus = env.storage().instance().get(&DataKey::Status).unwrap();
        if status != PoolStatus::Open {
            panic_with_error!(&env, PoolError::PoolNotOpen);
        }

        let mut participants: Vec<Address> = env
            .storage().instance().get(&DataKey::Participants).unwrap();

        if participants.contains(&participant) {
            panic_with_error!(&env, PoolError::AlreadyEntered);
        }

        let entry_fee: i128 = env.storage().instance().get(&DataKey::EntryFee).unwrap();
        let token_id: Address = env.storage().instance().get(&DataKey::TokenId).unwrap();

        // Transfer entry fee from participant → contract
        let token = token::Client::new(&env, &token_id);
        token.transfer(&participant, &env.current_contract_address(), &entry_fee);

        // Update pot
        let mut pot: i128 = env.storage().instance().get(&DataKey::PrizePot).unwrap();
        pot += entry_fee;
        env.storage().instance().set(&DataKey::PrizePot, &pot);

        participants.push_back(participant.clone());
        env.storage().instance().set(&DataKey::Participants, &participants);

        // Initialise vote tally for this contestant
        let mut vote_counts: Map<Address, u32> =
            env.storage().instance().get(&DataKey::VoteCounts).unwrap();
        vote_counts.set(participant.clone(), 0_u32);
        env.storage().instance().set(&DataKey::VoteCounts, &vote_counts);

        env.events().publish_event(&ParticipantEntered { participant, prize_pot: pot });
    }

    // ── Admin: open voting ───────────────────────────────────────────────────
    pub fn open_voting(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let status: PoolStatus = env.storage().instance().get(&DataKey::Status).unwrap();
        if status != PoolStatus::Open {
            panic_with_error!(&env, PoolError::PoolNotOpen);
        }

        let participants: Vec<Address> =
            env.storage().instance().get(&DataKey::Participants).unwrap();
        if participants.is_empty() {
            panic_with_error!(&env, PoolError::NoParticipants);
        }

        env.storage().instance().set(&DataKey::Status, &PoolStatus::Voting);
        env.events().publish_event(&VotingOpened {});
    }

    // ── Audience: cast vote ──────────────────────────────────────────────────
    /// Anyone (including non-participants) may vote exactly once.
    pub fn vote(env: Env, voter: Address, candidate: Address) {
        voter.require_auth();

        let status: PoolStatus = env.storage().instance().get(&DataKey::Status).unwrap();
        if status != PoolStatus::Voting {
            panic_with_error!(&env, PoolError::PoolNotVoting);
        }

        // Check voter hasn't voted
        let mut has_voted: Map<Address, bool> =
            env.storage().instance().get(&DataKey::HasVoted).unwrap();
        if has_voted.get(voter.clone()).unwrap_or(false) {
            panic_with_error!(&env, PoolError::AlreadyVoted);
        }

        // Candidate must be a registered participant
        let mut vote_counts: Map<Address, u32> =
            env.storage().instance().get(&DataKey::VoteCounts).unwrap();
        if !vote_counts.contains_key(candidate.clone()) {
            panic_with_error!(&env, PoolError::InvalidCandidate);
        }

        // Tally vote
        let current = vote_counts.get(candidate.clone()).unwrap_or(0);
        vote_counts.set(candidate.clone(), current + 1);
        env.storage().instance().set(&DataKey::VoteCounts, &vote_counts);

        has_voted.set(voter.clone(), true);
        env.storage().instance().set(&DataKey::HasVoted, &has_voted);

        env.events().publish_event(&VoteCast { voter, candidate });
    }

    // ── Admin: close voting & determine winner ───────────────────────────────
    pub fn close_voting(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let status: PoolStatus = env.storage().instance().get(&DataKey::Status).unwrap();
        if status != PoolStatus::Voting {
            panic_with_error!(&env, PoolError::PoolNotVoting);
        }

        let vote_counts: Map<Address, u32> =
            env.storage().instance().get(&DataKey::VoteCounts).unwrap();

        // Find the candidate with the highest vote count
        let mut winner_addr: Option<Address> = None;
        let mut top_votes: u32 = 0;
        let mut total_votes: u32 = 0;

        let participants: Vec<Address> =
            env.storage().instance().get(&DataKey::Participants).unwrap();

        for p in participants.iter() {
            let v = vote_counts.get(p.clone()).unwrap_or(0);
            total_votes += v;
            if v > top_votes {
                top_votes = v;
                winner_addr = Some(p.clone());
            }
        }

        if total_votes == 0 {
            panic_with_error!(&env, PoolError::NobodyVoted);
        }

        let winner = winner_addr.unwrap();
        env.storage().instance().set(&DataKey::Winner, &winner);
        env.storage().instance().set(&DataKey::Status, &PoolStatus::Closed);

        env.events().publish_event(&VotingClosed { winner, top_votes });
    }

    // ── Winner: claim prize ──────────────────────────────────────────────────
    pub fn claim_prize(env: Env) {
        let status: PoolStatus = env.storage().instance().get(&DataKey::Status).unwrap();
        if status != PoolStatus::Closed {
            panic_with_error!(&env, PoolError::PoolNotClosed);
        }

        let paid: bool = env.storage().instance().get(&DataKey::WinnerPaid).unwrap();
        if paid {
            panic_with_error!(&env, PoolError::WinnerAlreadyPaid);
        }

        let winner: Address = env.storage().instance().get(&DataKey::Winner).unwrap();
        winner.require_auth();

        let pot: i128 = env.storage().instance().get(&DataKey::PrizePot).unwrap();
        let token_id: Address = env.storage().instance().get(&DataKey::TokenId).unwrap();

        let token = token::Client::new(&env, &token_id);
        token.transfer(&env.current_contract_address(), &winner, &pot);

        env.storage().instance().set(&DataKey::WinnerPaid, &true);
        env.storage().instance().set(&DataKey::Status, &PoolStatus::Paid);

        env.events().publish_event(&PrizeClaimed { winner, amount: pot });
    }

    // ── Read-only helpers ────────────────────────────────────────────────────

    pub fn get_status(env: Env) -> PoolStatus {
        env.storage().instance().get(&DataKey::Status).unwrap()
    }

    pub fn get_participants(env: Env) -> Vec<Address> {
        env.storage().instance().get(&DataKey::Participants).unwrap()
    }

    pub fn get_vote_counts(env: Env) -> Map<Address, u32> {
        env.storage().instance().get(&DataKey::VoteCounts).unwrap()
    }

    pub fn get_prize_pot(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::PrizePot).unwrap()
    }

    pub fn get_winner(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Winner).unwrap()
    }

    pub fn get_entry_fee(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::EntryFee).unwrap()
    }
}