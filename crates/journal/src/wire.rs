//! Versioned, bounded canonical journal and kernel-event encoding.

use crate::Error;
use crate::{Head, model::*};
use cinder_kernel::{
    amounts::*,
    codec::Canonical,
    identity::*,
    ledger::{economics::*, evidence::*, funds::*, protection::*, *},
    math::Rounding,
    position::*,
};

/// Maximum plaintext or opaque record length. Oversized records fail closed.
pub const MAX_RECORD: usize = 1_048_576;
/// Maximum elements in any one encoded collection.
pub const MAX_ITEMS: usize = 1024;

pub(crate) struct Writer {
    bytes: Vec<u8>,
    valid: bool,
}
impl Writer {
    pub(crate) fn new(tag: u8) -> Self {
        let mut s = Self {
            bytes: Vec::new(),
            valid: true,
        };
        s.raw(b"CINDER-J\0");
        s.raw(&8_u16.to_be_bytes());
        s.byte(tag);
        s
    }
    pub(crate) fn raw(&mut self, b: &[u8]) {
        if b.len() > MAX_RECORD.saturating_sub(self.bytes.len()) {
            self.valid = false;
        } else {
            self.bytes.extend_from_slice(b);
        }
    }
    pub(crate) fn byte(&mut self, b: u8) {
        self.raw(&[b]);
    }
    pub(crate) fn invalid(&mut self) {
        self.valid = false;
    }
    pub(crate) fn u64(&mut self, n: u64) {
        self.raw(&n.to_be_bytes());
    }
    pub(crate) fn count(&mut self, n: usize) {
        self.valid &= n <= MAX_ITEMS;
        self.u64(n as u64);
    }
    pub(crate) fn blob(&mut self, b: &[u8]) {
        self.u64(b.len() as u64);
        self.raw(b);
    }
    pub(crate) fn item<T: Canonical>(&mut self, v: &T) {
        self.blob(&v.encode());
    }
    pub(crate) fn option<T>(&mut self, v: &Option<T>, f: impl FnOnce(&mut Self, &T)) {
        match v {
            Some(v) => {
                self.byte(1);
                f(self, v);
            }
            None => self.byte(0),
        }
    }
    pub(crate) fn finish(self) -> Result<Vec<u8>, Error> {
        if self.valid {
            Ok(self.bytes)
        } else {
            Err(Error::Limit)
        }
    }
}
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    pub(crate) fn bool(&mut self) -> Result<bool, Error> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::Codec),
        }
    }
    pub(crate) fn new(bytes: &'a [u8], tag: u8) -> Result<Self, Error> {
        if bytes.len() > MAX_RECORD {
            return Err(Error::Limit);
        }
        let mut r = Self { bytes, offset: 0 };
        if r.take(9)? != b"CINDER-J\0" {
            return Err(Error::Codec);
        }
        if r.array::<2>()? != 8_u16.to_be_bytes() {
            return Err(Error::Version);
        }
        if r.byte()? != tag {
            return Err(Error::Codec);
        }
        Ok(r)
    }
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.offset.checked_add(n).ok_or(Error::Limit)?;
        let b = self.bytes.get(self.offset..end).ok_or(Error::Codec)?;
        self.offset = end;
        Ok(b)
    }
    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        self.take(N)?.try_into().map_err(|_| Error::Codec)
    }
    pub(crate) fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.array::<1>()?[0])
    }
    pub(crate) fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    pub(crate) fn count(&mut self) -> Result<usize, Error> {
        let n = usize::try_from(self.u64()?).map_err(|_| Error::Limit)?;
        if n > MAX_ITEMS {
            Err(Error::Limit)
        } else {
            Ok(n)
        }
    }
    pub(crate) fn blob(&mut self) -> Result<&'a [u8], Error> {
        let n = usize::try_from(self.u64()?).map_err(|_| Error::Limit)?;
        if n > MAX_RECORD {
            return Err(Error::Limit);
        }
        self.take(n)
    }
    pub(crate) fn item<T: Canonical>(&mut self) -> Result<T, Error> {
        T::decode(self.blob()?).map_err(|_| Error::Codec)
    }
    pub(crate) fn option<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<Option<T>, Error> {
        match self.byte()? {
            0 => Ok(None),
            1 => f(self).map(Some),
            _ => Err(Error::Codec),
        }
    }
    pub(crate) fn done(self) -> Result<(), Error> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::Codec)
        }
    }
}
fn domain(w: &mut Writer, d: Domain) {
    w.raw(&d.network.bytes());
    w.raw(&d.deployment.bytes());
}
fn read_domain(r: &mut Reader<'_>) -> Result<Domain, Error> {
    Ok(Domain {
        network: NetworkId::new(r.array()?).map_err(|_| Error::Codec)?,
        deployment: DeploymentId::new(r.array()?).map_err(|_| Error::Codec)?,
    })
}
fn asset(w: &mut Writer, a: AssetUnit) {
    w.item(&QuoteAtoms::new(a, 0));
}
fn read_asset(r: &mut Reader<'_>) -> Result<AssetUnit, Error> {
    let value: QuoteAtoms = r.item()?;
    if value.atoms() != 0 {
        return Err(Error::Codec);
    }
    Ok(value.unit())
}
fn market(w: &mut Writer, m: MarketUnit) {
    w.item(&QuantityLots::new(m, 0));
}
fn read_market(r: &mut Reader<'_>) -> Result<MarketUnit, Error> {
    let value: QuantityLots = r.item()?;
    if value.lots() != 0 {
        return Err(Error::Codec);
    }
    Ok(value.unit())
}
pub(crate) fn owner(w: &mut Writer, o: Owner) {
    match o {
        Owner::Customer(id) => {
            w.byte(0);
            w.raw(&id.bytes());
        }
        Owner::House => w.byte(1),
        Owner::Suspense => w.byte(2),
    }
}
pub(crate) fn read_owner(r: &mut Reader<'_>) -> Result<Owner, Error> {
    match r.byte()? {
        0 => Ok(Owner::Customer(
            AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
        )),
        1 => Ok(Owner::House),
        2 => Ok(Owner::Suspense),
        _ => Err(Error::Codec),
    }
}
pub(crate) fn location(w: &mut Writer, l: Location) {
    w.byte(match l {
        Location::Vault => 0,
        Location::Venue => 1,
    });
}
pub(crate) fn read_location(r: &mut Reader<'_>) -> Result<Location, Error> {
    match r.byte()? {
        0 => Ok(Location::Vault),
        1 => Ok(Location::Venue),
        _ => Err(Error::Codec),
    }
}
fn target(w: &mut Writer, t: FillTarget) {
    match t {
        FillTarget::Customer(k) => {
            w.byte(0);
            w.item(&k);
        }
        FillTarget::House => w.byte(1),
        FillTarget::Unattributed => w.byte(2),
    }
}
fn read_target(r: &mut Reader<'_>) -> Result<FillTarget, Error> {
    match r.byte()? {
        0 => Ok(FillTarget::Customer(r.item()?)),
        1 => Ok(FillTarget::House),
        2 => Ok(FillTarget::Unattributed),
        _ => Err(Error::Codec),
    }
}
pub(crate) fn pnl(w: &mut Writer, p: NativePnl) {
    match p {
        NativePnl::Gross(v) => {
            w.byte(0);
            w.item(&v);
        }
        NativePnl::NetOfFee(v) => {
            w.byte(1);
            w.item(&v);
        }
    }
}
pub(crate) fn read_pnl(r: &mut Reader<'_>) -> Result<NativePnl, Error> {
    match r.byte()? {
        0 => Ok(NativePnl::Gross(r.item()?)),
        1 => Ok(NativePnl::NetOfFee(r.item()?)),
        _ => Err(Error::Codec),
    }
}

