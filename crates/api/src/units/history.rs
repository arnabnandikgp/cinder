//! Durable administrative effects and recorded-only financial intents. This is
//! not a handler returning financial Accepted receipts or a native capability.
use super::*;
use cinder_journal::{model::CommitId, units::api as retained};

/// Original private mutation and its capture result. Recorded financial requests
/// have not passed admission; never map them to `Outcome::Accepted`.
#[derive(Clone, PartialEq, Eq)]
pub struct Captured {
    /// Original signed request, not the retry's refreshed auth envelope.
    pub request: Request,
    /// Original journal time, not retry time.
    pub at: u64,
    /// Completed administration or recorded-only financial intent.
    pub effect: retained::Effect,
    /// Exact economic retry; no second mutation occurred.
    pub duplicate: bool,
}
impl fmt::Debug for Captured {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Captured([PRIVATE])")
    }
}

/// Immutable accepted-history interpretation at one fresh head. No independent
/// persistent store, financial state, or reusable authentication result.
pub struct History {
    head: Head,
    contract: [u8; 32],
    rows: Vec<Captured>,
    grants: Vec<(Scope, GrantCut)>,
}
impl fmt::Debug for History {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiHistory([PRIVATE])")
    }
}
impl History {
    /// Trusted in-enclave loading only. A later caller still authenticates and
    /// rechecks freshness; a retained History is not perpetual current authority.
    pub fn load<B: Backend, P: Protection>(
        journal: &mut units::Journal<B, P>,
        auth: &Authorizer,
    ) -> Result<Self, Error> {
        journal.verified_state().map_err(|_| Error::Unavailable)?;
        let state = journal.state().map_err(|_| Error::Unavailable)?;
        let mut w = Writer::new(b"CINDER-UNIT-API-CONTRACT\0\x00\x01");
        w.raw(
            &state
                .configuration()
                .commitment()
                .map_err(|_| Error::Unavailable)?,
        );
        w.u64(auth.maximum_auth_lifetime);
        w.u64(auth.maximum_grant_lifetime);
        let mut history = Self {
            head: journal.head(),
            contract: Sha256::digest(&*w.0).into(),
            rows: vec![],
            grants: vec![],
        };
        for accepted in state.api_records() {
            history.push(accepted, state, auth)?;
        }
        // Count actual admitted orders, never mere capture. Unattributed orders
        // cannot silently become zero agent usage when financial joining arrives.
        for descriptor in state.configuration().units() {
            for order in state
                .unit(descriptor.id)
                .ok_or(Error::Unavailable)?
                .orders()
            {
                let original = history
                    .find(order.intent.request.account, order.intent.request.request)
                    .ok_or(Error::Unavailable)?;
                if original.request.unit != descriptor.id
                    || original.request.body.epoch != order.intent.authority_epoch
                    || !matches!(original.request.body.command, Command::Order { .. })
                {
                    return Err(Error::Unavailable);
                }
                let unit = original.request.unit;
                let account = original.request.body.account;
                let epoch = original.request.body.epoch;
                let signer = original.request.body.signer;
                if let Some((_, cut)) = history.grants.iter_mut().find(|(s, g)| {
                    s.unit == unit
                        && s.account == account
                        && s.epoch == epoch
                        && g.grant.key == signer
                }) {
                    cut.used_orders = cut.used_orders.checked_add(1).ok_or(Error::Unavailable)?;
                    if cut.used_orders > cut.grant.maximum_orders {
                        return Err(Error::Unavailable);
                    }
                } else if !state
                    .customers()
                    .iter()
                    .any(|c| c.account == account && c.wallet == signer)
                {
                    return Err(Error::Unavailable);
                }
            }
        }
        Ok(history)
    }
    fn find(&self, account: AccountId, request: RequestId) -> Option<&Captured> {
        self.rows
            .iter()
            .find(|r| r.request.body.account == account && r.request.body.id == request)
    }
    fn push(
        &mut self,
        a: &retained::Accepted,
        state: &units::State,
        auth: &Authorizer,
    ) -> Result<(), Error> {
        let r = &a.record;
        let req = Request::decode(r.wire.as_bytes()).map_err(|_| Error::Unavailable)?;
        let b = &req.body;
        let owner = state
            .customers()
            .iter()
            .find(|c| c.account == b.account)
            .ok_or(Error::Unavailable)?;
        let config = &state
            .configuration()
            .units()
            .iter()
            .find(|u| u.id == req.unit)
            .ok_or(Error::Unavailable)?
            .ledger;
        if r.contract != self.contract
            || r.unit != req.unit
            || r.request != key(&req)
            || r.digest != req.digest()
            || r.epoch != b.epoch
            || r.signer != b.signer
            || r.policy != b.policy
            || b.policy != config.policy
            || b.domain != config.domain
            || b.epoch == 0
            || b.session == [0; 32]
            || b.expires_at <= a.at
            || b.expires_at - a.at > auth.maximum_auth_lifetime
            || self.find(b.account, b.id).is_some()
        {
            return Err(Error::Unavailable);
        }
        VerifyingKey::from_bytes(&b.signer)
            .map_err(|_| Error::Unavailable)?
            .verify_strict(&req.message(), &Signature::from_bytes(&b.signature))
            .map_err(|_| Error::Unavailable)?;
        let is_owner = b.signer == owner.wallet;
        if !is_owner {
            let (_, cut) = self
                .grants
                .iter()
                .find(|(s, g)| *s == scope(&req) && g.grant.key == b.signer)
                .ok_or(Error::Unavailable)?;
            authorization::permissions(&b.command, &cut.grant, a.at)
                .map_err(|_| Error::Unavailable)?;
        }
        let effect = self
            .effect(&req, is_owner, auth, owner.wallet, config, a.at)
            .map_err(|_| Error::Unavailable)?;
        if effect != r.effect {
            return Err(Error::Unavailable);
        }
        if let Command::Grant(g) = &b.command {
            self.grants.push((
                scope(&req),
                GrantCut {
                    head: self.head,
                    scope: scope(&req),
                    grant: g.clone(),
                    accepted_at: a.at,
                    used_orders: 0,
                },
            ));
        }
        self.rows.push(Captured {
            request: req,
            at: a.at,
            effect,
            duplicate: false,
        });
        Ok(())
    }
    fn effect(
        &self,
        req: &Request,
        is_owner: bool,
        auth: &Authorizer,
        owner: [u8; 32],
        config: &cinder_kernel::ledger::Config,
        now: u64,
    ) -> Result<retained::Effect, Error> {
        let market = |id| {
            if config.markets.iter().any(|m| m.unit().market == id) {
                Ok(())
            } else {
                Err(Error::Invalid)
            }
        };
        let deadline = |at| {
            if at > now {
                Ok(())
            } else {
                Err(Error::Invalid)
            }
        };
        match &req.body.command {
            Command::Grant(g) if is_owner => {
                auth.validate_grant(g, owner, config, now)?;
                if self
                    .grants
                    .iter()
                    .any(|(s, cut)| *s == scope(req) && cut.grant.key == g.key)
                {
                    return Err(Error::Invalid);
                }
                Ok(retained::Effect::Grant)
            }
            Command::Revoke if is_owner => {
                req.body.epoch.checked_add(1).ok_or(Error::Invalid)?;
                Ok(retained::Effect::Revoke)
            }
            Command::Order {
                market: m,
                lots,
                minimum,
                maximum,
                fee,
                good_until,
                ..
            } => {
                market(*m)?;
                deadline(*good_until)?;
                if *lots == 0
                    || minimum > maximum
                    || *fee < 0
                    || fee.checked_mul(i128::from(lots.unsigned_abs())).is_none()
                {
                    return Err(Error::Invalid);
                }
                Ok(retained::Effect::Recorded)
            }
            Command::Cancel {
                target, good_until, ..
            } => {
                deadline(*good_until)?;
                let old = self
                    .find(req.body.account, *target)
                    .filter(|o| o.request.unit == req.unit)
                    .ok_or(Error::NotFound)?;
                let Command::Order { market, .. } = &old.request.body.command else {
                    return Err(Error::Invalid);
                };
                if !is_owner
                    && (old.request.body.signer != req.body.signer
                        || !self.grants.iter().any(|(s, g)| {
                            *s == scope(req)
                                && g.grant.key == req.body.signer
                                && g.grant.market == *market
                        }))
                {
                    return Err(Error::Unauthorized);
                }
                Ok(retained::Effect::Recorded)
            }
            Command::Payout {
                net,
                maximum_fee,
                good_until,
                ..
            } if is_owner => {
                deadline(*good_until)?;
                if *net <= 0 || *maximum_fee < 0 || net.checked_add(*maximum_fee).is_none() {
                    return Err(Error::Invalid);
                }
                Ok(retained::Effect::Recorded)
            }
            Command::Leverage {
                market: m,
                leverage,
                good_until,
            } if is_owner => {
                market(*m)?;
                deadline(*good_until)?;
                if *leverage == 0 {
                    return Err(Error::Invalid);
                }
                Ok(retained::Effect::Recorded)
            }
            _ => Err(Error::Invalid),
        }
    }
}
impl Grants for History {
    fn lookup(
        &self,
        scope: Scope,
        signer: [u8; 32],
        head: Head,
    ) -> Result<Option<GrantCut>, Error> {
        if head != self.head {
            return Err(Error::Unavailable);
        }
        Ok(self
            .grants
            .iter()
            .find(|(s, g)| *s == scope && g.grant.key == signer)
            .map(|(_, g)| g.clone()))
    }
}

