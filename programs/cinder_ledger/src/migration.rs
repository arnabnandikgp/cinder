use super::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
struct OpenOidV0 {
    client_oid: [u8; 16],
    asset_id: u16,
    lots_delta: i64,
    state: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
struct UserLedgerV0 {
    user: Pubkey,
    free: u64,
    reserved: u64,
    withdrawable: u64,
    pending_oid_count: u8,
    nonce: u64,
    last_funding_epoch: u64,
    positions_len: u8,
    positions: [Position; 16],
    open_oids: [OpenOidV0; 8],
    bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
struct BookV0 {
    residual_len: u8,
    residuals: [Residual; 32],
    phoenix_collateral: u64,
    last_ack_slot_er: u64,
    invariant_ok: u8,
    halt: u8,
    funding_epoch: u64,
    last_scan_ms: u64,
    bump: u8,
}

pub const USER_LEDGER_V0_SPACE: usize = 8 + UserLedgerV0::INIT_SPACE;
pub const BOOK_V0_SPACE: usize = 8 + BookV0::INIT_SPACE;

pub fn migrate_user_ledger_handler(ctx: Context<MigrateUserLedger>) -> Result<()> {
    let cfg = load_vault_config(&ctx.accounts.config)?;
    require_keys_eq!(
        cfg.adapter,
        ctx.accounts.adapter.key(),
        LedgerError::Unauthorized
    );

    let info = ctx.accounts.user_ledger.to_account_info();
    let old: Box<UserLedgerV0> = read_legacy_account(
        &info,
        UserLedger::DISCRIMINATOR,
        USER_LEDGER_V0_SPACE,
        UserLedger::ACCOUNT_SPACE,
    )?;
    require!(
        old.positions_len as usize <= cc::MAX_USER_POSITIONS,
        LedgerError::InvalidMigrationData
    );
    require!(
        old.pending_oid_count as usize <= cc::MAX_OPEN_OIDS_PER_USER,
        LedgerError::InvalidMigrationData
    );

    let (expected, bump) =
        Pubkey::find_program_address(&[cc::SEED_USER, old.user.as_ref()], &crate::ID);
    require_keys_eq!(
        expected,
        ctx.accounts.user_ledger.key(),
        LedgerError::InvalidMigrationAccount
    );
    require!(bump == old.bump, LedgerError::InvalidMigrationAccount);

    let migrated = convert_user_ledger(old);

    resize_prefunded(&info, UserLedger::ACCOUNT_SPACE)?;
    write_account(&info, migrated.as_ref())
}

#[inline(never)]
fn convert_user_ledger(old: Box<UserLedgerV0>) -> Box<UserLedger> {
    Box::new(UserLedger {
        schema_version: cc::ACCOUNT_SCHEMA_VERSION,
        user: old.user,
        free: old.free,
        reserved: old.reserved,
        withdrawable: old.withdrawable,
        bad_debt_usdc: 0,
        pending_oid_count: old.pending_oid_count,
        nonce: old.nonce,
        last_funding_epoch: old.last_funding_epoch,
        positions_len: old.positions_len,
        positions: old.positions,
        open_oids: old.open_oids.map(|oid| OpenOid {
            client_oid: oid.client_oid,
            asset_id: oid.asset_id,
            lots_delta: oid.lots_delta,
            state: oid.state,
            limit_price_ticks: 0,
            last_valid_slot: 0,
        }),
        bump: old.bump,
    })
}

pub fn migrate_book_handler(ctx: Context<MigrateBook>) -> Result<()> {
    let cfg = load_vault_config(&ctx.accounts.config)?;
    require_keys_eq!(
        cfg.adapter,
        ctx.accounts.adapter.key(),
        LedgerError::Unauthorized
    );

    let info = ctx.accounts.book.to_account_info();
    let old: Box<BookV0> = read_legacy_account(
        &info,
        Book::DISCRIMINATOR,
        BOOK_V0_SPACE,
        Book::ACCOUNT_SPACE,
    )?;
    require!(
        old.residual_len as usize <= cc::MAX_BOOK_MARKETS,
        LedgerError::InvalidMigrationData
    );

    let (expected, bump) = Pubkey::find_program_address(&[cc::SEED_BOOK], &crate::ID);
    require_keys_eq!(
        expected,
        ctx.accounts.book.key(),
        LedgerError::InvalidMigrationAccount
    );
    require!(bump == old.bump, LedgerError::InvalidMigrationAccount);

    let migrated = convert_book(old);

    resize_prefunded(&info, Book::ACCOUNT_SPACE)?;
    write_account(&info, migrated.as_ref())
}

#[inline(never)]
fn convert_book(old: Box<BookV0>) -> Box<Book> {
    Box::new(Book {
        schema_version: cc::ACCOUNT_SCHEMA_VERSION,
        residual_len: old.residual_len,
        residuals: old.residuals,
        phoenix_collateral: old.phoenix_collateral,
        last_ack_slot_er: old.last_ack_slot_er,
        invariant_ok: old.invariant_ok,
        halt: old.halt,
        funding_epoch: old.funding_epoch,
        last_scan_ms: old.last_scan_ms,
        bump: old.bump,
    })
}

#[inline(never)]
fn read_legacy_account<T: AnchorDeserialize>(
    info: &AccountInfo,
    discriminator: &[u8],
    legacy_space: usize,
    current_space: usize,
) -> Result<Box<T>> {
    let data = info.try_borrow_data()?;
    if data.len() == current_space {
        return err!(LedgerError::AccountAlreadyMigrated);
    }
    require!(
        data.len() == legacy_space,
        LedgerError::UnsupportedAccountSchema
    );
    require!(
        data.starts_with(discriminator),
        LedgerError::InvalidMigrationData
    );

    let mut payload = &data[8..];
    let value =
        T::deserialize(&mut payload).map_err(|_| error!(LedgerError::InvalidMigrationData))?;
    require!(payload.is_empty(), LedgerError::InvalidMigrationData);
    Ok(Box::new(value))
}

fn resize_prefunded(info: &AccountInfo, new_space: usize) -> Result<()> {
    let minimum = Rent::get()?.minimum_balance(new_space);
    require!(
        info.lamports() >= minimum,
        LedgerError::MigrationRentShortfall
    );
    info.resize(new_space)?;
    Ok(())
}

fn write_account<T: AccountSerialize>(info: &AccountInfo, value: &T) -> Result<()> {
    let mut data = info.try_borrow_mut_data()?;
    let mut destination = &mut data[..];
    value.try_serialize(&mut destination)?;
    require!(destination.is_empty(), LedgerError::InvalidMigrationData);
    Ok(())
}

#[derive(Accounts)]
pub struct MigrateUserLedger<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: vault Config; its owner, PDA, and discriminator are verified.
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    /// CHECK: legacy bytes are decoded and the user PDA and bump are verified.
    #[account(mut, owner = crate::ID)]
    pub user_ledger: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct MigrateBook<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: vault Config; its owner, PDA, and discriminator are verified.
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    /// CHECK: legacy bytes are decoded and the Book PDA and bump are verified.
    #[account(mut, seeds = [cc::SEED_BOOK], bump, owner = crate::ID)]
    pub book: UncheckedAccount<'info>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_sizes_are_stable() {
        assert_eq!(USER_LEDGER_V0_SPACE, 843);
        assert_eq!(UserLedger::ACCOUNT_SPACE, 980);
        assert_eq!(BOOK_V0_SPACE, 364);
        assert_eq!(Book::ACCOUNT_SPACE, 365);
    }

