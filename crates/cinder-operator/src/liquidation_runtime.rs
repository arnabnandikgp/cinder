//! Whole-user liquidation selection and atomic restrictive halt mirroring.
use crate::journal::maintenance_writes::MaintenanceParameters as P;
use crate::rpc::{Result, RuntimeError};
use crate::runtime::{book_key, config_key, decode, ledger_id, text, vault_id};
use crate::transaction::{anchor_ix, send, sign};
use crate::OperatorRuntime;
use anchor_lang::InstructionData;
use cinder_common as cc;
use cinder_ledger::{Book, UserLedger};
use solana_signer::Signer;

impl OperatorRuntime {
    pub(crate) fn mirror_halts(&mut self, flags: u8) -> Result<bool> {
        let mut changed = false;
        for _ in 0..4 {
            {
                let mut c = self.context.borrow_mut();
                let c = &mut *c;
                let cfg = c.vault_config()?;
                let (_, rows) = c.qfs.accounts(&[text(&book_key())], &c.signer)?;
                let book: Book = decode(&rows[0], &text(&ledger_id()))?;
                if book.schema_version != cc::ACCOUNT_SCHEMA_VERSION {
                    return Err(RuntimeError::Identity);
                }
                let union = cfg.paused | book.halt | flags;
                if cfg.paused == union && book.halt == union {
                    return Ok(changed);
                }
                changed = true;
                let adapter = c.signer.pubkey().to_bytes();
                // Idempotent atomic OR, not a stale absolute assignment. Unknown
                // nonfinancial additions cannot undo any concurrent money write.
                if cfg.paused != union {
                    let ix = anchor_ix(
                        vault_id(),
                        vec![(adapter, true, false), (config_key(), false, true)],
                        cinder_vault::instruction::AddOperatorHalt { flags: union }.data(),
                    );
                    let tx = sign(&mut c.l1, &c.signer, ix)?;
                    send(&mut c.l1, &c.signer, &tx)?;
                }
                if book.halt != union {
                    let ix = anchor_ix(
                        ledger_id(),
                        vec![
                            (adapter, true, false),
                            (config_key(), false, false),
                            (book_key(), false, true),
                        ],
                        cinder_ledger::instruction::AddOperatorHalt { flags: union }.data(),
                    );
                    let tx = sign(&mut c.qfs, &c.signer, ix)?;
                    send(&mut c.qfs, &c.signer, &tx)?;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Err(RuntimeError::Incomplete)
    }

    pub(crate) fn initiate_liquidation(
        &mut self,
        policy: &crate::MaintenancePolicy,
        book: &Book,
        users: &[([u8; 32], UserLedger)],
        view: &crate::rise::RiseView,
        health: &[crate::maintenance::UserHealth],
    ) -> Result<bool> {
        if (book.halt | view.halt) & (cc::INVARIANT_BROKEN | cc::VENUE_BREACH) != 0 {
            return Ok(false);
        }
        let Some(candidate) = health.iter().find(|h| {
            (h.below_mm()
                || view.risk_tier >= 2
                    && h.effective_equity_usdc < i128::from(h.initial_margin_usdc))
                && h.confirmed_positions.iter().any(|(_, l)| *l != 0)
        }) else {
            return Ok(false);
        };
        let (asset, lots) = candidate
            .confirmed_positions
            .iter()
            .filter(|(_, l)| *l != 0)
            .max_by_key(|(_, l)| l.unsigned_abs())
            .copied()
            .ok_or(RuntimeError::Identity)?;
        let config = self.context.borrow();
        let execution = config
            .config
            .execution_policy
            .as_ref()
            .ok_or(RuntimeError::Configuration)?;
        let scenarios = config
            .config
            .solvency_policy
            .as_ref()
            .ok_or(RuntimeError::Configuration)?
            .scenarios
            .len();
        let cap = 65_536usize
            .checked_div(
                users
                    .len()
                    .checked_mul(scenarios + 1)
                    .ok_or(RuntimeError::Decode)?,
            )
            .and_then(|n| n.checked_sub(1))
            .ok_or(RuntimeError::Unsupported)?;
        let quantity = lots
            .unsigned_abs()
            .min(u64::from(execution.max_admission_lots))
            .min(cap as u64);
        drop(config);
        if quantity == 0 {
            return Err(RuntimeError::Unsupported);
        }
        let l = &users
            .iter()
            .find(|(a, _)| *a == candidate.ledger)
            .ok_or(RuntimeError::Identity)?
            .1;
        if l.pending_oid_count != 0 {
            return Err(RuntimeError::Incomplete);
        }
        let post = lots
            .checked_add(
                i64::try_from(quantity).map_err(|_| RuntimeError::Decode)? * (-lots.signum()),
            )
            .ok_or(RuntimeError::Decode)?;
        let entry = l.positions[..l.positions_len as usize]
            .iter()
            .find(|p| p.asset_id == asset)
            .ok_or(RuntimeError::Identity)?
            .entry_quote_lots;
        let post_im = crate::rise::user_margin(view, l, asset, post, entry)?;
        let mark = view
            .markets
            .get(&asset)
            .ok_or(RuntimeError::Incomplete)?
            .mark_price
            .as_inner();
        let numerator = u128::from(mark)
            * if lots > 0 {
                u128::from(cc::BPS_DENOM - u64::from(policy.liquidation_slippage_bps))
            } else {
                u128::from(cc::BPS_DENOM + u64::from(policy.liquidation_slippage_bps))
            };
        let bound = u64::try_from(if lots > 0 {
            numerator / u128::from(cc::BPS_DENOM)
        } else {
            numerator.div_ceil(u128::from(cc::BPS_DENOM))
        })
        .map_err(|_| RuntimeError::Decode)?;
        if bound == 0 {
            return Err(RuntimeError::Configuration);
        }
        let deadline = view
            .slot
            .checked_add(policy.liquidation_deadline_slots)
            .ok_or(RuntimeError::Decode)?;
        let oid = solana_keypair::Keypair::new().pubkey().to_bytes()[..16]
            .try_into()
            .map_err(|_| RuntimeError::Decode)?;
        self.maintenance_send(
            candidate.ledger,
            cinder_ledger::instruction::LiquidateUserBounded {
                asset_id: asset,
                client_oid: oid,
                limit_price_ticks: bound,
                last_valid_slot: deadline,
                close_lots: quantity,
                post_position_im_usdc: post_im,
            }
            .data(),
            P::Liquidate {
                asset,
                oid,
                bound,
                deadline,
                close_lots: quantity,
                post_im,
            },
            book,
            users,
            view.financial_fingerprint(),
        )?;
        Ok(true)
    }
}
