//! Versioned owner-scoped retained reads. No raw/native ID or global sequence.
use crate::{
    Admission, Error, Record, Response, Service,
    wire::{Command, Reader, Request, Writer},
};
use cinder_journal::{
    model::{PrivateBytes, Resource},
    read::Source,
};
use cinder_kernel::{
    amounts::MarketUnit,
    identity::RecordKey,
    ledger::{Change, FillTarget, Owner, economics::EconomicChange},
};
use sha2::{Digest, Sha256};

/// Hard page bound, separate from retained journal evidence.
pub const MAX_PAGE: usize = 64;
/// Account/positions/operations/open orders/fills/funding/book movements/
/// grants/configured markets/committed account snapshot family count.
pub const KINDS: u8 = 10;
/// Signed read. Zero cursor starts a new snapshot, never a global journal cut.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Query {
    /// Exact family 0..9 in the order listed above.
    pub kind: u8,
    /// Permitted-row commitment then offset; all zero for initial page.
    pub cursor: [u8; 40],
    /// Finite page length, 1..64.
    pub limit: u16,
}
impl Query {
    pub(crate) fn encode(&self, w: &mut Writer) {
        w.byte(1);
        w.byte(self.kind);
        w.raw(&self.cursor);
        w.raw(&self.limit.to_be_bytes());
    }
    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, Error> {
        if r.byte()? != 1 {
            return Err(Error::Invalid);
        }
        let q = Self {
            kind: r.byte()?,
            cursor: r.array()?,
            limit: u16::from_be_bytes(r.array()?),
        };
        q.validate()?;
        Ok(q)
    }
    fn validate(self) -> Result<(), Error> {
        if self.kind >= KINDS
            || !(1..=MAX_PAGE as u16).contains(&self.limit)
            || (self.cursor[..32] == [0; 32]) != (self.cursor[32..] == [0; 8])
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
/// Finite projection; completeness is only for the explicitly retained range.
#[derive(Clone, PartialEq, Eq)]
pub struct Page {
    /// Requested family.
    pub kind: u8,
    /// Commitment to all permitted rows/query/account/signer/epoch/policy.
    pub revision: [u8; 32],
    /// Zero means no next retained row, not complete native history.
    pub next: [u8; 40],
    /// Qualified current read time, not native execution/commit time.
    pub evaluated_at: u64,
    /// Fixed family-specific private row schemas.
    pub rows: Vec<PrivateBytes>,
}
impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Page([PRIVATE])")
    }
}
impl Page {
    pub(crate) fn encode(&self, w: &mut Writer) -> Result<(), Error> {
        if self.rows.len() > MAX_PAGE {
            return Err(Error::Unavailable);
        }
        w.byte(1);
        w.byte(self.kind);
        w.raw(&self.revision);
        w.raw(&self.next);
        w.u64(self.evaluated_at);
        w.u64(self.rows.len() as u64);
        for row in &self.rows {
            w.raw(&(row.as_bytes().len() as u32).to_be_bytes());
            w.raw(row.as_bytes());
        }
        if w.0.len() > 1_048_576 {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
}
fn row(w: Writer) -> Result<PrivateBytes, Error> {
    PrivateBytes::new(w.0.to_vec()).map_err(|_| Error::Unavailable)
}
fn market(w: &mut Writer, m: MarketUnit) {
    w.raw(&m.market.bytes());
    w.raw(&m.precision.get().to_be_bytes());
}
fn optional(w: &mut Writer, n: Option<i128>) {
    w.byte(u8::from(n.is_some()));
    if let Some(n) = n {
        w.i128(n);
    }
}
impl<A: Admission> Service<A> {
    pub(crate) fn read(
        &self,
        journal: &impl Source,
        req: &Request,
        records: &[Record],
        q: Query,
        owner: bool,
        now: u64,
    ) -> Result<Page, Error> {
        q.validate()?;
        let state = journal.state().map_err(|_| Error::Unavailable)?;
        let book = state
            .ledger()
            .book(Owner::Customer(req.account))
            .map_err(|_| Error::Unavailable)?;
        let mut rows = Vec::new();
        match q.kind {
            0 | 9 => {
                let mut w = Writer::new(if q.kind == 9 { &[2] } else { &[] });
                w.u64(req.epoch);
                w.i128(book.cash().atoms());
                w.i128(book.funding().atoms());
                w.i128(
                    state
                        .reserved(Resource::Customer(req.account))
                        .map_err(|_| Error::Unavailable)?
                        .atoms(),
                );
                let risk = state.risk_report_at(now).ok();
                let customer = risk.as_ref().and_then(|r| {
                    r.current
                        .customers
                        .iter()
                        .find(|c| c.account == req.account)
                });
                w.byte(u8::from(customer.is_some()));
                if let Some(c) = customer {
                    for n in [
                        c.equity,
                        c.initial,
                        c.maintenance,
                        c.outcome_requirement,
                        c.other_held,
                        c.free,
                    ] {
                        w.i128(n.atoms());
                    }
                }
                let accounting = state.read_marks(now).ok().and_then(|marks| {
                    book.positions().iter().try_fold(
                        book.cash().atoms().checked_add(book.funding().atoms())?,
                        |sum, p| {
                            let m = journal
                                .configuration()
                                .markets
                                .iter()
                                .find(|m| m.unit() == p.quantity().unit())?;
                            let mark = marks.iter().find(|m| m.unit() == p.quantity().unit())?;
                            sum.checked_add(p.unrealized(*m, *mark).ok()?.atoms())
                        },
                    )
                });
                optional(&mut w, accounting);
                rows.push(row(w)?);
                if q.kind == 9 {
                    for p in book.positions() {
                        let mut w = Writer::new(&[0]);
                        market(&mut w, p.quantity().unit());
                        w.raw(&p.quantity().lots().to_be_bytes());
                        w.i128(p.basis().atoms());
                        rows.push(row(w)?);
                    }
                    for r in records.iter().filter(|r| r.request.account == req.account) {
                        let mut w = Writer::new(&[1]);
                        w.raw(
                            Response::Receipt(self.receipt(state, r)?)
                                .encode()?
                                .as_bytes(),
                        );
                        rows.push(row(w)?);
                    }
                }
            }
            1 => {
                let marks = state.read_marks(now).ok();
                for p in book.positions() {
                    let mut w = Writer::new(&[]);
                    market(&mut w, p.quantity().unit());
                    w.raw(&p.quantity().lots().to_be_bytes());
                    w.i128(p.basis().atoms());
                    let mark = marks
                        .as_ref()
                        .and_then(|m| m.iter().find(|m| m.unit() == p.quantity().unit()));
                    w.byte(u8::from(mark.is_some()));
                    if let Some(mark) = mark {
                        w.u64(mark.ticks());
                        let m = journal
                            .configuration()
                            .markets
                            .iter()
                            .find(|m| m.unit() == p.quantity().unit())
                            .ok_or(Error::Unavailable)?;
                        w.i128(
                            p.unrealized(*m, *mark)
                                .map_err(|_| Error::Unavailable)?
                                .atoms(),
                        );
                    }
                    let leverage = state.private_leverage(req.account, p.quantity().unit());
                    w.byte(u8::from(leverage.is_some()));
                    if let Some((cap, chosen)) = leverage {
                        w.u64(cap);
                        w.u64(chosen);
                    }
                    rows.push(row(w)?);
                }
            }
            2 | 3 => {
                for r in records.iter().filter(|r| r.request.account == req.account) {
                    // Account READ never becomes a directory of other agent keys.
                    if !owner && matches!(r.request.command, Command::Grant(_)) {
                        continue;
                    }
                    if q.kind == 3 && !matches!(r.request.command, Command::Order { .. }) {
                        continue;
                    }
                    let receipt = self.receipt(state, r)?;
                    if q.kind == 3
                        && matches!(
                            receipt.outcome,
                            crate::Outcome::Complete | crate::Outcome::Rejected
                        )
                    {
                        continue;
                    }
                    let mut w = Writer::new(&[]);
                    w.u64(r.at);
                    r.request.command.encode(&mut w);
                    w.raw(Response::Receipt(receipt).encode()?.as_bytes());
                    rows.push(row(w)?);
                }
            }
            4 => {
                for (_, e) in state.applied_order_fills() {
                    let observed = state.ledger().observations().iter().find(|o| {
                        o.event.key == e.key
                            && o.disposition
                                == cinder_kernel::ledger::evidence::Disposition::Applied
                    });
                    if observed.is_none() {
                        continue;
                    }
                    let (target, quantity, price, fee) = match &e.change {
                        Change::Fill {
                            target,
                            quantity,
                            price,
                        } => (*target, *quantity, *price, 0),
                        Change::Economics(EconomicChange::Execution {
                            target,
                            quantity,
                            price,
                            fee,
                            ..
                        }) => (*target, *quantity, *price, fee.atoms()),
                        _ => continue,
                    };
                    let FillTarget::Customer(attempt) = target else {
                        continue;
                    };
                    if attempt.request.account != req.account {
                        continue;
                    }
                    let mut w = Writer::new(&[]);
                    w.raw(&attempt.request.request.bytes());
                    market(&mut w, quantity.unit());
                    w.raw(&quantity.lots().to_be_bytes());
                    w.u64(price.ticks());
                    w.i128(fee);
                    let at = observed.map(|o| o.observed_at);
                    w.byte(u8::from(at.is_some()));
                    if let Some(at) = at {
                        w.u64(at);
                    }
                    rows.push(row(w)?);
                }
            }
            5 => {
                for f in state.ledger().funding_views(Owner::Customer(req.account)) {
                    let mut w = Writer::new(&[]);
                    market(&mut w, f.market);
                    w.raw(&f.lots.to_be_bytes());
                    optional(&mut w, f.payment.map(|p| p.atoms()));
                    w.byte(u8::from(f.settled));
                    let at = state
                        .ledger()
                        .observations()
                        .iter()
                        .find(|o| o.event.key == RecordKey::Economic(f.boundary.clone()))
                        .map(|o| o.observed_at);
                    w.byte(u8::from(at.is_some()));
                    if let Some(at) = at {
                        w.u64(at);
                    }
                    rows.push(row(w)?);
                }
            }
            6 => {
                for h in journal
                    .customer_book_history(req.account)
                    .map_err(|_| Error::Unavailable)?
                {
                    let mut w = Writer::new(&[]);
                    w.u64(h.at);
                    // A compound book transition is not mislabelled as a single
                    // deposit/funding/fee. Pre/post values also avoid delta overflow.
                    for b in [&h.before, &h.after] {
                        w.i128(b.cash().atoms());
                        w.i128(b.funding().atoms());
                        if b.positions().len() > MAX_PAGE {
                            return Err(Error::Unavailable);
                        }
                        w.u64(b.positions().len() as u64);
                        for p in b.positions() {
                            market(&mut w, p.quantity().unit());
                            w.raw(&p.quantity().lots().to_be_bytes());
                            w.i128(p.basis().atoms());
                        }
                    }
                    rows.push(row(w)?);
                }
            }
            7 => {
                for r in records.iter().filter(|r| {
                    r.accepted && r.request.account == req.account && r.request.epoch == req.epoch
                }) {
                    let Command::Grant(g) = &r.request.command else {
                        continue;
                    };
                    if now >= g.expires_at || !owner && g.key != req.signer {
                        continue;
                    }
                    let mut w = Writer::new(&[]);
                    Command::Grant(g.clone()).encode(&mut w);
                    w.u64(
                        records
                            .iter()
                            .filter(|r| {
                                r.accepted
                                    && r.request.account == req.account
                                    && r.request.epoch == req.epoch
                                    && r.request.signer == g.key
                                    && matches!(r.request.command, Command::Order { .. })
                            })
                            .count() as u64,
                    );
                    rows.push(row(w)?);
                }
            }
            8 => {
                for m in &journal.configuration().markets {
                    let mut w = Writer::new(&[]);
                    market(&mut w, m.unit());
                    w.raw(&m.unit().quote.asset.bytes());
                    w.raw(&m.unit().quote.precision.get().to_be_bytes());
                    let (n, d) = m.conversion();
                    w.u64(n);
                    w.u64(d);
                    w.byte(7);
                    let cap = state.private_leverage(req.account, m.unit()).map(|p| p.0);
                    w.byte(u8::from(cap.is_some()));
                    if let Some(cap) = cap {
                        w.u64(cap);
                    }
                    rows.push(row(w)?);
                }
            }
            _ => return Err(Error::Invalid),
        }
        let mut hash = Sha256::new();
        hash.update(b"CINDER-PRIVATE-READ-1\0");
        hash.update(req.domain.network.bytes());
        hash.update(req.domain.deployment.bytes());
        hash.update(req.account.bytes());
        hash.update(req.signer);
        hash.update(req.epoch.to_be_bytes());
        hash.update(req.policy.get().to_be_bytes());
        hash.update([q.kind]);
        for r in &rows {
            hash.update((r.as_bytes().len() as u64).to_be_bytes());
            hash.update(r.as_bytes());
        }
        let revision: [u8; 32] = hash.finalize().into();
        let start = if q.cursor == [0; 40] {
            0
        } else {
            if q.cursor[..32] != revision {
                return Err(Error::Conflict);
            }
            usize::try_from(u64::from_be_bytes(
                q.cursor[32..].try_into().map_err(|_| Error::Invalid)?,
            ))
            .map_err(|_| Error::Invalid)?
        };
        if start > rows.len() || start == rows.len() && start != 0 {
            return Err(Error::Invalid);
        }
        let end = start.saturating_add(q.limit as usize).min(rows.len());
        let mut next = [0; 40];
        if end < rows.len() {
            next[..32].copy_from_slice(&revision);
            next[32..].copy_from_slice(&(end as u64).to_be_bytes());
        }
        Ok(Page {
            kind: q.kind,
            revision,
            next,
            evaluated_at: now,
            rows: rows[start..end].to_vec(),
        })
    }
}