    #[test]
    fn user_conversion_preserves_legacy_state() {
        let user = Pubkey::new_unique();
        let position = Position {
            asset_id: 7,
            lots: -9,
            entry_quote_lots: 12,
            unsettled_funding: -3,
            reserved_im: 44,
        };
        let oid = OpenOidV0 {
            client_oid: [5; 16],
            asset_id: 7,
            lots_delta: -2,
            state: cc::OID_PENDING,
        };
        let mut old = UserLedgerV0 {
            user,
            free: 101,
            reserved: 44,
            withdrawable: 8,
            pending_oid_count: 1,
            nonce: 22,
            last_funding_epoch: 6,
            positions_len: 1,
            positions: [Position::default(); 16],
            open_oids: [OpenOidV0::default(); 8],
            bump: 254,
        };
        old.positions[0] = position;
        old.open_oids[0] = oid;

        let migrated = convert_user_ledger(Box::new(old));

        assert_eq!(migrated.schema_version, 1);
        assert_eq!(migrated.user, user);
        assert_eq!(migrated.free, 101);
        assert_eq!(migrated.reserved, 44);
        assert_eq!(migrated.withdrawable, 8);
        assert_eq!(migrated.bad_debt_usdc, 0);
        assert_eq!(migrated.positions[0].lots, -9);
        assert_eq!(migrated.open_oids[0].client_oid, [5; 16]);
        assert_eq!(migrated.open_oids[0].limit_price_ticks, 0);
        assert_eq!(migrated.open_oids[0].last_valid_slot, 0);
    }

