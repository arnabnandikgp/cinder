//! Distinct new-history tags; legacy schema/engine decoding is not modified.
use super::*;
use crate::wire::{Reader, Writer};
use sha2::{Digest, Sha256};

pub(super) const GENESIS: u8 = 30;
pub(super) const RECORD: u8 = 31;
// Unit ownership/controller changes require a separate replay revision even
// when the reused single-unit financial engine remains unchanged.
const REVISION: u16 = 5;
pub(super) fn writer(tag: u8) -> Writer {
    let mut w = Writer::new(tag);
    w.raw(&REVISION.to_be_bytes());
    w.raw(&crate::ENGINE_REVISION.to_be_bytes());
    w
}
pub(super) fn reader(bytes: &[u8], tag: u8) -> Result<Reader<'_>, Error> {
    let mut r = Reader::new(bytes, tag)?;
    if r.array::<2>()? != REVISION.to_be_bytes()
        || r.array::<2>()? != crate::ENGINE_REVISION.to_be_bytes()
    {
        return Err(Error::Version);
    }
    Ok(r)
}

pub(super) fn configuration(c: &Configuration) -> Result<Vec<u8>, Error> {
    let mut w = writer(32);
    w.raw(&c.domain.network.bytes());
    w.raw(&c.domain.deployment.bytes());
    w.count(c.units.len());
    for u in &c.units {
        w.raw(&u.id.bytes());
        for b in [
            u.environment,
            u.native_account,
            u.funding_wallet,
            u.custody_config,
            u.payout_vault,
            u.key_policy,
        ] {
            w.raw(&b);
        }
        w.blob(&wire::encode_config(&u.ledger)?);
    }
    w.finish()
}

pub(super) fn transaction(tx: &Transaction) -> Result<Vec<u8>, Error> {
    let mut w = writer(33);
    w.raw(&tx.id.bytes());
    w.u64(tx.expected.sequence);
    w.raw(&tx.expected.hash);
    w.u64(tx.at);
    match &tx.change {
        Change::EnrollOwner {
            wallet,
            recipients,
            proof,
        } => {
            w.byte(6);
            w.raw(wallet);
            w.count(recipients.len());
            for r in recipients {
                w.raw(&r.unit.bytes());
                w.raw(&r.tokens);
            }
            w.blob(proof.as_bytes());
        }
        Change::BeginRecovery {
            versions,
            authority_epoch,
            valid_until,
            fence,
        } => {
            w.byte(5);
            w.count(versions.len());
            for (unit, version) in versions {
                w.raw(&unit.bytes());
                w.u64(*version);
            }
            w.u64(*authority_epoch);
            w.u64(*valid_until);
            w.raw(fence);
        }
        Change::Admit(a) => {
            w.byte(4);
            api_record(&mut w, &a.record);
            match a.kind {
                api::Kind::Order => w.byte(0),
                api::Kind::Cancel { target, attempt } => {
                    w.byte(1);
                    w.item(&target);
                    w.raw(&attempt.bytes());
                }
                api::Kind::Payout => w.byte(2),
                api::Kind::Leverage => w.byte(3),
            }
            w.blob(&wire::encode_transaction(&a.proposal)?);
        }
        Change::Api(record) => {
            w.byte(3);
            api_record(&mut w, record);
        }
        Change::Enroll { wallet, recipients } => {
            w.byte(0);
            w.raw(wallet);
            w.count(recipients.len());
            for r in recipients {
                w.raw(&r.unit.bytes());
                w.raw(&r.tokens);
            }
        }
        Change::AdvanceAuthority { account, epoch } => {
            w.byte(1);
            w.raw(&account.bytes());
            w.u64(*epoch);
        }
        Change::Unit { unit, proposal } => {
            w.byte(2);
            w.raw(&unit.bytes());
            w.blob(&wire::encode_transaction(proposal)?);
        }
    }
    w.finish()
}

pub(super) fn decode_transaction(bytes: &[u8]) -> Result<Transaction, Error> {
    let mut r = reader(bytes, 33)?;
    let id = model::CommitId::new(r.array()?)?;
    let expected = Head {
        sequence: r.u64()?,
        hash: r.array()?,
    };
    let at = r.u64()?;
    let change = match r.byte()? {
        6 => {
            let wallet = r.array()?;
            let n = r.count()?;
            if n > MAX_UNITS {
                return Err(Error::Limit);
            }
            let mut recipients = Vec::with_capacity(n);
            for _ in 0..n {
                recipients.push(Recipient {
                    unit: UnitId::new(r.array()?).map_err(|_| Error::Codec)?,
                    tokens: r.array()?,
                });
            }
            Change::EnrollOwner {
                wallet,
                recipients,
                proof: model::PrivateBytes::new(r.blob()?.to_vec())?,
            }
        }
        5 => {
            let n = r.count()?;
            if n > MAX_UNITS {
                return Err(Error::Limit);
            }
            let mut versions = Vec::with_capacity(n);
            for _ in 0..n {
                versions.push((UnitId::new(r.array()?).map_err(|_| Error::Codec)?, r.u64()?));
            }
            Change::BeginRecovery {
                versions,
                authority_epoch: r.u64()?,
                valid_until: r.u64()?,
                fence: r.array()?,
            }
        }
        4 => {
            let record = read_api_record(&mut r)?;
            let kind = match r.byte()? {
                0 => api::Kind::Order,
                1 => api::Kind::Cancel {
                    target: r.item()?,
                    attempt: AttemptId::new(r.array()?).map_err(|_| Error::Codec)?,
                },
                2 => api::Kind::Payout,
                3 => api::Kind::Leverage,
                _ => return Err(Error::Codec),
            };
            Change::Admit(Box::new(api::Admission {
                record,
                kind,
                proposal: wire::decode_transaction(r.blob()?)?,
            }))
        }
        3 => Change::Api(Box::new(read_api_record(&mut r)?)),
        0 => {
            let wallet = r.array()?;
            let count = r.count()?;
            if count > MAX_UNITS {
                return Err(Error::Limit);
            }
            let mut recipients = Vec::with_capacity(count);
            for _ in 0..count {
                recipients.push(Recipient {
                    unit: UnitId::new(r.array()?).map_err(|_| Error::Codec)?,
                    tokens: r.array()?,
                });
            }
            Change::Enroll { wallet, recipients }
        }
        1 => Change::AdvanceAuthority {
            account: AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
            epoch: r.u64()?,
        },
        2 => Change::Unit {
            unit: UnitId::new(r.array()?).map_err(|_| Error::Codec)?,
            proposal: Box::new(wire::decode_transaction(r.blob()?)?),
        },
        _ => return Err(Error::Codec),
    };
    r.done()?;
    let tx = Transaction {
        id,
        expected,
        at,
        change,
    };
    if transaction(&tx)? != bytes {
        return Err(Error::Codec);
    }
    Ok(tx)
}