/// Complete initial configuration, including exact native price conversions.
pub fn encode_config(c: &Config) -> Result<Vec<u8>, Error> {
    let mut w = Writer::new(1);
    domain(&mut w, c.domain);
    w.raw(&c.policy.get().to_be_bytes());
    asset(&mut w, c.quote);
    w.raw(&c.venue.bytes());
    w.raw(&c.venue_account.bytes());
    w.count(c.sources.len());
    for s in &c.sources {
        domain(&mut w, s.scope.domain);
        w.raw(&s.scope.venue.bytes());
        w.raw(&s.scope.account.bytes());
        w.raw(&s.scope.namespace.bytes());
        location(&mut w, s.location);
    }
    w.count(c.markets.len());
    for m in &c.markets {
        market(&mut w, m.unit());
        let (n, d) = m.conversion();
        w.u64(n);
        w.u64(d);
    }
    w.count(c.customers.len());
    for id in &c.customers {
        w.raw(&id.bytes());
    }
    w.finish()
}
/// Decode structure and validate the kernel's complete configuration constraints.
pub fn decode_config(bytes: &[u8]) -> Result<Config, Error> {
    let mut r = Reader::new(bytes, 1)?;
    let domain = read_domain(&mut r)?;
    let policy = PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)?;
    let quote = read_asset(&mut r)?;
    let venue = VenueId::new(r.array()?).map_err(|_| Error::Codec)?;
    let venue_account = VenueAccountId::new(r.array()?).map_err(|_| Error::Codec)?;
    let mut sources = Vec::new();
    for _ in 0..r.count()? {
        sources.push(Source {
            scope: EventScope {
                domain: read_domain(&mut r)?,
                venue: VenueId::new(r.array()?).map_err(|_| Error::Codec)?,
                account: VenueAccountId::new(r.array()?).map_err(|_| Error::Codec)?,
                namespace: NamespaceId::new(r.array()?).map_err(|_| Error::Codec)?,
            },
            location: read_location(&mut r)?,
        });
    }
    let mut markets = Vec::new();
    for _ in 0..r.count()? {
        markets
            .push(Market::new(read_market(&mut r)?, r.u64()?, r.u64()?).map_err(|_| Error::Codec)?);
    }
    let mut customers = Vec::new();
    for _ in 0..r.count()? {
        customers.push(AccountId::new(r.array()?).map_err(|_| Error::Codec)?);
    }
    r.done()?;
    let c = Config {
        domain,
        policy,
        quote,
        venue,
        venue_account,
        sources,
        markets,
        customers,
    };
    Ledger::new(c.clone()).map_err(|_| Error::Codec)?;
    Ok(c)
}
/// Complete P03/P04 semantic event. Arrival metadata belongs to the journal envelope.
pub fn encode_event(e: &Event) -> Result<Vec<u8>, Error> {
    let mut w = Writer::new(2);
    w.item(&e.key);
    w.raw(&e.policy.get().to_be_bytes());
    match &e.change {
        Change::Restoration(c) => {
            w.byte(10);
            crate::restoration::encode_change(&mut w, c);
        }
        Change::Close(change) => {
            w.byte(9);
            match change {
                close::CloseChange::Bind { house, quantity } => {
                    w.byte(0);
                    w.byte(u8::from(*house));
                    w.item(quantity);
                }
                close::CloseChange::Execution {
                    attempt,
                    quantity,
                    price,
                    fee,
                    pnl: p,
                    customer_allowed,
                } => {
                    w.byte(1);
                    w.item(attempt);
                    w.item(quantity);
                    w.item(price);
                    w.item(fee);
                    w.option(p, |w, p| pnl(w, *p));
                    w.byte(u8::from(*customer_allowed));
                }
            }
        }
        Change::Protection(change) => {
            w.byte(8);
            encode_protection(&mut w, change);
        }
        Change::Funds(change) => {
            w.byte(7);
            encode_funds(&mut w, change);
        }
        Change::BindExecution { market: m, side } => {
            w.byte(0);
            market(&mut w, *m);
            w.byte(match side {
                Side::Buy => 0,
                Side::Sell => 1,
            });
        }
        Change::Receipt {
            owner: o,
            location: l,
            amount,
        } => {
            w.byte(1);
            owner(&mut w, *o);
            location(&mut w, *l);
            w.item(amount);
        }
        Change::Fill {
            target: t,
            quantity,
            price,
        } => {
            w.byte(2);
            target(&mut w, *t);
            w.item(quantity);
            w.item(price);
        }
        Change::TransferDebit {
            source,
            destination,
            amount,
        } => {
            w.byte(3);
            location(&mut w, *source);
            location(&mut w, *destination);
            w.item(amount);
        }
        Change::TransferArrival { debit } => {
            w.byte(4);
            w.item(debit);
        }
        Change::Economics(c) => {
            w.byte(5);
            match c {
                EconomicChange::Execution {
                    target: t,
                    quantity,
                    price,
                    fee,
                    pnl: p,
                } => {
                    w.byte(0);
                    target(&mut w, *t);
                    w.item(quantity);
                    w.item(price);
                    w.item(fee);
                    w.option(p, |w, p| pnl(w, *p));
                }
                EconomicChange::CorrectExecutionPnl { execution, pnl: p } => {
                    w.byte(1);
                    w.item(execution);
                    pnl(&mut w, *p);
                }
                EconomicChange::BrokerFee { customer, amount } => {
                    w.byte(2);
                    w.raw(&customer.bytes());
                    w.item(amount);
                }
                EconomicChange::FundingBoundary {
                    market: m,
                    expected_version,
                } => {
                    w.byte(3);
                    market(&mut w, *m);
                    w.u64(*expected_version);
                }
                EconomicChange::FundingInputs {
                    boundary,
                    rate,
                    native,
                } => {
                    w.byte(4);
                    w.item(boundary);
                    w.option(rate, |w, r| {
                        w.item(&r.numerator);
                        w.u64(r.denominator);
                        w.byte(match r.rounding {
                            Rounding::Exact => 0,
                            Rounding::TowardZero => 1,
                            Rounding::Floor => 2,
                            Rounding::Ceil => 3,
                        });
                    });
                    w.option(native, |w, v| w.item(v));
                }
                EconomicChange::FundingSettlement { boundary, native } => {
                    w.byte(5);
                    w.item(boundary);
                    w.item(native);
                }
                EconomicChange::CorrectFundingNative { boundary, native } => {
                    w.byte(6);
                    w.item(boundary);
                    w.item(native);
                }
            }
        }
        Change::Reconcile(c) => {
            w.byte(6);
            w.u64(c.expected_version);
            w.option(&c.cash, |w, v| w.item(v));
            w.option(&c.funding, |w, v| w.item(v));
            w.option(&c.positions, |w, ps| {
                w.count(ps.len());
                for p in ps {
                    w.item(&p.quantity());
                    w.item(&p.basis());
                }
            });
            w.byte(u8::from(c.complete));
            w.count(c.resolves.len());
            for a in &c.resolves {
                w.item(&a.check);
                w.count(a.applied_effects.len());
                for e in &a.applied_effects {
                    w.item(e);
                }
            }
        }
    }
    w.finish()
}
/// Lossless event decoding. Semantically rejected facts still decode for retention.
pub fn decode_event(bytes: &[u8]) -> Result<Event, Error> {
    let mut r = Reader::new(bytes, 2)?;
    let key = r.item()?;
    let policy = PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)?;
    let change = match r.byte()? {
        10 => Change::Restoration(crate::restoration::decode_change(&mut r)?),
        9 => Change::Close(match r.byte()? {
            0 => close::CloseChange::Bind {
                house: r.bool()?,
                quantity: r.item()?,
            },
            1 => close::CloseChange::Execution {
                attempt: r.item()?,
                quantity: r.item()?,
                price: r.item()?,
                fee: r.item()?,
                pnl: r.option(read_pnl)?,
                customer_allowed: r.bool()?,
            },
            _ => return Err(Error::Codec),
        }),
        8 => Change::Protection(decode_protection(&mut r)?),
        7 => Change::Funds(decode_funds(&mut r)?),
        0 => Change::BindExecution {
            market: read_market(&mut r)?,
            side: match r.byte()? {
                0 => Side::Buy,
                1 => Side::Sell,
                _ => return Err(Error::Codec),
            },
        },
        1 => Change::Receipt {
            owner: read_owner(&mut r)?,
            location: read_location(&mut r)?,
            amount: r.item()?,
        },
        2 => Change::Fill {
            target: read_target(&mut r)?,
            quantity: r.item()?,
            price: r.item()?,
        },
        3 => Change::TransferDebit {
            source: read_location(&mut r)?,
            destination: read_location(&mut r)?,
            amount: r.item()?,
        },
        4 => Change::TransferArrival { debit: r.item()? },
        5 => Change::Economics(match r.byte()? {
            0 => EconomicChange::Execution {
                target: read_target(&mut r)?,
                quantity: r.item()?,
                price: r.item()?,
                fee: r.item()?,
                pnl: r.option(read_pnl)?,
            },
            1 => EconomicChange::CorrectExecutionPnl {
                execution: r.item()?,
                pnl: read_pnl(&mut r)?,
            },
            2 => EconomicChange::BrokerFee {
                customer: AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
                amount: r.item()?,
            },
            3 => EconomicChange::FundingBoundary {
                market: read_market(&mut r)?,
                expected_version: r.u64()?,
            },
            4 => EconomicChange::FundingInputs {
                boundary: r.item()?,
                rate: r.option(|r| {
                    Ok(FundingRate {
                        numerator: r.item()?,
                        denominator: r.u64()?,
                        rounding: match r.byte()? {
                            0 => Rounding::Exact,
                            1 => Rounding::TowardZero,
                            2 => Rounding::Floor,
                            3 => Rounding::Ceil,
                            _ => return Err(Error::Codec),
                        },
                    })
                })?,
                native: r.option(|r| r.item())?,
            },
            5 => EconomicChange::FundingSettlement {
                boundary: r.item()?,
                native: r.item()?,
            },
            6 => EconomicChange::CorrectFundingNative {
                boundary: r.item()?,
                native: r.item()?,
            },
            _ => return Err(Error::Codec),
        }),
        6 => {
            let expected_version = r.u64()?;
            let cash = r.option(|r| r.item())?;
            let funding = r.option(|r| r.item())?;
            let positions = r.option(|r| {
                let mut ps = Vec::new();
                for _ in 0..r.count()? {
                    ps.push(Position::new(r.item()?, r.item()?).map_err(|_| Error::Codec)?);
                }
                Ok(ps)
            })?;
            let complete = match r.byte()? {
                0 => false,
                1 => true,
                _ => return Err(Error::Codec),
            };
            let mut resolves = Vec::new();
            for _ in 0..r.count()? {
                let check = r.item()?;
                let mut applied_effects = Vec::new();
                for _ in 0..r.count()? {
                    applied_effects.push(r.item()?);
                }
                resolves.push(CheckResolution {
                    check,
                    applied_effects,
                });
            }
            Change::Reconcile(NativeCheck {
                expected_version,
                cash,
                funding,
                positions,
                complete,
                resolves,
            })
        }
        _ => return Err(Error::Codec),
    };
    r.done()?;
    Ok(Event {
        key,
        policy,
        change,
    })
}

