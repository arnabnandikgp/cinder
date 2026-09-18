//! Fee readiness is independent of native Phoenix execution. The validator
//! owns magic-vault provisioning; the operator must not impersonate it or spend
//! user USDC on SOL fees. Long-session startup refuses missing provisioning.
use crate::rpc::{data, number, Result, RuntimeError};
use crate::runtime::text;
use crate::{MaintenancePolicy, OperatorRuntime};
use solana_signer::Signer;

impl OperatorRuntime {
    pub(crate) fn monitor_per_fees(&mut self, p: &MaintenancePolicy) -> Result<()> {
        let mut c = self.context.borrow_mut();
        let c = &mut *c;
        let cfg = c.vault_config()?;
        let dlp = dlp_api::id().to_bytes();
        let payer = dlp_api::compat::Pubkey::new_from_array(c.signer.pubkey().to_bytes());
        let validator = dlp_api::compat::Pubkey::new_from_array(cfg.er_validator.to_bytes());
        let balance =
            dlp_api::pda::ephemeral_balance_pda_from_payer(&payer, p.fee_balance_index).to_bytes();
        let vault = dlp_api::pda::magic_fee_vault_pda_from_validator(&validator).to_bytes();
        let records = [balance, vault].map(|a| {
            dlp_api::pda::delegation_record_pda_from_delegated_account(
                &dlp_api::compat::Pubkey::new_from_array(a),
            )
            .to_bytes()
        });
        let (_, base) = c.l1.accounts(&records.map(|a| text(&a)), &c.signer)?;
        for (i, row) in base.iter().enumerate() {
            let bytes = data(row, &text(&dlp))?;
            let record =
                dlp_api::state::DelegationRecord::try_from_bytes_with_discriminator(&bytes)
                    .map_err(|_| RuntimeError::Identity)?;
            if record.authority.to_bytes() != validator.to_bytes()
                || record.owner.to_bytes() != if i == 0 { [0; 32] } else { dlp }
            {
                return Err(RuntimeError::Identity);
            }
        }
        let (_, rows) = c.qfs.accounts(&[text(&balance), text(&vault)], &c.signer)?;
        if !data(&rows[0], "11111111111111111111111111111111")?.is_empty()
            || rows[0]["executable"] != false
            || rows[1]["executable"] != false
            || data(&rows[1], &text(&dlp))?.len() != 8
            || number(&rows[0]["lamports"])? < p.minimum_fee_balance_lamports
            || number(&rows[1]["lamports"])? < p.minimum_magic_vault_lamports
        {
            return Err(RuntimeError::Incomplete);
        }
        Ok(())
    }
}