impl Authorizer {
    /// Authenticate then capture one original mutation. Financial results are
    /// recorded-only. No unknown-outcome retry, admission or signer is hidden here.
    pub fn capture<B: Backend, P: Protection>(
        &self,
        journal: &mut units::Journal<B, P>,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<Captured, Error> {
        let history = History::load(journal, self)?;
        let checked = self.check_inner(journal, channel, bytes, &history, now, false)?;
        if checked.head() != history.head {
            return Err(Error::Unavailable);
        }
        let req = checked.request();
        if matches!(
            req.body.command,
            Command::View | Command::Read(_) | Command::Operation(_)
        ) {
            return Err(Error::Invalid);
        }
        if let Some(old) = history.find(req.body.account, req.body.id) {
            if checked.principal() != Principal::Owner && old.request.body.signer != req.body.signer
            {
                return Err(Error::Unauthorized);
            }
            if old.request.unit != req.unit || old.request.digest() != req.digest() {
                return Err(Error::Conflict);
            }
            let mut old = old.clone();
            old.duplicate = true;
            return Ok(old);
        }
        let state = journal.state().map_err(|_| Error::Unavailable)?;
        let descriptor = state
            .configuration()
            .units()
            .iter()
            .find(|u| u.id == req.unit)
            .ok_or(Error::Unavailable)?;
        let owner = state
            .customers()
            .iter()
            .find(|c| c.account == req.body.account)
            .ok_or(Error::Unavailable)?;
        let effect = history.effect(
            req,
            checked.principal() == Principal::Owner,
            self,
            owner.wallet,
            &descriptor.ledger,
            now,
        )?;
        let mut w = Writer::new(b"CINDER-UNIT-API-COMMIT\0\x00\x01");
        for id in [
            req.body.domain.network.bytes(),
            req.body.domain.deployment.bytes(),
            req.body.account.bytes(),
            req.body.id.bytes(),
        ] {
            w.raw(&id);
        }
        let tx = units::Transaction {
            id: CommitId::new(Sha256::digest(&*w.0).into()).map_err(|_| Error::Invalid)?,
            expected: checked.head(),
            at: now,
            change: units::Change::Api(Box::new(retained::Record {
                unit: req.unit,
                request: key(req),
                policy: req.body.policy,
                digest: req.digest(),
                epoch: req.body.epoch,
                signer: req.body.signer,
                contract: history.contract,
                effect,
                wire: req.encode()?,
            })),
        };
        journal.commit(tx).map_err(|e| {
            if e == cinder_journal::Error::Conflict {
                Error::Conflict
            } else {
                Error::Unavailable
            }
        })?;
        Ok(Captured {
            request: req.clone(),
            at: now,
            effect,
            duplicate: false,
        })
    }
    /// Authenticated unit-scoped original-record lookup, not a financial lifecycle
    /// response. READ is unit-wide. No lookup result is returned before auth.
    pub fn captured_operation<B: Backend, P: Protection>(
        &self,
        journal: &mut units::Journal<B, P>,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<Captured, Error> {
        let history = History::load(journal, self)?;
        let checked = self.check(journal, channel, bytes, &history, now)?;
        if checked.head() != history.head {
            return Err(Error::Unavailable);
        }
        let req = checked.request();
        let Command::Operation(target) = req.body.command else {
            return Err(Error::Invalid);
        };
        history
            .find(req.body.account, target)
            .filter(|o| o.request.unit == req.unit)
            .cloned()
            .ok_or(Error::NotFound)
    }
}
fn key(req: &Request) -> RequestKey {
    RequestKey {
        domain: req.body.domain,
        account: req.body.account,
        request: req.body.id,
    }
}
fn scope(req: &Request) -> Scope {
    Scope {
        domain: req.body.domain,
        unit: req.unit,
        account: req.body.account,
        epoch: req.body.epoch,
    }
}