pub(crate) fn head(w: &mut Writer, h: Head) {
    w.u64(h.sequence);
    w.raw(&h.hash);
}

pub(crate) fn destination(w: &mut Writer, d: Destination) {
    match d {
        Destination::Location(l) => {
            w.byte(0);
            location(w, l);
        }
        Destination::Recipient(key) => {
            w.byte(1);
            w.raw(&key);
        }
    }
}
pub(crate) fn read_destination(r: &mut Reader<'_>) -> Result<Destination, Error> {
    match r.byte()? {
        0 => Ok(Destination::Location(read_location(r)?)),
        1 => Ok(Destination::Recipient(r.array()?)),
        _ => Err(Error::Codec),
    }
}
pub(crate) fn mandate(w: &mut Writer, m: &Mandate) {
    w.item(&m.attempt);
    location(w, m.source);
    destination(w, m.destination);
    w.item(&m.net);
    w.item(&m.maximum_fee);
    owner(w, m.fee_payer);
}
pub(crate) fn read_mandate(r: &mut Reader<'_>) -> Result<Mandate, Error> {
    Ok(Mandate {
        attempt: r.item()?,
        source: read_location(r)?,
        destination: read_destination(r)?,
        net: r.item()?,
        maximum_fee: r.item()?,
        fee_payer: read_owner(r)?,
    })
}
pub(crate) fn encode_protection(w: &mut Writer, p: &ProtectionChange) {
    match p {
        ProtectionChange::Designate { delta } => {
            w.byte(0);
            w.item(delta);
        }
        ProtectionChange::Recognize {
            kind,
            cause,
            amount,
        } => {
            w.byte(1);
            w.byte(match kind {
                Kind::Deficit => 0,
                Kind::Remediation => 1,
            });
            w.raw(cause);
            w.item(amount);
        }
        ProtectionChange::Commit { claim, amount } => {
            w.byte(2);
            w.item(claim);
            w.item(amount);
        }
        ProtectionChange::Absorb { allocations } => {
            w.byte(3);
            w.count(allocations.len());
            for a in allocations {
                w.item(&a.claim);
                w.item(&a.amount);
            }
        }
    }
}
pub(crate) fn decode_protection(r: &mut Reader<'_>) -> Result<ProtectionChange, Error> {
    Ok(match r.byte()? {
        0 => ProtectionChange::Designate { delta: r.item()? },
        1 => ProtectionChange::Recognize {
            kind: match r.byte()? {
                0 => Kind::Deficit,
                1 => Kind::Remediation,
                _ => return Err(Error::Codec),
            },
            cause: r.array()?,
            amount: r.item()?,
        },
        2 => ProtectionChange::Commit {
            claim: r.item()?,
            amount: r.item()?,
        },
        3 => {
            let mut allocations = vec![];
            for _ in 0..r.count()? {
                allocations.push(Allocation {
                    claim: r.item()?,
                    amount: r.item()?,
                });
            }
            ProtectionChange::Absorb { allocations }
        }
        _ => return Err(Error::Codec),
    })
}

