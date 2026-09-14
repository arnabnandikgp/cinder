use super::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
struct ReserveRootV0 {
    epoch: u64,
    root: [u8; 32],
    user_count: u32,
    total_free: u64,
    total_reserved: u64,
    book_hash: [u8; 32],
    committed_at_base_slot: u64,
}

pub const RESERVE_ROOT_V0_SPACE: usize = 8 + ReserveRootV0::INIT_SPACE;

pub fn migrate_reserve_root_handler(ctx: Context<MigrateReserveRoot>) -> Result<()> {
    let info = ctx.accounts.reserve_root.to_account_info();
    let old = read_legacy_reserve_root(&info)?;
    let migrated = convert_reserve_root(old);

    fund_and_resize(
        &ctx.accounts.admin.to_account_info(),
        &info,
        &ctx.accounts.system_program.to_account_info(),
        ReserveRoot::ACCOUNT_SPACE,
    )?;
    write_account(&info, migrated.as_ref())
}

#[inline(never)]
fn convert_reserve_root(old: Box<ReserveRootV0>) -> Box<ReserveRoot> {
    Box::new(ReserveRoot {
        schema_version: cc::ACCOUNT_SCHEMA_VERSION,
        epoch: old.epoch,
        root: old.root,
        user_count: old.user_count,
        total_free: old.total_free,
        total_reserved: old.total_reserved,
        total_bad_debt: 0,
        book_hash: old.book_hash,
        committed_at_base_slot: old.committed_at_base_slot,
    })
}

#[inline(never)]
fn read_legacy_reserve_root(info: &AccountInfo) -> Result<Box<ReserveRootV0>> {
    let data = info.try_borrow_data()?;
    if data.len() == ReserveRoot::ACCOUNT_SPACE {
        return err!(VaultError::AccountAlreadyMigrated);
    }
    require!(
        data.len() == RESERVE_ROOT_V0_SPACE,
        VaultError::UnsupportedAccountSchema
    );
    require!(
        data.starts_with(ReserveRoot::DISCRIMINATOR),
        VaultError::InvalidMigrationData
    );

    let mut payload = &data[8..];
    let value = ReserveRootV0::deserialize(&mut payload)
        .map_err(|_| error!(VaultError::InvalidMigrationData))?;
    require!(payload.is_empty(), VaultError::InvalidMigrationData);
    Ok(Box::new(value))
}

fn fund_and_resize<'info>(
    payer: &AccountInfo<'info>,
    account: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    new_space: usize,
) -> Result<()> {
    let required = Rent::get()?.minimum_balance(new_space);
    let shortfall = required.saturating_sub(account.lamports());
    if shortfall > 0 {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                system_program.key(),
                anchor_lang::system_program::Transfer {
                    from: payer.clone(),
                    to: account.clone(),
                },
            ),
            shortfall,
        )?;
    }
    account.resize(new_space)?;
    Ok(())
}

fn write_account<T: AccountSerialize>(info: &AccountInfo, value: &T) -> Result<()> {
    let mut data = info.try_borrow_mut_data()?;
    let mut destination = &mut data[..];
    value.try_serialize(&mut destination)?;
    require!(destination.is_empty(), VaultError::InvalidMigrationData);
    Ok(())
}

#[derive(Accounts)]
pub struct MigrateReserveRoot<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump = config.bump_config,
        has_one = admin @ VaultError::Unauthorized
    )]
    pub config: Account<'info, Config>,
    /// CHECK: legacy bytes and the fixed ReserveRoot PDA are verified.
    #[account(
        mut,
        seeds = [cc::SEED_RESERVE],
        bump,
        owner = crate::ID
    )]
    pub reserve_root: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_layout_sizes_are_stable() {
        assert_eq!(RESERVE_ROOT_V0_SPACE, 108);
        assert_eq!(ReserveRoot::ACCOUNT_SPACE, 117);
    }

    #[test]
    fn conversion_preserves_legacy_totals() {
        let old = ReserveRootV0 {
            epoch: 3,
            root: [1; 32],
            user_count: 4,
            total_free: 5,
            total_reserved: 6,
            book_hash: [7; 32],
            committed_at_base_slot: 8,
        };
        let migrated = convert_reserve_root(Box::new(old));

        assert_eq!(migrated.schema_version, 1);
        assert_eq!(migrated.epoch, 3);
        assert_eq!(migrated.total_free, 5);
        assert_eq!(migrated.total_reserved, 6);
        assert_eq!(migrated.total_bad_debt, 0);
        assert_eq!(migrated.committed_at_base_slot, 8);
    }

    #[test]
    fn legacy_decoder_accepts_exact_v0_and_rejects_current_size() {
        let old = ReserveRootV0 {
            epoch: 13,
            root: [1; 32],
            user_count: 2,
            total_free: 3,
            total_reserved: 4,
            book_hash: [5; 32],
            committed_at_base_slot: 6,
        };
        let key = Pubkey::new_unique();
        let owner = crate::ID;
        let mut lamports = 1;
        let mut data = vec![0u8; RESERVE_ROOT_V0_SPACE];
        data[..8].copy_from_slice(ReserveRoot::DISCRIMINATOR);
        old.serialize(&mut &mut data[8..]).unwrap();
        let info = AccountInfo::new(&key, false, true, &mut lamports, &mut data, &owner, false);
        let decoded = read_legacy_reserve_root(&info).unwrap();
        assert_eq!(decoded.epoch, 13);
        drop(info);

        let mut current_data = vec![0u8; ReserveRoot::ACCOUNT_SPACE];
        let current_info = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut current_data,
            &owner,
            false,
        );
        assert!(read_legacy_reserve_root(&current_info).is_err());
    }
}