    #[test]
    fn book_conversion_preserves_legacy_state() {
        let mut old = BookV0 {
            residual_len: 1,
            residuals: [Residual::default(); 32],
            phoenix_collateral: 55,
            last_ack_slot_er: 66,
            invariant_ok: 1,
            halt: cc::OPERATOR_DOWN,
            funding_epoch: 77,
            last_scan_ms: 88,
            bump: 253,
        };
        old.residuals[0] = Residual {
            asset_id: 4,
            lots: -12,
        };

        let migrated = convert_book(Box::new(old));
        assert_eq!(migrated.schema_version, 1);
        assert_eq!(migrated.residual_len, 1);
        assert_eq!(migrated.residuals[0].asset_id, 4);
        assert_eq!(migrated.residuals[0].lots, -12);
        assert_eq!(migrated.phoenix_collateral, 55);
        assert_eq!(migrated.last_ack_slot_er, 66);
        assert_eq!(migrated.halt, cc::OPERATOR_DOWN);
        assert_eq!(migrated.funding_epoch, 77);
        assert_eq!(migrated.last_scan_ms, 88);
    }

    #[test]
    fn legacy_decoder_accepts_exact_v0_and_rejects_current_size() {
        let old = BookV0 {
            residual_len: 0,
            residuals: [Residual::default(); 32],
            phoenix_collateral: 9,
            last_ack_slot_er: 10,
            invariant_ok: 1,
            halt: 0,
            funding_epoch: 11,
            last_scan_ms: 12,
            bump: 252,
        };
        let key = Pubkey::new_unique();
        let owner = crate::ID;
        let mut lamports = 1;
        let mut data = vec![0u8; BOOK_V0_SPACE];
        data[..8].copy_from_slice(Book::DISCRIMINATOR);
        old.serialize(&mut &mut data[8..]).unwrap();
        let info = AccountInfo::new(&key, false, true, &mut lamports, &mut data, &owner, false);

        let decoded: Box<BookV0> = read_legacy_account(
            &info,
            Book::DISCRIMINATOR,
            BOOK_V0_SPACE,
            Book::ACCOUNT_SPACE,
        )
        .unwrap();
        assert_eq!(decoded.phoenix_collateral, 9);
        drop(info);

        let mut current_data = vec![0u8; Book::ACCOUNT_SPACE];
        let current_info = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut current_data,
            &owner,
            false,
        );
        assert!(read_legacy_account::<BookV0>(
            &current_info,
            Book::DISCRIMINATOR,
            BOOK_V0_SPACE,
            Book::ACCOUNT_SPACE,
        )
        .is_err());
    }
}