fn encode_funds(w: &mut Writer, f: &FundsChange) {
    match f {
        FundsChange::Authorize(m) => {
            w.byte(0);
            mandate(w, m);
        }
        FundsChange::Observe {
            attempt,
            leg,
            amount,
            fee,
        } => {
            w.byte(1);
            w.item(attempt);
            match leg {
                Leg::Debit => w.byte(0),
                Leg::Arrive(d) => {
                    w.byte(1);
                    destination(w, *d);
                }
                Leg::Return => w.byte(2),
                Leg::Impair => w.byte(3),
            }
            w.item(amount);
            w.item(fee);
        }
    }
}
fn decode_funds(r: &mut Reader<'_>) -> Result<FundsChange, Error> {
    match r.byte()? {
        0 => Ok(FundsChange::Authorize(read_mandate(r)?)),
        1 => {
            let attempt = r.item()?;
            let leg = match r.byte()? {
                0 => Leg::Debit,
                1 => Leg::Arrive(read_destination(r)?),
                2 => Leg::Return,
                3 => Leg::Impair,
                _ => return Err(Error::Codec),
            };
            Ok(FundsChange::Observe {
                attempt,
                leg,
                amount: r.item()?,
                fee: r.item()?,
            })
        }
        _ => Err(Error::Codec),
    }
}
pub(crate) fn read_head(r: &mut Reader<'_>) -> Result<Head, Error> {
    Ok(Head {
        sequence: r.u64()?,
        hash: r.array()?,
    })
}
pub(crate) fn scope(w: &mut Writer, s: EventScope) {
    domain(w, s.domain);
    w.raw(&s.venue.bytes());
    w.raw(&s.account.bytes());
    w.raw(&s.namespace.bytes());
}
pub(crate) fn read_scope(r: &mut Reader<'_>) -> Result<EventScope, Error> {
    Ok(EventScope {
        domain: read_domain(r)?,
        venue: VenueId::new(r.array()?).map_err(|_| Error::Codec)?,
        account: VenueAccountId::new(r.array()?).map_err(|_| Error::Codec)?,
        namespace: NamespaceId::new(r.array()?).map_err(|_| Error::Codec)?,
    })
}
fn resource(w: &mut Writer, r: Resource) {
    match r {
        Resource::Customer(id) => {
            w.byte(0);
            w.raw(&id.bytes());
        }
        Resource::House => w.byte(1),
        Resource::Location(l) => {
            w.byte(2);
            location(w, l);
        }
    }
}
fn read_resource(r: &mut Reader<'_>) -> Result<Resource, Error> {
    match r.byte()? {
        0 => Ok(Resource::Customer(
            AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
        )),
        1 => Ok(Resource::House),
        2 => Ok(Resource::Location(read_location(r)?)),
        _ => Err(Error::Codec),
    }
}
pub(crate) fn reservations(w: &mut Writer, rs: &[Reservation]) {
    w.count(rs.len());
    for r in rs {
        resource(w, r.resource);
        w.item(&r.amount);
    }
}
pub(crate) fn read_reservations(r: &mut Reader<'_>) -> Result<Vec<Reservation>, Error> {
    let mut values = Vec::new();
    for _ in 0..r.count()? {
        values.push(Reservation {
            resource: read_resource(r)?,
            amount: r.item()?,
        });
    }
    Ok(values)
}