pub(super) fn receipt(receipt: &Receipt) -> Result<Vec<u8>, Error> {
    let mut w = writer(34);
    match receipt {
        Receipt::Admitted { unit, receipt } => {
            w.byte(3);
            w.raw(&unit.bytes());
            w.blob(&wire::receipt(receipt)?);
        }
        Receipt::Api => w.byte(2),
        Receipt::Shared => w.byte(0),
        Receipt::Unit { unit, receipt } => {
            w.byte(1);
            w.raw(&unit.bytes());
            w.blob(&wire::receipt(receipt)?);
        }
    }
    w.finish()
}

pub(super) fn state(s: &State) -> Result<[u8; 32], Error> {
    let mut w = writer(35);
    w.blob(&configuration(&s.config)?);
    w.u64(s.now);
    w.count(s.api.len());
    for accepted in &s.api {
        let mut record = writer(37);
        api_record(&mut record, &accepted.record);
        record.u64(accepted.at);
        w.raw(&Sha256::digest(record.finish()?));
    }
    w.count(s.decisions.len());
    for decision in &s.decisions {
        w.item(&decision.request);
        w.u64(decision.at);
        w.blob(&wire::receipt(&decision.receipt)?);
    }
    w.count(s.customers.len());
    for c in &s.customers {
        w.raw(&c.account.bytes());
        w.raw(&c.wallet);
        w.u64(c.epoch);
    }
    w.count(s.routes.len());
    for r in &s.routes {
        w.raw(&r.request.account.bytes());
        w.raw(&r.request.request.bytes());
        w.raw(&r.unit.bytes());
        w.raw(&r.reservation_digest);
    }
    w.count(s.units.len());
    for (id, state) in &s.units {
        w.raw(&id.bytes());
        w.raw(&wire::state_commitment(state)?);
    }
    Ok(Sha256::digest(w.finish()?).into())
}

fn api_record(w: &mut Writer, a: &api::Record) {
    w.raw(&a.unit.bytes());
    w.raw(&a.request.domain.network.bytes());
    w.raw(&a.request.domain.deployment.bytes());
    w.raw(&a.request.account.bytes());
    w.raw(&a.request.request.bytes());
    w.raw(&a.policy.get().to_be_bytes());
    w.raw(&a.digest);
    w.u64(a.epoch);
    w.raw(&a.signer);
    w.raw(&a.contract);
    w.byte(match a.effect {
        api::Effect::Recorded => 0,
        api::Effect::Grant => 1,
        api::Effect::Revoke => 2,
    });
    w.blob(a.wire.as_bytes());
}
fn read_api_record(r: &mut Reader<'_>) -> Result<api::Record, Error> {
    Ok(api::Record {
        unit: UnitId::new(r.array()?).map_err(|_| Error::Codec)?,
        request: RequestKey {
            domain: Domain {
                network: NetworkId::new(r.array()?).map_err(|_| Error::Codec)?,
                deployment: DeploymentId::new(r.array()?).map_err(|_| Error::Codec)?,
            },
            account: AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
            request: RequestId::new(r.array()?).map_err(|_| Error::Codec)?,
        },
        policy: PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)?,
        digest: r.array()?,
        epoch: r.u64()?,
        signer: r.array()?,
        contract: r.array()?,
        effect: match r.byte()? {
            0 => api::Effect::Recorded,
            1 => api::Effect::Grant,
            2 => api::Effect::Revoke,
            _ => return Err(Error::Codec),
        },
        wire: model::PrivateBytes::new(r.blob()?.to_vec())?,
    })
}

pub(super) fn reservation(control: &model::Control) -> Result<[u8; 32], Error> {
    let tx = model::Transaction {
        id: model::CommitId::new([1; 32])?,
        expected: Head::default(),
        at: 0,
        evidence: vec![],
        inputs: vec![],
        order_observations: vec![],
        funds_observations: vec![],
        controls: vec![control.clone()],
    };
    let mut w = writer(36);
    w.blob(&wire::encode_transaction(&tx)?);
    Ok(Sha256::digest(w.finish()?).into())
}
