//! P21 binding of the settled journal cut to the already-installed custody route.
//! Decoded observations come from a separately authenticated chain port. Matching
//! them is not RPC authentication, native-key fencing, a tail seal or activation.
use super::{Controller, Rail};
use crate::Error;
use cinder_journal::{
    Backend, Head, Journal, Protection,
    recovery::{Cut, CutError},
};
use cinder_kernel::{
    identity::{AccountId, EventKey},
    ledger::Location,
};

/// Exact finalized public customer account plus verified recipient-token binding.
/// The chain port checks the canonical counter PDA/config/owner and classic SPL
/// mint/owner/initialized recipient; the counter does not itself store `tokens`.
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomerState {
    /// Registered owner decoded from the canonical P15 counter.
    pub wallet: [u8; 32],
    /// Verified mint/owner recipient token account, not a counter field.
    pub tokens: [u8; 32],
    /// Lifetime settled beneficiary mint atoms, excluding rail fees.
    pub paid: u64,
    /// Lifetime payout sequence; zero only for a fresh counter.
    pub payout_sequence: u64,
}
impl std::fmt::Debug for CustomerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CustomerState([PRIVATE])")
    }
}

/// Trusted-port input, NOT a publicly acceptable proof or customer request.
/// All accounts must share a coherent finalized slot. Before supplying it, the
/// port must validate actual cluster/genesis, program/config owners and schema,
/// canonical config/vault/customer PDAs, configured roles, mint decimals, classic
/// SPL ownership and clean/unfrozen custody authority. Never accept a host JSON
/// assertion as the port's observation. These responsibilities remain explicit.
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyState {
    /// Independently qualified cluster/genesis identity.
    pub network: [u8; 32],
    /// Actual deployed custody executable.
    pub program: [u8; 32],
    /// Canonical typed P15 config account.
    pub config: [u8; 32],
    /// Canonical custody token vault.
    pub vault: [u8; 32],
    /// Classic SPL collateral mint.
    pub mint: [u8; 32],
    /// Config's governed deployment-domain seed.
    pub domain: [u8; 32],
    /// Config's immutable pool seed.
    pub pool: [u8; 32],
    /// Configured external funding/return wallet.
    pub broker: [u8; 32],
    /// Config's fixed broker token account.
    pub broker_tokens: [u8; 32],
    /// Observed mint precision; must match the already-installed P16 atom scale.
    pub decimals: u8,
    /// Ordinary authority epoch after the single P15 freeze increment.
    pub epoch: u64,
    /// Actual P15 mode; this preparation slice requires FROZEN (1).
    pub mode: u8,
    /// Config's lifetime normal beneficiary payments, in mint atoms.
    pub normal_paid: u64,
    /// Config's lifetime working-capital release sequence; returns do not reset it.
    pub funding_sequence: u64,
    /// Actual clean custody token amount, not displayed aggregate venue equity.
    pub vault_amount: u64,
    /// Coherent finalized Solana slot covering these observations.
    pub finalized_slot: u64,
    /// Complete registered inventory for the installed private beneficiary binding.
    pub customers: Vec<CustomerState>,
}
impl std::fmt::Debug for CustodyState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CustodyState([PRIVATE])")
    }
}
/// No identities, amounts, payloads or keys in refusals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingError {
    /// The authoritative financial cut is not ready.
    Cut(CutError),
    /// Installed route/history or journal freshness failed.
    Adapter(Error),
    /// Wrong route/frozen epoch/slot, incomplete inventory or counters/asset mismatch.
    Mismatch,
    /// Existing exact atom units exceed the program's u64; never scale or truncate.
    Precision,
}