/// Full input fingerprint, including exact body, source, time, epoch, cut and proposal.
/// This identifies evidence; it proves neither source authenticity nor no effect.
pub fn input_fingerprint(input: &Input) -> Result<[u8; 32], Error> {
    use sha2::{Digest, Sha256};
    let mut w = Writer::new(79);
    scope(&mut w, input.source);
    w.option(&input.source_cut, |w, cut| w.u64(*cut));
    w.u64(input.authority_epoch);
    w.u64(input.observed_at);
    w.blob(input.raw.as_bytes());
    w.option(&input.event, |w, event| match encode_event(event) {
        Ok(bytes) => w.blob(&bytes),
        Err(_) => w.invalid(),
    });
    let bytes = PrivateBytes::new(w.finish()?)?;
    Ok(Sha256::digest(bytes.as_bytes()).into())
}

/// Complete immutable transaction bytes, including exact raw evidence/action material.
/// Returned bytes are private and must pass through Protection before external storage.
pub fn encode_transaction(tx: &Transaction) -> Result<Vec<u8>, Error> {
    let mut w = Writer::new(3);
    w.raw(&tx.id.bytes());
    head(&mut w, tx.expected);
    w.u64(tx.at);
    w.count(tx.evidence.len());
    for body in &tx.evidence {
        w.blob(body.as_bytes());
    }
    w.count(tx.inputs.len());
    for i in &tx.inputs {
        scope(&mut w, i.source);
        w.option(&i.source_cut, |w, v| w.u64(*v));
        w.u64(i.authority_epoch);
        w.u64(i.observed_at);
        w.blob(i.raw.as_bytes());
        match &i.event {
            None => w.byte(0),
            Some(e) => {
                w.byte(1);
                w.blob(&encode_event(e)?);
            }
        }
    }
    w.count(tx.order_observations.len());
    for o in &tx.order_observations {
        w.item(&o.key);
        w.item(&o.attempt);
        crate::orders::encode_status(&mut w, &o.status);
        w.u64(o.authority_epoch);
        w.u64(o.observed_at);
        w.blob(o.raw.as_bytes());
    }
    w.count(tx.funds_observations.len());
    for o in &tx.funds_observations {
        w.item(&o.key);
        crate::funds::encode_terminal(&mut w, &o.terminal);
        w.u64(o.authority_epoch);
        w.u64(o.observed_at);
        w.blob(o.raw.as_bytes());
    }
    w.count(tx.controls.len());
    for c in &tx.controls {
        match c {
            Control::ResolveRaw(resolution) => {
                w.byte(11);
                crate::raw::encode_resolution(&mut w, resolution);
            }
            Control::Restoration(a) => {
                w.byte(10);
                crate::restoration::encode_action(&mut w, a);
            }
            Control::Liquidation(action) => {
                w.byte(9);
                crate::liquidation::encode_action(&mut w, action);
            }
            Control::Protection(action) => {
                w.byte(8);
                crate::protection::encode_action(&mut w, action);
            }
            Control::Risk(action) => {
                w.byte(7);
                crate::risk::encode_action(&mut w, action);
            }
            Control::Funds(action) => {
                w.byte(6);
                crate::funds::encode_action(&mut w, action);
            }
            Control::Collateral(cut) => {
                w.byte(5);
                crate::collateral::encode(&mut w, cut);
            }
            Control::Order(action) => {
                w.byte(4);
                crate::orders::encode_action(&mut w, action);
            }
            Control::Reserve {
                request,
                reservations: rs,
            } => {
                w.byte(0);
                w.item(request);
                reservations(&mut w, rs);
            }
            Control::Release(k) => {
                w.byte(1);
                w.item(k);
            }
            Control::Prepare {
                key,
                message,
                authority_epoch,
                expires_at,
            } => {
                w.byte(2);
                w.item(key);
                w.blob(message.as_bytes());
                w.u64(*authority_epoch);
                w.u64(*expires_at);
            }
            Control::Expose(k) => {
                w.byte(3);
                w.item(k);
            }
        }
    }
    w.finish()
}
/// Decode all transaction fields with bounded allocation and exact canonical enums.
pub fn decode_transaction(bytes: &[u8]) -> Result<Transaction, Error> {
    let mut r = Reader::new(bytes, 3)?;
    let id = CommitId::new(r.array()?)?;
    let expected = read_head(&mut r)?;
    let at = r.u64()?;
    let mut evidence = Vec::new();
    for _ in 0..r.count()? {
        evidence.push(PrivateBytes::new(r.blob()?.to_vec())?);
    }
    let mut inputs = Vec::new();
    for _ in 0..r.count()? {
        inputs.push(Input {
            source: read_scope(&mut r)?,
            source_cut: r.option(|r| r.u64())?,
            authority_epoch: r.u64()?,
            observed_at: r.u64()?,
            raw: PrivateBytes::new(r.blob()?.to_vec())?,
            event: r.option(|r| decode_event(r.blob()?))?,
        });
    }
    let mut order_observations = Vec::new();
    for _ in 0..r.count()? {
        order_observations.push(crate::orders::Observation {
            key: r.item()?,
            attempt: r.item()?,
            status: crate::orders::decode_status(&mut r)?,
            authority_epoch: r.u64()?,
            observed_at: r.u64()?,
            raw: PrivateBytes::new(r.blob()?.to_vec())?,
        });
    }
    let mut funds_observations = Vec::new();
    for _ in 0..r.count()? {
        funds_observations.push(crate::funds::Observation {
            key: r.item()?,
            terminal: crate::funds::decode_terminal(&mut r)?,
            authority_epoch: r.u64()?,
            observed_at: r.u64()?,
            raw: PrivateBytes::new(r.blob()?.to_vec())?,
        });
    }
    let mut controls = Vec::new();
    for _ in 0..r.count()? {
        controls.push(match r.byte()? {
            0 => Control::Reserve {
                request: r.item()?,
                reservations: read_reservations(&mut r)?,
            },
            1 => Control::Release(r.item()?),
            2 => Control::Prepare {
                key: r.item()?,
                message: PrivateBytes::new(r.blob()?.to_vec())?,
                authority_epoch: r.u64()?,
                expires_at: r.u64()?,
            },
            3 => Control::Expose(r.item()?),
            4 => Control::Order(crate::orders::decode_action(&mut r)?),
            5 => Control::Collateral(crate::collateral::decode(&mut r)?),
            6 => Control::Funds(crate::funds::decode_action(&mut r)?),
            7 => Control::Risk(crate::risk::decode_action(&mut r)?),
            8 => Control::Protection(crate::protection::decode_action(&mut r)?),
            9 => Control::Liquidation(crate::liquidation::decode_action(&mut r)?),
            10 => Control::Restoration(crate::restoration::decode_action(&mut r)?),
            11 => Control::ResolveRaw(crate::raw::decode_resolution(&mut r)?),
            _ => return Err(Error::Codec),
        });
    }
    r.done()?;
    Ok(Transaction {
        id,
        expected,
        at,
        evidence,
        inputs,
        order_observations,
        funds_observations,
        controls,
    })
}
fn primitive_error(e: cinder_kernel::Error) -> u8 {
    use cinder_kernel::Error::*;
    match e {
        Overflow => 0,
        DivisionByZero => 1,
        InvalidSign => 2,
        InvalidDecimal => 3,
        Overprecision => 4,
        Inexact => 5,
        UnitMismatch => 6,
        InvalidId => 7,
        UnknownVersion => 8,
        InvalidEncoding => 9,
        ScopeMismatch => 10,
        InvalidRatio => 11,
    }
}
fn ledger_error(w: &mut Writer, e: LedgerError) {
    use LedgerError::*;
    match e {
        Primitive(p) => {
            w.byte(0);
            w.byte(primitive_error(p));
        }
        StaleCut => w.byte(1),
        FundingState => w.byte(2),
        Evidence => w.byte(3),
        Restricted => w.byte(4),
        Configuration => w.byte(5),
        UnknownIdentity => w.byte(6),
        Attribution => w.byte(7),
        Conflict => w.byte(8),
        PhysicalShortfall => w.byte(9),
        Transfer => w.byte(10),
        Bridge => w.byte(11),
        Valuation => w.byte(12),
    }
}
fn disposition(w: &mut Writer, d: Disposition) {
    match d {
        Disposition::Applied => w.byte(0),
        Disposition::Duplicate => w.byte(1),
        Disposition::Rejected(e) => {
            w.byte(2);
            ledger_error(w, e);
        }
    }
}
pub(crate) fn receipt(r: &Receipt) -> Result<Vec<u8>, Error> {
    let mut w = Writer::new(4);
    w.u64(r.ledger_version);
    w.count(r.inputs.len());
    for i in &r.inputs {
        match i {
            InputResult::Unnormalized => w.byte(0),
            InputResult::EnvelopeRejected => w.byte(1),
            InputResult::Normalized(d) => {
                w.byte(2);
                disposition(&mut w, *d);
            }
        }
    }
    w.count(r.order_observations.len());
    for ok in &r.order_observations {
        w.byte(u8::from(*ok));
    }
    w.option(&r.controls, |w, e| {
        w.byte(match e {
            ControlError::Invalid => 0,
            ControlError::Capacity => 1,
            ControlError::Unqualified => 2,
            ControlError::Exposed => 3,
            ControlError::Expired => 4,
        })
    });
    w.count(r.funds_observations.len());
    for ok in &r.funds_observations {
        w.byte(u8::from(*ok));
    }
    w.finish()
}
pub(crate) fn state_commitment(s: &State) -> Result<[u8; 32], Error> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"CINDER-STATE-1\0");
    h.update(encode_config(&s.config)?);
    let mut w = Writer::new(5);
    w.u64(s.now);
    w.u64(s.raw_unresolved);
    crate::raw::encode_entries(&mut w, &s.raw_inputs);
    w.option(&s.collateral, crate::collateral::encode);
    w.option(&s.risk, crate::risk::encode_policy);
    w.option(&s.protection_policy, crate::protection::encode_policy);
    w.option(&s.liquidation, crate::liquidation::encode_policy);
    crate::restoration::encode_state(&mut w, s);
    w.count(s.closes.len());
    for c in &s.closes {
        crate::liquidation::encode_close(&mut w, c);
    }
    w.count(s.selections.len());
    for selection in &s.selections {
        crate::risk::encode_selection(&mut w, selection);
    }
    w.u64(s.ledger.version());
    w.count(s.ledger.closes().len());
    for c in s.ledger.closes() {
        w.item(&c.attempt);
        w.byte(u8::from(c.house));
        w.item(&c.quantity);
        w.u64(c.received);
        w.u64(c.customer_filled);
        w.u64(c.excess);
        w.item(&c.spent);
    }
    let protection = s.ledger.protection();
    w.byte(u8::from(protection.active));
    w.item(&protection.reserve);
    w.item(&protection.absorbed_total);
    w.count(protection.claims.len());
    for c in &protection.claims {
        w.item(&c.id);
        w.byte(match c.kind {
            Kind::Deficit => 0,
            Kind::Remediation => 1,
        });
        w.raw(&c.cause);
        for v in [
            c.amount,
            c.committed,
            c.absorbed,
            c.recovered,
            c.offset,
            c.replenished,
        ] {
            w.item(&v);
        }
    }
    w.count(protection.receipt_faults.len());
    for k in &protection.receipt_faults {
        w.item(k);
    }
    w.item(&s.ledger.vault());
    w.item(&s.ledger.in_transit().map_err(|_| Error::Invalid)?);
    w.item(&s.ledger.unpaired().map_err(|_| Error::Invalid)?);
    w.count(s.ledger.movements().len());
    for m in s.ledger.movements() {
        mandate(&mut w, &m.mandate);
        for amount in [
            m.debit,
            m.settled,
            m.arrived,
            m.returned,
            m.impaired,
            m.fees,
            m.customer_fees,
            m.paid,
            m.sent,
        ] {
            w.item(&amount);
        }
        w.byte(u8::from(m.faulted));
        w.count(m.receipts.len());
        for k in &m.receipts {
            w.item(k);
        }
    }
    w.u64(s.ledger.unresolved_attribution() as u64);
    for b in s
        .config
        .customers
        .iter()
        .map(|id| s.ledger.book(Owner::Customer(*id)))
        .chain([
            s.ledger.book(Owner::House),
            s.ledger.book(Owner::Suspense),
            Ok(s.ledger.venue()),
        ])
    {
        let b = b.map_err(|_| Error::Invalid)?;
        w.item(&b.cash());
        w.item(&b.funding());
        w.count(b.positions().len());
        for p in b.positions() {
            w.item(&p.quantity());
            w.item(&p.basis());
        }
    }
    w.count(s.holds.len());
    for hold in &s.holds {
        w.item(&hold.request);
        w.byte(u8::from(hold.active));
        reservations(&mut w, &hold.reservations);
    }
    w.count(s.attempts.len());
    for a in &s.attempts {
        w.byte(match a.kind {
            AttemptKind::Restoration => 5,
            AttemptKind::Emergency => 4,
            AttemptKind::Funds => 3,
            AttemptKind::Generic => 0,
            AttemptKind::Order => 1,
            AttemptKind::Cancel => 2,
        });
        w.item(&a.key);
        w.blob(a.message.as_bytes());
        w.u64(a.authority_epoch);
        w.u64(a.expires_at);
        w.byte(u8::from(a.possibly_exposed));
        w.option(&a.exposed_at, |w, t| w.u64(*t));
    }
    w.count(s.authorities.len());
    for (a, epoch) in &s.authorities {
        w.raw(&a.bytes());
        w.u64(*epoch);
    }
    w.count(s.orders.len());
    for o in &s.orders {
        w.byte(u8::from(o.abandoned));
        crate::orders::encode_intent(&mut w, &o.intent);
        w.option(&o.attempt, |w, a| w.item(a));
        w.item(&o.filled);
        w.count(o.executions.len());
        for (key, cut) in &o.executions {
            w.item(key);
            w.option(cut, |w, c| w.u64(*c));
        }
        w.byte(u8::from(o.acknowledged));
        w.byte(u8::from(o.cancel_acknowledged));
        w.byte(u8::from(o.unknown));
        w.byte(u8::from(o.faulted));
        w.byte(u8::from(o.bound_violated));
        w.option(&o.terminal, |w, t| {
            crate::orders::encode_status(w, &crate::orders::Status::Terminal(t.clone()))
        });
    }
    w.count(s.order_observations.len());
    for o in &s.order_observations {
        w.item(&o.key);
        w.item(&o.attempt);
        crate::orders::encode_status(&mut w, &o.status);
        w.u64(o.authority_epoch);
        w.u64(o.observed_at);
        w.blob(o.raw.as_bytes());
    }
    w.count(s.order_fills.len());
    for (original, classified) in &s.order_fills {
        w.blob(&encode_event(original)?);
        w.blob(&encode_event(classified)?);
    }
    w.byte(u8::from(s.frozen));
    w.count(s.funds.len());
    for o in &s.funds {
        crate::funds::encode_intent(&mut w, &o.intent);
        w.option(&o.attempt, |w, a| w.item(a));
        w.option(&o.proof, crate::funds::encode_terminal);
        w.byte(u8::from(o.terminal));
        w.byte(u8::from(o.faulted));
    }
    w.count(s.funds_observations.len());
    for o in &s.funds_observations {
        w.item(&o.key);
        crate::funds::encode_terminal(&mut w, &o.terminal);
        w.u64(o.authority_epoch);
        w.u64(o.observed_at);
        w.blob(o.raw.as_bytes());
    }
    w.count(s.funds_receipts.len());
    for (k, c) in &s.funds_receipts {
        w.item(k);
        w.option(c, |w, c| w.u64(*c));
    }
    h.update(w.finish()?);
    h.update((s.ledger.events().len() as u64).to_be_bytes());
    for e in s.ledger.events() {
        let b = encode_event(e)?;
        h.update((b.len() as u64).to_be_bytes());
        h.update(b);
    }
    h.update((s.ledger.observations().len() as u64).to_be_bytes());
    for o in s.ledger.observations() {
        let b = encode_event(&o.event)?;
        h.update((b.len() as u64).to_be_bytes());
        h.update(b);
        let mut w = Writer::new(6);
        w.u64(o.observed_at);
        disposition(&mut w, o.disposition);
        h.update(w.finish()?);
    }
    h.update((s.ledger.issues().len() as u64).to_be_bytes());
    for i in s.ledger.issues() {
        let mut w = Writer::new(7);
        w.item(&i.subject);
        w.byte(match i.kind {
            IssueKind::NativePnl => 0,
            IssueKind::FundingInputs => 1,
            IssueKind::FundingDifference => 2,
            IssueKind::NativeSnapshot => 3,
            IssueKind::Rejected(_) => 4,
            IssueKind::ReplayConflict => 5,
        });
        if let IssueKind::Rejected(e) = i.kind {
            ledger_error(&mut w, e);
        }
        w.u64(i.first_seen);
        w.u64(i.last_seen);
        w.byte(u8::from(i.open));
        w.option(&i.difference, |w, v| w.item(v));
        h.update(w.finish()?);
    }
    Ok(h.finalize().into())
}
