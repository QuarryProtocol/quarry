use crate::*;

/// Executes an instruction handler, given a slice of accounts.
fn execute_ix_handler_raw<'a, 'b, 'c, 'info, T: Accounts<'info> + Validate<'info>>(
    program_id: &'a Pubkey,
    accounts_unchecked: &mut &[AccountInfo<'info>],
    ix_handler: fn(ctx: Context<T>) -> Result<()>,
) -> Result<()> {
    let mut bumps = std::collections::BTreeMap::new();
    let ctx: Context<T> = Context {
        program_id,
        accounts: &mut T::try_accounts(program_id, accounts_unchecked, &[], &mut bumps)?,
        remaining_accounts: &[],
        bumps,
    };
    // re-validate newly parsed accounts struct
    ctx.accounts.validate()?;
    ix_handler(ctx)
}

/// Executes an instruction handler, re-validating the accounts.
pub fn execute_ix_handler<
    'a,
    'b,
    'c,
    'info,
    T: Accounts<'info> + Validate<'info>,
    V: ToAccountInfos<'info>,
>(
    program_id: &'a Pubkey,
    accounts_unchecked: V,
    ix_handler: fn(ctx: Context<T>) -> Result<()>,
) -> Result<()> {
    execute_ix_handler_raw(
        program_id,
        &mut accounts_unchecked.to_account_infos().as_slice(),
        ix_handler,
    )
}