/// Positive claim input in custody mint atoms; the already-net amount is never
/// reduced by `paid_base` again. Zero/negative customers stay in `Backing::cut()`.
#[derive(Clone, PartialEq, Eq)]
pub struct ClaimBasis {
    account: AccountId,
    wallet: [u8; 32],
    tokens: [u8; 32],
    amount: u64,
    paid_base: u64,
    payout_sequence_base: u64,
}
impl std::fmt::Debug for ClaimBasis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ClaimBasis([PRIVATE])")
    }
}
impl ClaimBasis {
    /// Original opaque financial owner.
    pub fn account(&self) -> AccountId {
        self.account
    }
    /// Registered wallet for owner-signed recovery claims.
    pub fn wallet(&self) -> [u8; 32] {
        self.wallet
    }
    /// Exact governed recipient, never selected by the package publisher.
    pub fn tokens(&self) -> [u8; 32] {
        self.tokens
    }
    /// Positive unpaid mint atoms, already net of losses/fees/normal payments.
    pub fn amount(&self) -> u64 {
        self.amount
    }
    /// Lifetime confirmed beneficiary payments at this cut, not another deduction.
    pub fn paid_base(&self) -> u64 {
        self.paid_base
    }
    /// Exact lifetime payout sequence at the cut, not inferred from paid amount.
    pub fn payout_sequence_base(&self) -> u64 {
        self.payout_sequence_base
    }
}
/// Private matching result, not an independently authenticated solvency statement
/// or non-clone spending capability. Staging still needs sealed/fenced preparation,
/// independent package availability and separately checked chain submission.
#[derive(Clone, PartialEq, Eq)]
pub struct Backing {
    cut: Cut,
    claims: Vec<ClaimBasis>,
    epoch: u64,
    normal_paid: u64,
    funding_sequence: u64,
    finalized_slot: u64,
    total: u64,
}
impl std::fmt::Debug for Backing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryBacking([PRIVATE])")
    }
}
impl Backing {
    /// Same private financial projection, including zero/negative accounts.
    pub fn cut(&self) -> &Cut {
        &self.cut
    }
    /// Positive finalized claim inputs in immutable customer configuration order.
    pub fn claims(&self) -> &[ClaimBasis] {
        &self.claims
    }
    /// P15 frozen recovery epoch; no root is yet authorized.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    /// Aggregate lifetime beneficiary payments matched to both journal and config.
    pub fn normal_paid(&self) -> u64 {
        self.normal_paid
    }
    /// Matched lifetime working-capital release counter.
    pub fn funding_sequence(&self) -> u64 {
        self.funding_sequence
    }
    /// Qualified observation slot, not a venue causal cut or injected time.
    pub fn finalized_slot(&self) -> u64 {
        self.finalized_slot
    }
    /// Checked positive claim sum in the program's exact u64 mint atoms.
    pub fn total(&self) -> u64 {
        self.total
    }
}

impl Controller {
    /// Reuse the installed P16 atom/beneficiary binding and accepted plan/receipt
    /// history. P15 counters originate at zero; unsupported preexisting counter
    /// baselines need explicit migration, not inferred reset or an operator input.
    pub fn recovery_backing<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        expected: Head,
        check: EventKey,
        observed: &CustodyState,
    ) -> Result<Backing, BindingError> {
        let cut = j.recovery_cut(expected, check).map_err(BindingError::Cut)?;
        let at = j
            .state()
            .map_err(|e| BindingError::Adapter(Error::Journal(e)))?
            .logical_time();
        let history = self.history(j, at).map_err(BindingError::Adapter)?;
        let r = &self.route;
        if !history.bound
            || observed.network != self.profile.config.domain.network.bytes()
            || observed.program != r.program || observed.config != r.config
            || observed.vault != r.vault || observed.mint != r.mint
            || observed.domain != r.domain || observed.pool != r.pool
            || observed.broker != r.broker || observed.broker_tokens != r.broker_tokens
            || observed.decimals != r.decimals
            || r.epoch.checked_add(1) != Some(observed.epoch)
            || observed.mode != 1 // P15 FROZEN, before any root is staged.
            || observed.finalized_slot == 0
            || observed.customers.len() != r.beneficiaries.len()
            || observed.customers.iter().enumerate().any(|(i, c)| observed.customers[..i].iter().any(|p| p.wallet == c.wallet || p.tokens == c.tokens))
            || u64::try_from(cut.vault().atoms()).map_err(|_| BindingError::Precision)? != observed.vault_amount
        {
            return Err(BindingError::Mismatch);
        }

        // Do not mix venue causal cuts/milliseconds with Solana slots. Vault and
        // broker receipt cuts belong to the qualified chain source only.
        for (tx, receipt) in j.transactions_with_receipts() {
            for (input, outcome) in tx.inputs.iter().zip(&receipt.inputs) {
                // Resolved raw/rejected envelopes are not applied chain postings.
                // Their original absent cut cannot become a permanent extra fault.
                if !matches!(
                    outcome,
                    cinder_journal::model::InputResult::Normalized(
                        cinder_kernel::ledger::evidence::Disposition::Applied
                            | cinder_kernel::ledger::evidence::Disposition::Duplicate
                    )
                ) {
                    continue;
                }
                if self.profile.config.sources.iter().any(|s| {
                    s.scope == input.source
                        && matches!(s.location, Location::Vault | Location::Broker)
                }) && input
                    .source_cut
                    .is_none_or(|slot| slot > observed.finalized_slot)
                {
                    return Err(BindingError::Mismatch);
                }
            }
            if receipt.controls.is_none() {
                for control in &tx.controls {
                    if let cinder_journal::model::Control::ResolveRaw(r) = control
                        && self.profile.config.sources.iter().any(|s| {
                            s.scope == r.source
                                && matches!(s.location, Location::Vault | Location::Broker)
                        })
                        && r.through > observed.finalized_slot
                    {
                        return Err(BindingError::Mismatch);
                    }
                }
            }
        }
        let mut funding_sequence = 0_u64;
        let mut paid = vec![(0_u64, 0_u64); r.beneficiaries.len()];
        let mut seen = Vec::new();
        for plan in &history.plans {
            if seen.contains(&plan.attempt) {
                return Err(BindingError::Mismatch);
            }
            seen.push(plan.attempt.clone());
            if !history
                .chains
                .iter()
                .any(|(a, _, succeeded)| a == &plan.attempt && *succeeded)
            {
                continue;
            }
            match plan.rail {
                Rail::Release => {
                    if plan.counters.sequence != funding_sequence {
                        return Err(BindingError::Mismatch);
                    }
                    funding_sequence = funding_sequence
                        .checked_add(1)
                        .ok_or(BindingError::Precision)?;
                }
                Rail::Payout => {
                    let account = plan
                        .attempt()
                        .map_err(BindingError::Adapter)?
                        .request
                        .account
                        .bytes();
                    let i = r
                        .beneficiaries
                        .iter()
                        .position(|b| b.account == account)
                        .ok_or(BindingError::Mismatch)?;
                    let (amount, sequence) = paid[i];
                    if plan.customer != r.beneficiaries[i].wallet
                        || plan.counters.paid != amount
                        || plan.counters.sequence != sequence
                    {
                        return Err(BindingError::Mismatch);
                    }
                    paid[i] = (
                        amount
                            .checked_add(plan.gross)
                            .ok_or(BindingError::Precision)?,
                        sequence.checked_add(1).ok_or(BindingError::Precision)?,
                    );
                }
                Rail::Deposit | Rail::Withdraw | Rail::Return => {}
            }
        }
        if funding_sequence != observed.funding_sequence {
            return Err(BindingError::Mismatch);
        }
        let mut normal_paid = 0_u64;
        let mut claims = Vec::new();
        for a in cut.accounts() {
            let i = r
                .beneficiaries
                .iter()
                .position(|b| b.account == a.account().bytes())
                .ok_or(BindingError::Mismatch)?;
            let b = &r.beneficiaries[i];
            let chain = observed
                .customers
                .iter()
                .find(|c| c.wallet == b.wallet && c.tokens == b.tokens)
                .ok_or(BindingError::Mismatch)?;
            let amount = u64::try_from(a.payable().atoms()).map_err(|_| BindingError::Precision)?;
            let paid_base = u64::try_from(a.paid().atoms()).map_err(|_| BindingError::Precision)?;
            if chain.paid != paid_base || (chain.paid, chain.payout_sequence) != paid[i] {
                return Err(BindingError::Mismatch);
            }
            normal_paid = normal_paid
                .checked_add(paid_base)
                .ok_or(BindingError::Precision)?;
            if amount > 0 {
                claims.push(ClaimBasis {
                    account: a.account(),
                    wallet: b.wallet,
                    tokens: b.tokens,
                    amount,
                    paid_base,
                    payout_sequence_base: chain.payout_sequence,
                });
            }
        }
        if normal_paid != observed.normal_paid {
            return Err(BindingError::Mismatch);
        }
        let total = u64::try_from(cut.total().atoms()).map_err(|_| BindingError::Precision)?;
        Ok(Backing {
            cut,
            claims,
            epoch: observed.epoch,
            normal_paid,
            funding_sequence,
            finalized_slot: observed.finalized_slot,
            total,
        })
    }
}
